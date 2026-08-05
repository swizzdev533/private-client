package client.privateclient.branding;

import net.minecraft.launchwrapper.IClassTransformer;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;

/** Rebrands the LWJGL window before Forge's splash screen becomes visible. */
public final class WindowTitleTransformer implements IClassTransformer {
    public static final String PRIVATE_TITLE = "Private Client 1.8.9";
    private static final String TARGET = "net.minecraft.client.Minecraft";
    private static final String STOCK_TITLE = "Minecraft 1.8.9";

    @Override
    public byte[] transform(String name, String transformedName, byte[] basicClass) {
        if (!TARGET.equals(transformedName) || basicClass == null) {
            return basicClass;
        }

        ClassNode node = new ClassNode();
        new ClassReader(basicClass).accept(node, 0);
        int replacements = 0;
        LdcInsnNode candidate = null;
        for (MethodNode method : node.methods) {
            if (!"()V".equals(method.desc)) {
                continue;
            }
            for (AbstractInsnNode instruction = method.instructions.getFirst();
                    instruction != null; instruction = instruction.getNext()) {
                if (instruction instanceof LdcInsnNode
                        && STOCK_TITLE.equals(((LdcInsnNode) instruction).cst)) {
                    AbstractInsnNode next = nextMeaningful(instruction);
                    if (next instanceof MethodInsnNode) {
                        MethodInsnNode call = (MethodInsnNode) next;
                        if (call.getOpcode() == Opcodes.INVOKESTATIC
                                && "org/lwjgl/opengl/Display".equals(call.owner)
                                && "setTitle".equals(call.name)
                                && "(Ljava/lang/String;)V".equals(call.desc)) {
                            candidate = (LdcInsnNode) instruction;
                            replacements++;
                        }
                    }
                }
            }
        }
        if (replacements != 1 || candidate == null) {
            return basicClass;
        }
        candidate.cst = PRIVATE_TITLE;
        ClassWriter writer = new ClassWriter(ClassWriter.COMPUTE_MAXS);
        node.accept(writer);
        return writer.toByteArray();
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
}
