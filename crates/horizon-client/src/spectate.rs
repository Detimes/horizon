//! Spectating, as a native mod hears it (`player_start_spectating_entity`).
use std::sync::OnceLock;

/// The mods' listener: the executable installs it (`entry.rs`) before any player can spectate.
type Listener = fn(u128, &str, i32, &str);

static LISTENER: OnceLock<Listener> = OnceLock::new();

/// Installs the listener once; a second installation keeps the first.
pub fn install(listener: Listener) {
    let _ = LISTENER.set(listener);
}

fn player_start_spectating_entity(player: u128, name: &str, entity_id: i32, kind: &str) {
    if let Some(listener) = LISTENER.get() {
        listener(player, name, entity_id, kind);
    }
}

/// `player` (`name`) started spectating `entity_id`, whose type is `kind` (`minecraft:player` for
/// another player). Heard after the spectator's camera was set to it, on a server; the mirror of
/// `native_player_stop_spectating_entity`. API @API:player_start_spectating_entity@.
#[inline(never)]
pub fn native_player_start_spectating_entity(player: u128, name: &str, entity_id: i32, kind: &str) {
    player_start_spectating_entity(player, name, entity_id, kind);
}

#[cfg(test)]
mod tests {
    use super::{install, native_player_start_spectating_entity};
    use std::sync::Mutex;

    static SEEN: Mutex<Vec<(u128, String, i32, String)>> = Mutex::new(Vec::new());

    /// The hook hands the spectator, the target's id and its type to the installed listener.
    #[test]
    fn start_hook_reaches_the_installed_listener() {
        install(|player, name, entity_id, kind| {
            SEEN.lock()
                .unwrap()
                .push((player, name.to_owned(), entity_id, kind.to_owned()));
        });
        native_player_start_spectating_entity(7, "Watcher", 42, "minecraft:zombie");
        native_player_start_spectating_entity(7, "Watcher", 43, "minecraft:player");
        assert_eq!(
            *SEEN.lock().unwrap(),
            vec![
                (7, "Watcher".to_owned(), 42, "minecraft:zombie".to_owned()),
                (7, "Watcher".to_owned(), 43, "minecraft:player".to_owned()),
            ]
        );
    }
}
