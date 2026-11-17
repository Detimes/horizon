//! A server player's advancement progress for mods that carry it between servers, read and
//! replaced whole.
use super::{Game, native_server::PlayerRead};
use mod_api::PlayerRef;

impl Game<'_> {
    /// `PlayerAdvancements.save`'s JSON of a player's progress now.
    pub(super) fn player_advancements(&self, player: PlayerRef) -> Option<String> {
        if self.remote.is_some() {
            return None;
        }
        let progress = match self.player_handle_read(player)? {
            PlayerRead::Acting { .. } => &self.adv.progress,
            PlayerRead::Guest(g) => &g.advancements,
        };
        serde_json::to_string_pretty(&progress.to_json(&self.adv.tree, horizon_world::DATA_VERSION))
            .ok()
    }
    /// `PlayerAdvancements.reload` from `json`, then a full advancement packet: the player's
    /// visible list is forgotten, so the next update resets the client's tree.
    pub(super) fn player_set_advancements(&mut self, player: PlayerRef, json: &str) -> bool {
        if self.remote.is_some() {
            return false;
        }
        let Ok(value @ serde_json::Value::Object(_)) = serde_json::from_str(json) else {
            return false;
        };
        let conn = match self.player_handle_read(player) {
            Some(PlayerRead::Acting { .. }) => self.lan_acting,
            Some(PlayerRead::Guest(g)) => Some(g.conn),
            None => None,
        };
        let Some(conn) = conn else {
            return false;
        };
        let tree = self.advancement_tree();
        let mut progress = horizon_client::advancements::Progress::from_json(&value);
        progress.criteria.retain(|id, _| tree.by_id.contains_key(id));
        let set = self.native_as_player_handle(player, |game| {
            game.adv.progress = progress;
            game.adv.changed.clear();
        });
        if set && let Some(host) = self.lan.as_mut() {
            host.hub.guests.update_connection(conn, |g| {
                g.advancements_loaded = true;
                g.advancements_visible.clear();
            });
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_guest_turn_tests::with_guests;

    /// Progress read as the save writes it goes back in whole; an advancement this server
    /// lacks is dropped, and the client is sent the whole tree again.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn advancements_are_read_and_replaced_whole() {
        with_guests(|game| {
            game.advancement_tree();
            let b = game.player(13).unwrap();
            let json = r#"{"minecraft:story/root":{"criteria":{"crafting_table":"2026-11-11 10:00:00 +0000"},"done":true},"other:gone":{"criteria":{"x":"2026-11-11 10:00:00 +0000"},"done":true},"DataVersion":4000}"#;
            game.lan
                .as_mut()
                .unwrap()
                .hub
                .guests
                .update_connection(13, |g| g.advancements_visible = vec![true]);
            assert!(game.player_set_advancements(b, json));
            let guests = &game.lan.as_ref().unwrap().hub.guests;
            let g = &guests[guests.connection_index(13).unwrap()];
            assert!(g.advancements.has("minecraft:story/root", "crafting_table"));
            assert!(!g.advancements.criteria.contains_key("other:gone"));
            assert!(g.advancements_visible.is_empty(), "the next update resets the client");
            let read: serde_json::Value =
                serde_json::from_str(&game.player_advancements(b).unwrap()).unwrap();
            assert_eq!(
                read["minecraft:story/root"]["criteria"]["crafting_table"],
                "2026-11-11 10:00:00 +0000"
            );
            assert_eq!(read["minecraft:story/root"]["done"], true);
            assert!(read.get("DataVersion").is_some());
            assert!(!game.player_set_advancements(b, "[1, 2]"));
            assert!(!game.player_set_advancements(b, "not json"));
            assert!(game.player_set_advancements(b, "{}"));
            let guests = &game.lan.as_ref().unwrap().hub.guests;
            let g = &guests[guests.connection_index(13).unwrap()];
            assert!(g.advancements.criteria.is_empty());
        });
    }
}
