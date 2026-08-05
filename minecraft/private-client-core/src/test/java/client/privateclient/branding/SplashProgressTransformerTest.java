package client.privateclient.branding;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotSame;
import static org.junit.Assert.assertSame;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import org.junit.Test;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.IntInsnNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.VarInsnNode;

public final class SplashProgressTransformerTest {
    private static final String SPLASH = "net/minecraftforge/fml/client/SplashProgress";
    private static final String RENDERER = "net/minecraftforge/fml/client/SplashProgress$3";
    private static final String TEXTURE = "net/minecraftforge/fml/client/SplashProgress$Texture";
    private static final String GL11 = "org/lwjgl/opengl/GL11";
    private static final String BAR = "net/minecraftforge/fml/common/ProgressManager$ProgressBar";
    private static final String BAR_RENDERER = "client/privateclient/branding/SplashBarRenderer";
    private static final String DRAW_BAR_DESC = "(L" + BAR + ";)V";

    @Test
    public void patchesTheRealForgeTintAndBackgroundGeometry() throws Exception {
        byte[] original = readClass("/net/minecraftforge/fml/client/SplashProgress$3.class");
        assertEquals(4, countStockBackgroundVertices(original));

        byte[] transformed = new SplashProgressTransformer().transform(
                "net.minecraftforge.fml.client.SplashProgress$3",
                "net.minecraftforge.fml.client.SplashProgress$3",
                original);

        assertNotSame(original, transformed);
        assertEquals(2, countWhiteSetColorCalls(transformed));
        assertEquals(0, countStockBackgroundVertices(transformed));
        assertEquals(1, countViewportVertices(transformed, Opcodes.ISUB, Opcodes.ISUB));
        assertEquals(1, countViewportVertices(transformed, Opcodes.ISUB, Opcodes.IADD));
        assertEquals(1, countViewportVertices(transformed, Opcodes.IADD, Opcodes.IADD));
        assertEquals(1, countViewportVertices(transformed, Opcodes.IADD, Opcodes.ISUB));
        assertEquals(3, countCalls(transformed, SPLASH, "access$500", "()I"));
        assertEquals(
                countCalls(original, SPLASH, "access$200", "()L" + TEXTURE + ";"),
                countCalls(transformed, SPLASH, "access$200", "()L" + TEXTURE + ";"));
    }

    @Test
    public void drawsASingleBarThroughThePrivateClientRenderer() throws Exception {
        byte[] original = readClass("/net/minecraftforge/fml/client/SplashProgress$3.class");
        assertEquals(2, countNestedBarGuards(original));
        assertEquals(3, countCalls(original, RENDERER, "drawBar", DRAW_BAR_DESC));

        byte[] transformed = new SplashProgressTransformer().transform(
                "net.minecraftforge.fml.client.SplashProgress$3",
                "net.minecraftforge.fml.client.SplashProgress$3",
                original);

        // The three stacked bar draws survive, but two of them are now unreachable and the
        // remaining one renders through us instead of Forge's title/step drawing.
        assertEquals(0, countNestedBarGuards(transformed));
        assertEquals(1, countCalls(transformed, BAR_RENDERER, "drawBar", DRAW_BAR_DESC));
        assertEquals(0, countCalls(transformed, SPLASH, "access$1000", "()I"));
        assertEquals(0, countCalls(transformed, RENDERER, "drawBox", "(II)V"));
    }

    @Test
    public void refusesTheBackgroundPatchWhenOneVertexDoesNotMatch() throws Exception {
        byte[] original = readClass("/net/minecraftforge/fml/client/SplashProgress$3.class");
        byte[] altered = alterFirstStockVertex(original);

        byte[] result = new SplashProgressTransformer().transform(
                "net.minecraftforge.fml.client.SplashProgress$3",
                "net.minecraftforge.fml.client.SplashProgress$3",
                altered);

        assertNotSame(altered, result);
        assertEquals(5, countCalls(result, SPLASH, "access$500", "()I"));
        // The independent bar patch still applies.
        assertEquals(1, countCalls(result, BAR_RENDERER, "drawBar", DRAW_BAR_DESC));
    }

    @Test
    public void keepsTheClassUntouchedWhenNothingMatches() throws Exception {
        byte[] unrelated = readClass("/net/minecraftforge/fml/client/SplashProgress$Texture.class");

        assertSame(unrelated, new SplashProgressTransformer().transform(
                "net.minecraftforge.fml.client.SplashProgress$Texture",
                "net.minecraftforge.fml.client.SplashProgress$Texture",
                unrelated));
    }

    private static int countNestedBarGuards(byte[] bytes) {
        ClassNode node = readNode(bytes);
        int count = 0;
        for (MethodNode method : node.methods) {
            if (!"run".equals(method.name) || !"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (!isVariable(instruction, Opcodes.ALOAD, 2)
                        && !isVariable(instruction, Opcodes.ALOAD, 3)) {
                    continue;
                }
                AbstractInsnNode next = nextMeaningful(instruction);
                if (next != null && next.getOpcode() == Opcodes.IFNULL) {
                    count++;
                }
            }
        }
        return count;
    }

    private static int countWhiteSetColorCalls(byte[] bytes) {
        ClassNode node = readNode(bytes);
        int count = 0;
        for (MethodNode method : node.methods) {
            if (!"run".equals(method.name) || !"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (!(instruction instanceof LdcInsnNode)
                        || !Integer.valueOf(0xFFFFFF).equals(((LdcInsnNode) instruction).cst)) {
                    continue;
                }
                AbstractInsnNode next = nextMeaningful(instruction);
                if (next instanceof MethodInsnNode
                        && isCall((MethodInsnNode) next, Opcodes.INVOKESPECIAL,
                                RENDERER, "setColor", "(I)V")) {
                    count++;
                }
            }
        }
        return count;
    }

    private static int countStockBackgroundVertices(byte[] bytes) {
        ClassNode node = readNode(bytes);
        float[][] expected = {
            {64.0F, -16.0F},
            {64.0F, 496.0F},
            {576.0F, 496.0F},
            {576.0F, -16.0F}
        };
        int count = 0;
        for (MethodNode method : node.methods) {
            if (!"run".equals(method.name) || !"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (!(instruction instanceof MethodInsnNode)
                        || !isCall((MethodInsnNode) instruction, Opcodes.INVOKESTATIC,
                                GL11, "glVertex2f", "(FF)V")) {
                    continue;
                }
                AbstractInsnNode y = previousMeaningful(instruction);
                AbstractInsnNode x = previousMeaningful(y);
                AbstractInsnNode texCoord = previousMeaningful(x);
                if (!(x instanceof LdcInsnNode) || !(y instanceof LdcInsnNode)
                        || !(texCoord instanceof MethodInsnNode)
                        || !isCall((MethodInsnNode) texCoord, Opcodes.INVOKEVIRTUAL,
                                TEXTURE, "texCoord", "(IFF)V")) {
                    continue;
                }
                for (float[] vertex : expected) {
                    if (Float.valueOf(vertex[0]).equals(((LdcInsnNode) x).cst)
                            && Float.valueOf(vertex[1]).equals(((LdcInsnNode) y).cst)) {
                        count++;
                        break;
                    }
                }
            }
        }
        return count;
    }

    private static int countViewportVertices(byte[] bytes, int xOperation, int yOperation) {
        ClassNode node = readNode(bytes);
        int count = 0;
        for (MethodNode method : node.methods) {
            if (!"run".equals(method.name) || !"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (instruction instanceof MethodInsnNode
                        && isCall((MethodInsnNode) instruction, Opcodes.INVOKESTATIC,
                                GL11, "glVertex2f", "(FF)V")
                        && isViewportVertex(instruction, xOperation, yOperation)) {
                    count++;
                }
            }
        }
        return count;
    }

    private static boolean isViewportVertex(
            AbstractInsnNode vertex, int xOperation, int yOperation) {
        AbstractInsnNode[] sequence = new AbstractInsnNode[12];
        AbstractInsnNode cursor = vertex;
        for (int index = sequence.length - 1; index >= 0; index--) {
            cursor = previousMeaningful(cursor);
            if (cursor == null) {
                return false;
            }
            sequence[index] = cursor;
        }
        AbstractInsnNode before = previousMeaningful(sequence[0]);
        return before instanceof MethodInsnNode
                && isCall((MethodInsnNode) before, Opcodes.INVOKEVIRTUAL,
                        TEXTURE, "texCoord", "(IFF)V")
                && isIntInstruction(sequence[0], Opcodes.SIPUSH, 320)
                && isVariable(sequence[1], Opcodes.ILOAD, 5)
                && sequence[2].getOpcode() == Opcodes.ICONST_2
                && sequence[3].getOpcode() == Opcodes.IDIV
                && sequence[4].getOpcode() == xOperation
                && sequence[5].getOpcode() == Opcodes.I2F
                && isIntInstruction(sequence[6], Opcodes.SIPUSH, 240)
                && isVariable(sequence[7], Opcodes.ILOAD, 6)
                && sequence[8].getOpcode() == Opcodes.ICONST_2
                && sequence[9].getOpcode() == Opcodes.IDIV
                && sequence[10].getOpcode() == yOperation
                && sequence[11].getOpcode() == Opcodes.I2F;
    }

    private static int countCalls(byte[] bytes, String owner, String name, String desc) {
        ClassNode node = readNode(bytes);
        int count = 0;
        for (MethodNode method : node.methods) {
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (instruction instanceof MethodInsnNode) {
                    MethodInsnNode call = (MethodInsnNode) instruction;
                    if (owner.equals(call.owner) && name.equals(call.name) && desc.equals(call.desc)) {
                        count++;
                    }
                }
            }
        }
        return count;
    }

    private static byte[] alterFirstStockVertex(byte[] bytes) {
        ClassNode node = readNode(bytes);
        for (MethodNode method : node.methods) {
            if (!"run".equals(method.name) || !"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (!(instruction instanceof MethodInsnNode)
                        || !isCall((MethodInsnNode) instruction, Opcodes.INVOKESTATIC,
                                GL11, "glVertex2f", "(FF)V")) {
                    continue;
                }
                AbstractInsnNode y = previousMeaningful(instruction);
                AbstractInsnNode x = previousMeaningful(y);
                if (x instanceof LdcInsnNode && y instanceof LdcInsnNode
                        && Float.valueOf(64.0F).equals(((LdcInsnNode) x).cst)
                        && Float.valueOf(-16.0F).equals(((LdcInsnNode) y).cst)) {
                    ((LdcInsnNode) x).cst = Float.valueOf(65.0F);
                    ClassWriter writer = new ClassWriter(0);
                    node.accept(writer);
                    return writer.toByteArray();
                }
            }
        }
        throw new IllegalStateException("Pinned Forge background vertex was not found");
    }

    private static boolean isCall(
            MethodInsnNode call, int opcode, String owner, String name, String desc) {
        return call.getOpcode() == opcode
                && owner.equals(call.owner)
                && name.equals(call.name)
                && desc.equals(call.desc);
    }

    private static boolean isIntInstruction(
            AbstractInsnNode instruction, int opcode, int operand) {
        return instruction instanceof IntInsnNode
                && instruction.getOpcode() == opcode
                && ((IntInsnNode) instruction).operand == operand;
    }

    private static boolean isVariable(
            AbstractInsnNode instruction, int opcode, int variable) {
        return instruction instanceof VarInsnNode
                && instruction.getOpcode() == opcode
                && ((VarInsnNode) instruction).var == variable;
    }

    private static AbstractInsnNode previousMeaningful(AbstractInsnNode instruction) {
        AbstractInsnNode previous = instruction == null ? null : instruction.getPrevious();
        while (previous != null && (previous.getType() == AbstractInsnNode.LABEL
                || previous.getType() == AbstractInsnNode.LINE
                || previous.getType() == AbstractInsnNode.FRAME)) {
            previous = previous.getPrevious();
        }
        return previous;
    }

    private static AbstractInsnNode nextMeaningful(AbstractInsnNode instruction) {
        AbstractInsnNode next = instruction.getNext();
        while (next != null && (next.getType() == AbstractInsnNode.LABEL
                || next.getType() == AbstractInsnNode.LINE
                || next.getType() == AbstractInsnNode.FRAME)) {
            next = next.getNext();
        }
        return next;
    }

    private static ClassNode readNode(byte[] bytes) {
        ClassNode node = new ClassNode();
        new ClassReader(bytes).accept(node, 0);
        return node;
    }

    private static byte[] readClass(String resource) throws Exception {
        InputStream input = SplashProgressTransformerTest.class.getResourceAsStream(resource);
        if (input == null) {
            throw new IllegalStateException("Missing test fixture " + resource);
        }
        try {
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) >= 0) {
                output.write(buffer, 0, count);
            }
            return output.toByteArray();
        } finally {
            input.close();
        }
    }
}
