package client.privateclient.badge;

import client.privateclient.association.AssociationService;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.network.NetworkPlayerInfo;
import net.minecraft.scoreboard.ScorePlayerTeam;
import net.minecraft.util.IChatComponent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;

/**
 * Draws PrivetBadge icons on the player list using the same column math as
 * Minecraft 1.8.9 {@code GuiPlayerTabOverlay}.
 */
public final class TabListBadgeOverlay {
    private final AssociationService associationService;

    public TabListBadgeOverlay(AssociationService associationService) {
        this.associationService = associationService;
    }

    public void render(RenderGameOverlayEvent.Post event) {
        if (event.type != RenderGameOverlayEvent.ElementType.PLAYER_LIST) {
            return;
        }
        Minecraft mc = Minecraft.getMinecraft();
        if (mc == null || mc.thePlayer == null || mc.getNetHandler() == null
                || mc.fontRendererObj == null) {
            return;
        }

        Collection<NetworkPlayerInfo> collection = mc.getNetHandler().getPlayerInfoMap();
        List<NetworkPlayerInfo> players = new ArrayList<NetworkPlayerInfo>(collection);
        Collections.sort(players, new Comparator<NetworkPlayerInfo>() {
            @Override
            public int compare(NetworkPlayerInfo left, NetworkPlayerInfo right) {
                return left.getGameProfile().getId().compareTo(right.getGameProfile().getId());
            }
        });
        if (players.isEmpty()) {
            return;
        }

        int maxPlayers = mc.thePlayer.sendQueue.currentServerMaxPlayers;
        if (maxPlayers <= 0) {
            maxPlayers = players.size();
        }
        maxPlayers = Math.min(maxPlayers, players.size());
        maxPlayers = Math.min(maxPlayers, 80);

        int columns = 1;
        int rows = maxPlayers;
        while (rows > 20) {
            columns++;
            rows = (maxPlayers + columns - 1) / columns;
        }

        int entryWidth = 0;
        for (NetworkPlayerInfo info : players) {
            String name = getPlayerName(info);
            int width = mc.fontRendererObj.getStringWidth(name)
                    + PrivetBadgeRenderer.tabBadgeAdvance()
                    + 9
                    + 5;
            if (width > entryWidth) {
                entryWidth = width;
            }
        }
        entryWidth = Math.min(entryWidth, 100);

        ScaledResolution scaled = new ScaledResolution(mc);
        int listWidth = Math.min(columns * ((entryWidth + 9 + 1) ) + (columns - 1) * 5, scaled.getScaledWidth() - 50);
        int startX = (scaled.getScaledWidth() - listWidth) / 2;
        int startY = 10;
        int slotWidth = listWidth / columns;

        for (int index = 0; index < maxPlayers && index < players.size(); index++) {
            int column = index / rows;
            int row = index % rows;
            int x = startX + column * slotWidth;
            int y = startY + row * 9 + 9;

            NetworkPlayerInfo info = players.get(index);
            UUID uuid = info.getGameProfile().getId();
            if (!associationService.shouldShowBadge(uuid, info.getGameProfile().getName())) {
                continue;
            }

            // Vanilla draws head at x, name at x + 9 + 1. Place badge just before name text.
            int badgeX = x + 9 + 1;
            int badgeY = y;
            PrivetBadgeRenderer.drawScreenBadge(badgeX, badgeY);

            // Dim a 1px guide only if needed — avoid covering text by shifting painted name.
            // Vanilla already painted the name; draw a small opaque strip then badge sits in the
            // left padding before the first glyph when entryWidth reserved badgeAdvance.
            // Re-draw name shifted right so it is not under the badge.
            String name = getPlayerName(info);
            int nameX = badgeX + PrivetBadgeRenderer.tabBadgeAdvance();
            Gui.drawRect(nameX - 1, y, nameX + mc.fontRendererObj.getStringWidth(name) + 1, y + 8, 0x80000000);
            mc.fontRendererObj.drawStringWithShadow(name, nameX, y, -1);
        }
    }

    private static String getPlayerName(NetworkPlayerInfo info) {
        IChatComponent display = info.getDisplayName();
        if (display != null) {
            return display.getFormattedText();
        }
        return ScorePlayerTeam.formatPlayerName(info.getPlayerTeam(), info.getGameProfile().getName());
    }
}
