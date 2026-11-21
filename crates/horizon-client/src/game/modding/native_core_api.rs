//! What a server mod asks of one player's connection and of the server's files: a resource pack
//! offered in play, a player's own view distance, and the path of the server log.
use super::Game;
use horizon_client::server::{json::Json, properties::name_uuid};
use horizon_protocol::packets::{configuration::ResourcePackPush, play::Clientbound as C};
use mod_api::PlayerRef;
use std::path::PathBuf;

/// The longest url `StringUtil` of the protocol sends (`Utf8String` of 32767).
const MAX_URL_CHARS: usize = 32767;

impl Game<'_> {
    /// `Player.setResourcePack(url, hash, required, prompt)`: a `ClientboundResourcePackPushPacket`
    /// in play, its id the name UUID of the url as the `resource-pack` setting makes it.
    pub(super) fn player_send_resource_pack(
        &mut self,
        player: PlayerRef,
        url: &str,
        sha1: &str,
        required: bool,
        prompt: Option<&str>,
    ) -> bool {
        let hash_fits = sha1.is_empty() || (sha1.len() == 40 && sha1.bytes().all(|b| b.is_ascii_hexdigit()));
        if url.is_empty() || url.chars().count() > MAX_URL_CHARS || !hash_fits {
            return false;
        }
        let Ok(conn) = self.player_connection(player) else {
            return false;
        };
        let Some(guest) = self.lan.as_ref().and_then(|lan| lan.hub.guest(conn)) else {
            return false;
        };
        guest.send(&C::ResourcePackPush(ResourcePackPush {
            id: name_uuid(url.as_bytes()),
            url: url.to_owned(),
            hash: sha1.to_owned(),
            required,
            prompt: prompt
                .map(|text| horizon_client::lan::conn::component_tag(&Json::String(text.to_owned()))),
        }));
        true
    }
    /// `Player.setViewDistance`: the player's own radius, `Mth.clamp(chunks, 2, serverViewDistance)`
    /// as `ChunkMap.getPlayerViewDistance` clamps what a client asked for. Their chunk centre is
    /// forgotten so the next tick sends or drops the chunks that changed.
    pub(super) fn player_set_view_distance(&mut self, player: PlayerRef, chunks: i32) -> bool {
        let Ok(conn) = self.player_connection(player) else {
            return false;
        };
        let Some(lan) = self.lan.as_mut() else {
            return false;
        };
        let server = lan.hub.config.view_distance;
        lan.hub
            .update_guest(conn, |g| {
                g.view_distance = chunks.max(2).min(server);
                g.center = None;
            })
            .is_some()
    }
    /// The server log being written: `logs/latest.log` under the game's home.
    pub(super) fn log_file(&self) -> PathBuf {
        horizon_client::logging::logs_dir(&horizon_client::logging::game_home()).join("latest.log")
    }
}
