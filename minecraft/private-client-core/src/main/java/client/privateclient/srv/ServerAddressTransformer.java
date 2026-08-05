package client.privateclient.srv;

import net.minecraft.launchwrapper.IClassTransformer;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.InsnList;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.TypeInsnNode;
import org.objectweb.asm.tree.VarInsnNode;

public final class ServerAddressTransformer implements IClassTransformer {
    private static final String TARGET = "net.minecraft.client.multiplayer.ServerAddress";
    private static final String METHOD_DESCRIPTOR = "(Ljava/lang/String;)Lnet/minecraft/client/multiplayer/ServerAddress;";
    private static final String RESULT = Type.getInternalName(ResolvedServerAddress.class);
    private static final String RESOLVER = Type.getInternalName(SrvResolver.class);

    @Override
    public byte[] transform(String name, String transformedName, byte[] basicClass) {
        if (!TARGET.equals(transformedName) || basicClass == null) {
            return basicClass;
        }

        ClassNode node = new ClassNode();
        new ClassReader(basicClass).accept(node, 0);
        boolean changed = false;
        for (MethodNode method : node.methods) {
            if (("fromString".equals(method.name) || "func_78860_a".equals(method.name)
                    || "a".equals(method.name))
                    && METHOD_DESCRIPTOR.equals(method.desc)) {
                replaceFactory(node.name, method);
                changed = true;
            }
        }
        // Other coremods can change this method before us. Multiplayer must keep
        // working with Minecraft's parser even when the optional SRV patch cannot
        // be installed, so never make the target class unloadable here.
        if (!changed) {
            return basicClass;
        }
        ClassWriter writer = new ClassWriter(ClassWriter.COMPUTE_MAXS);
        node.accept(writer);
        return writer.toByteArray();
    }

    private static void replaceFactory(String owner, MethodNode method) {
        InsnList instructions = new InsnList();
        instructions.add(new VarInsnNode(Opcodes.ALOAD, 0));
        instructions.add(new MethodInsnNode(Opcodes.INVOKESTATIC, RESOLVER, "resolve",
                "(Ljava/lang/String;)L" + RESULT + ";", false));
        instructions.add(new VarInsnNode(Opcodes.ASTORE, 1));
        instructions.add(new TypeInsnNode(Opcodes.NEW, owner));
        instructions.add(new InsnNode(Opcodes.DUP));
        instructions.add(new VarInsnNode(Opcodes.ALOAD, 1));
        instructions.add(new MethodInsnNode(Opcodes.INVOKEVIRTUAL, RESULT, "getHost", "()Ljava/lang/String;", false));
        instructions.add(new VarInsnNode(Opcodes.ALOAD, 1));
        instructions.add(new MethodInsnNode(Opcodes.INVOKEVIRTUAL, RESULT, "getPort", "()I", false));
        instructions.add(new MethodInsnNode(Opcodes.INVOKESPECIAL, owner, "<init>", "(Ljava/lang/String;I)V", false));
        instructions.add(new InsnNode(Opcodes.ARETURN));

        method.instructions.clear();
        method.tryCatchBlocks.clear();
        if (method.localVariables != null) {
            method.localVariables.clear();
        }
        method.instructions.add(instructions);
        method.maxLocals = 2;
        method.maxStack = 4;
    }
}
