package client.privateclient.srv;

import org.junit.Test;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.MethodNode;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

public final class ServerAddressTransformerTest {
    private static final String TARGET = "net.minecraft.client.multiplayer.ServerAddress";
    private static final String DESCRIPTOR = "(Ljava/lang/String;)Lnet/minecraft/client/multiplayer/ServerAddress;";

    @Test
    public void transformsDeobfuscatedSrgAndNotchMethodNames() {
        for (String methodName : new String[] {"fromString", "func_78860_a", "a"}) {
            byte[] transformed = new ServerAddressTransformer().transform(TARGET, TARGET, fixture(methodName));
            ClassNode node = read(transformed);
            MethodNode method = find(node, methodName);
            assertEquals(Opcodes.ALOAD, method.instructions.getFirst().getOpcode());
            assertTrue(method.instructions.size() > 5);
        }
    }

    @Test
    public void leavesClassUsableWhenAnotherCoremodChangedTheFactory() {
        byte[] original = fixture("changedByAnotherCoremod");
        byte[] transformed = new ServerAddressTransformer().transform(TARGET, TARGET, original);
        assertArrayEquals(original, transformed);
    }

    private static byte[] fixture(String methodName) {
        ClassWriter writer = new ClassWriter(0);
        writer.visit(Opcodes.V1_8, Opcodes.ACC_PUBLIC, "net/minecraft/client/multiplayer/ServerAddress",
                null, "java/lang/Object", null);
        org.objectweb.asm.MethodVisitor method = writer.visitMethod(
                Opcodes.ACC_PUBLIC | Opcodes.ACC_STATIC, methodName, DESCRIPTOR, null, null);
        method.visitCode();
        method.visitInsn(Opcodes.ACONST_NULL);
        method.visitInsn(Opcodes.ARETURN);
        method.visitMaxs(1, 1);
        method.visitEnd();
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static ClassNode read(byte[] bytes) {
        ClassNode node = new ClassNode();
        new ClassReader(bytes).accept(node, 0);
        return node;
    }

    private static MethodNode find(ClassNode node, String name) {
        for (MethodNode method : node.methods) {
            if (name.equals(method.name)) {
                return method;
            }
        }
        throw new AssertionError("Missing method " + name);
    }
}
