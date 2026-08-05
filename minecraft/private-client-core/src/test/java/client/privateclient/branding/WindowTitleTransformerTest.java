package client.privateclient.branding;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotSame;
import static org.junit.Assert.assertSame;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import org.junit.Test;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodNode;

public final class WindowTitleTransformerTest {
    @Test
    public void patchesThePinnedMinecraftClassBeforeTheWindowIsCreated() throws Exception {
        byte[] original = readClass("/net/minecraft/client/Minecraft.class");
        byte[] transformed = new WindowTitleTransformer().transform(
                "ave", "net.minecraft.client.Minecraft", original);

        assertNotSame(original, transformed);
        assertEquals(1, countString(transformed, WindowTitleTransformer.PRIVATE_TITLE));
        assertEquals(0, countString(transformed, "Minecraft 1.8.9"));
    }

    @Test
    public void replacesOnlyTheDisplayTitleArgument() {
        byte[] transformed = new WindowTitleTransformer().transform(
                "ave", "net.minecraft.client.Minecraft", fixture(1));
        assertEquals(1, countString(transformed, WindowTitleTransformer.PRIVATE_TITLE));
        assertEquals(0, countString(transformed, "Minecraft 1.8.9"));
        assertEquals(1, countString(transformed, "Minecraft 1.8.9 diagnostics"));
    }

    @Test
    public void refusesAnAmbiguousTarget() {
        byte[] original = fixture(2);
        assertSame(original, new WindowTitleTransformer().transform(
                "ave", "net.minecraft.client.Minecraft", original));
    }

    private static byte[] fixture(int titleCalls) {
        ClassWriter writer = new ClassWriter(0);
        writer.visit(Opcodes.V1_8, Opcodes.ACC_PUBLIC,
                "net/minecraft/client/Minecraft", null, "java/lang/Object", null);
        MethodVisitor method = writer.visitMethod(
                Opcodes.ACC_PRIVATE, "createDisplay", "()V", null, null);
        method.visitCode();
        method.visitLdcInsn("Minecraft 1.8.9 diagnostics");
        method.visitInsn(Opcodes.POP);
        for (int index = 0; index < titleCalls; index++) {
            method.visitLdcInsn("Minecraft 1.8.9");
            method.visitMethodInsn(Opcodes.INVOKESTATIC, "org/lwjgl/opengl/Display",
                    "setTitle", "(Ljava/lang/String;)V", false);
        }
        method.visitInsn(Opcodes.RETURN);
        method.visitMaxs(1, 1);
        method.visitEnd();
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static int countString(byte[] bytes, String value) {
        ClassNode node = new ClassNode();
        new ClassReader(bytes).accept(node, 0);
        int count = 0;
        for (MethodNode method : node.methods) {
            for (AbstractInsnNode instruction : method.instructions.toArray()) {
                if (instruction instanceof LdcInsnNode
                        && value.equals(((LdcInsnNode) instruction).cst)) {
                    count++;
                }
            }
        }
        return count;
    }

    private static byte[] readClass(String resource) throws Exception {
        InputStream input = WindowTitleTransformerTest.class.getResourceAsStream(resource);
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
