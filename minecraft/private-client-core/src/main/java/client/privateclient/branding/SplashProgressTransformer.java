package client.privateclient.branding;

import java.util.ArrayList;
import java.util.List;
import net.minecraft.launchwrapper.IClassTransformer;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.InsnList;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.IntInsnNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.VarInsnNode;

/**
 * Keeps the monochrome Forge splash untinted, stretches its background to the viewport, and
 * replaces the three stacked Forge progress bars with a single Private Client bar.
 */
public final class SplashProgressTransformer implements IClassTransformer {
    private static final String TARGET = "net.minecraftforge.fml.client.SplashProgress$3";
    private static final String BAR = "net/minecraftforge/fml/common/ProgressManager$ProgressBar";
    private static final String BAR_RENDERER = "client/privateclient/branding/SplashBarRenderer";
    private static final String DRAW_BAR_DESC = "(L" + BAR + ";)V";
    private static final String SPLASH = "net/minecraftforge/fml/client/SplashProgress";
    private static final String RENDERER = "net/minecraftforge/fml/client/SplashProgress$3";
    private static final String TEXTURE = "net/minecraftforge/fml/client/SplashProgress$Texture";
    private static final String DISPLAY = "org/lwjgl/opengl/Display";
    private static final String GL11 = "org/lwjgl/opengl/GL11";
    private static final int WHITE = 0xFFFFFF;
    private static final float[][] STOCK_VERTICES = {
        {64.0F, -16.0F},
        {64.0F, 496.0F},
        {576.0F, 496.0F},
        {576.0F, -16.0F}
    };
    private static final int[] X_OPERATIONS = {
        Opcodes.ISUB, Opcodes.ISUB, Opcodes.IADD, Opcodes.IADD
    };
    private static final int[] Y_OPERATIONS = {
        Opcodes.ISUB, Opcodes.IADD, Opcodes.IADD, Opcodes.ISUB
    };

    @Override
    public byte[] transform(String name, String transformedName, byte[] basicClass) {
        if (!TARGET.equals(transformedName) || basicClass == null) {
            return basicClass;
        }

        ClassNode node = new ClassNode();
        new ClassReader(basicClass).accept(node, 0);
        MethodNode run = findRunMethod(node);
        if (run == null) {
            return basicClass;
        }

        // The background and the bar are independent patches: a Forge build that moves one of
        // them must not silently drop the other.
        boolean patched = patchBackground(run);
        patched |= patchSingleBar(node, run);
        if (!patched) {
            return basicClass;
        }

        ClassWriter writer = new ClassWriter(ClassWriter.COMPUTE_MAXS);
        node.accept(writer);
        return writer.toByteArray();
    }

    private static boolean patchBackground(MethodNode run) {
        ViewportLocals viewport = findViewportLocals(run);
        List<VertexPatch> vertices = findBackgroundVertices(run);
        List<MethodInsnNode> colorSources = findTextureColorSources(run);
        if (viewport == null || vertices == null || colorSources.size() != 2) {
            return false;
        }

        for (MethodInsnNode source : colorSources) {
            run.instructions.set(source, new LdcInsnNode(WHITE));
        }
        for (int index = 0; index < vertices.size(); index++) {
            VertexPatch vertex = vertices.get(index);
            run.instructions.remove(vertex.x);
            run.instructions.remove(vertex.y);
            run.instructions.insertBefore(vertex.call, viewportCoordinates(
                    viewport, X_OPERATIONS[index], Y_OPERATIONS[index]));
        }
        return true;
    }

    /**
     * Forge renders up to three stacked bars, each with the raw mod title and a step counter.
     * Private Client shows a single bar instead: the two nested bars are skipped by making their
     * null checks fail, and the remaining draw call is redirected to our renderer.
     */
    private static boolean patchSingleBar(ClassNode node, MethodNode run) {
        MethodNode drawBar = findDrawBarMethod(node);
        List<VarInsnNode> nestedBars = findNestedBarLoads(run);
        if (drawBar == null || nestedBars == null) {
            return false;
        }

        for (VarInsnNode load : nestedBars) {
            run.instructions.set(load, new InsnNode(Opcodes.ACONST_NULL));
        }

        drawBar.instructions.clear();
        if (drawBar.tryCatchBlocks != null) {
            drawBar.tryCatchBlocks.clear();
        }
        if (drawBar.localVariables != null) {
            drawBar.localVariables.clear();
        }
        drawBar.instructions.add(new VarInsnNode(Opcodes.ALOAD, 1));
        drawBar.instructions.add(new MethodInsnNode(
                Opcodes.INVOKESTATIC, BAR_RENDERER, "drawBar", DRAW_BAR_DESC, false));
        drawBar.instructions.add(new InsnNode(Opcodes.RETURN));
        return true;
    }

    private static MethodNode findDrawBarMethod(ClassNode node) {
        MethodNode drawBar = null;
        for (MethodNode method : node.methods) {
            if ("drawBar".equals(method.name) && DRAW_BAR_DESC.equals(method.desc)) {
                if (drawBar != null) {
                    return null;
                }
                drawBar = method;
            }
        }
        return drawBar;
    }

    /**
     * @return the loads of the second and third bar locals that guard their draw calls, or
     *     {@code null} when the expected pair is not found.
     */
    private static List<VarInsnNode> findNestedBarLoads(MethodNode run) {
        List<VarInsnNode> loads = new ArrayList<VarInsnNode>();
        for (AbstractInsnNode instruction = run.instructions.getFirst();
                instruction != null; instruction = instruction.getNext()) {
            if (!(instruction instanceof VarInsnNode)
                    || instruction.getOpcode() != Opcodes.ALOAD) {
                continue;
            }
            VarInsnNode load = (VarInsnNode) instruction;
            if (load.var != 2 && load.var != 3) {
                continue;
            }
            AbstractInsnNode next = nextMeaningful(load);
            if (next != null && next.getOpcode() == Opcodes.IFNULL) {
                loads.add(load);
            }
        }
        if (loads.size() != 2 || loads.get(0).var == loads.get(1).var) {
            return null;
        }
        return loads;
    }

    private static MethodNode findRunMethod(ClassNode node) {
        MethodNode run = null;
        for (MethodNode method : node.methods) {
            if ("run".equals(method.name) && "()V".equals(method.desc)) {
                if (run != null) {
                    return null;
                }
                run = method;
            }
        }
        return run;
    }

    private static ViewportLocals findViewportLocals(MethodNode run) {
        int width = -1;
        int height = -1;
        int widthMatches = 0;
        int heightMatches = 0;
        for (AbstractInsnNode instruction = run.instructions.getFirst();
                instruction != null; instruction = instruction.getNext()) {
            if (!(instruction instanceof MethodInsnNode)) {
                continue;
            }
            MethodInsnNode call = (MethodInsnNode) instruction;
            if (call.getOpcode() != Opcodes.INVOKESTATIC
                    || !DISPLAY.equals(call.owner)
                    || !"()I".equals(call.desc)) {
                continue;
            }
            boolean isWidth = "getWidth".equals(call.name);
            boolean isHeight = "getHeight".equals(call.name);
            if (!isWidth && !isHeight) {
                continue;
            }
            AbstractInsnNode next = nextMeaningful(call);
            if (!(next instanceof VarInsnNode) || next.getOpcode() != Opcodes.ISTORE) {
                return null;
            }
            int variable = ((VarInsnNode) next).var;
            if (isWidth) {
                width = variable;
                widthMatches++;
            } else {
                height = variable;
                heightMatches++;
            }
        }
        return widthMatches == 1 && heightMatches == 1
                ? new ViewportLocals(width, height) : null;
    }

    private static List<MethodInsnNode> findTextureColorSources(MethodNode run) {
        List<MethodInsnNode> colorSources = new ArrayList<MethodInsnNode>();
        for (AbstractInsnNode instruction = run.instructions.getFirst();
                instruction != null; instruction = instruction.getNext()) {
            if (!(instruction instanceof MethodInsnNode)) {
                continue;
            }
            MethodInsnNode source = (MethodInsnNode) instruction;
            if (source.getOpcode() != Opcodes.INVOKESTATIC
                    || !SPLASH.equals(source.owner)
                    || !"access$500".equals(source.name)
                    || !"()I".equals(source.desc)) {
                continue;
            }
            AbstractInsnNode next = nextMeaningful(source);
            if (next instanceof MethodInsnNode) {
                MethodInsnNode setColor = (MethodInsnNode) next;
                if (setColor.getOpcode() == Opcodes.INVOKESPECIAL
                        && RENDERER.equals(setColor.owner)
                        && "setColor".equals(setColor.name)
                        && "(I)V".equals(setColor.desc)) {
                    colorSources.add(source);
                }
            }
        }
        return colorSources;
    }

    private static List<VertexPatch> findBackgroundVertices(MethodNode run) {
        MethodInsnNode bind = null;
        int bindMatches = 0;
        for (AbstractInsnNode instruction = run.instructions.getFirst();
                instruction != null; instruction = instruction.getNext()) {
            if (!(instruction instanceof MethodInsnNode)) {
                continue;
            }
            MethodInsnNode source = (MethodInsnNode) instruction;
            if (!isCall(source, Opcodes.INVOKESTATIC, SPLASH,
                    "access$100", "()L" + TEXTURE + ";")) {
                continue;
            }
            AbstractInsnNode next = nextMeaningful(source);
            if (next instanceof MethodInsnNode
                    && isCall((MethodInsnNode) next, Opcodes.INVOKEVIRTUAL,
                            TEXTURE, "bind", "()V")) {
                bind = (MethodInsnNode) next;
                bindMatches++;
            }
        }
        if (bindMatches != 1 || bind == null) {
            return null;
        }

        AbstractInsnNode beginArgument = nextMeaningful(bind);
        AbstractInsnNode beginInstruction = nextMeaningful(beginArgument);
        if (!(beginArgument instanceof IntInsnNode)
                || beginArgument.getOpcode() != Opcodes.BIPUSH
                || ((IntInsnNode) beginArgument).operand != 7
                || !(beginInstruction instanceof MethodInsnNode)
                || !isCall((MethodInsnNode) beginInstruction, Opcodes.INVOKESTATIC,
                        GL11, "glBegin", "(I)V")) {
            return null;
        }

        List<MethodInsnNode> textureCoordinates = new ArrayList<MethodInsnNode>();
        List<MethodInsnNode> vertexCalls = new ArrayList<MethodInsnNode>();
        boolean foundEnd = false;
        for (AbstractInsnNode instruction = beginInstruction.getNext();
                instruction != null; instruction = instruction.getNext()) {
            if (!(instruction instanceof MethodInsnNode)) {
                continue;
            }
            MethodInsnNode call = (MethodInsnNode) instruction;
            if (isCall(call, Opcodes.INVOKESTATIC, GL11, "glEnd", "()V")) {
                foundEnd = true;
                break;
            }
            if (isCall(call, Opcodes.INVOKEVIRTUAL, TEXTURE,
                    "texCoord", "(IFF)V")) {
                textureCoordinates.add(call);
            } else if (isCall(call, Opcodes.INVOKESTATIC, GL11,
                    "glVertex2f", "(FF)V")) {
                vertexCalls.add(call);
            } else if (isCall(call, Opcodes.INVOKESTATIC, GL11,
                    "glBegin", "(I)V")) {
                return null;
            }
        }
        if (!foundEnd || textureCoordinates.size() != 4 || vertexCalls.size() != 4) {
            return null;
        }

        List<VertexPatch> patches = new ArrayList<VertexPatch>();
        for (int index = 0; index < vertexCalls.size(); index++) {
            MethodInsnNode call = vertexCalls.get(index);
            AbstractInsnNode y = previousMeaningful(call);
            AbstractInsnNode x = previousMeaningful(y);
            AbstractInsnNode textureCoordinate = previousMeaningful(x);
            if (!(x instanceof LdcInsnNode) || !(y instanceof LdcInsnNode)
                    || !(textureCoordinate instanceof MethodInsnNode)
                    || textureCoordinate != textureCoordinates.get(index)
                    || !Float.valueOf(STOCK_VERTICES[index][0]).equals(
                            ((LdcInsnNode) x).cst)
                    || !Float.valueOf(STOCK_VERTICES[index][1]).equals(
                            ((LdcInsnNode) y).cst)) {
                return null;
            }
            patches.add(new VertexPatch(
                    (LdcInsnNode) x, (LdcInsnNode) y, call));
        }
        return patches;
    }

    private static InsnList viewportCoordinates(
            ViewportLocals viewport, int xOperation, int yOperation) {
        InsnList instructions = new InsnList();
        instructions.add(new IntInsnNode(Opcodes.SIPUSH, 320));
        instructions.add(new VarInsnNode(Opcodes.ILOAD, viewport.width));
        instructions.add(new InsnNode(Opcodes.ICONST_2));
        instructions.add(new InsnNode(Opcodes.IDIV));
        instructions.add(new InsnNode(xOperation));
        instructions.add(new InsnNode(Opcodes.I2F));
        instructions.add(new IntInsnNode(Opcodes.SIPUSH, 240));
        instructions.add(new VarInsnNode(Opcodes.ILOAD, viewport.height));
        instructions.add(new InsnNode(Opcodes.ICONST_2));
        instructions.add(new InsnNode(Opcodes.IDIV));
        instructions.add(new InsnNode(yOperation));
        instructions.add(new InsnNode(Opcodes.I2F));
        return instructions;
    }

    private static boolean isCall(
            MethodInsnNode call, int opcode, String owner, String name, String desc) {
        return call.getOpcode() == opcode
                && owner.equals(call.owner)
                && name.equals(call.name)
                && desc.equals(call.desc);
    }

    private static AbstractInsnNode previousMeaningful(AbstractInsnNode instruction) {
        AbstractInsnNode previous = instruction.getPrevious();
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

    private static final class ViewportLocals {
        private final int width;
        private final int height;

        private ViewportLocals(int width, int height) {
            this.width = width;
            this.height = height;
        }
    }

    private static final class VertexPatch {
        private final LdcInsnNode x;
        private final LdcInsnNode y;
        private final MethodInsnNode call;

        private VertexPatch(LdcInsnNode x, LdcInsnNode y, MethodInsnNode call) {
            this.x = x;
            this.y = y;
            this.call = call;
        }
    }
}
