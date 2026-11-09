//! The chunk columns the hub sends a player and forgets (`ClientboundLevelChunkWithLightPacket`,
//! `ClientboundForgetLevelChunkPacket`), for the mods that hear them (`native_player_chunk_sent`,
//! `native_player_chunk_forgotten`). The hub records them only while a mod listens
//! (`native_player_chunk_listened`, asked at the end of each server tick); the game takes them
//! there. Recording costs one relaxed load while nobody listens.
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
};

/// One chunk column a player was sent (`sent`) or forgot.
pub struct ChunkEvent {
    pub player: u128,
    pub dimension: String,
    pub chunk: [i32; 2],
    pub sent: bool,
}

static LISTENING: AtomicBool = AtomicBool::new(false);
static EVENTS: Mutex<Vec<ChunkEvent>> = Mutex::new(Vec::new());

fn events() -> MutexGuard<'static, Vec<ChunkEvent>> {
    EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether the mods hear chunks now (set by the game at the end of each server tick).
pub fn listening() -> bool {
    LISTENING.load(Ordering::Relaxed)
}

/// Record from now on whether a mod listens to the chunks sent and forgotten.
pub fn set_listening(on: bool) {
    LISTENING.store(on, Ordering::Relaxed);
}

/// The chunks `chunks` the hub sends `player` in `dimension`: recorded while a mod listens, and
/// added to `kept`, the columns the player holds.
pub fn sent(
    player: u128,
    dimension: &str,
    chunks: Vec<(i32, i32)>,
    kept: &mut impl Extend<(i32, i32)>,
) {
    record(player, dimension, &chunks, true);
    kept.extend(chunks);
}

/// The chunks `chunks` the hub is about to forget for `player` in `dimension`: recorded while a
/// mod listens, and handed back as they were.
pub fn forgetting(player: u128, dimension: &str, chunks: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    record(player, dimension, &chunks, false);
    chunks
}

/// Every event recorded since the last call, in order.
pub fn take() -> Vec<ChunkEvent> {
    std::mem::take(&mut *events())
}

fn record(player: u128, dimension: &str, chunks: &[(i32, i32)], sent: bool) {
    if !listening() || chunks.is_empty() {
        return;
    }
    events().extend(chunks.iter().map(|&(x, z)| ChunkEvent {
        player,
        dimension: dimension.to_owned(),
        chunk: [x, z],
        sent,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests share the process-wide switch and queue: one at a time.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn nothing_is_recorded_while_no_mod_listens() {
        let _guard = LOCK.lock().unwrap();
        set_listening(false);
        take();
        let mut kept = Vec::new();
        sent(7, "minecraft:overworld", vec![(1, 2)], &mut kept);
        assert_eq!(kept, [(1, 2)]);
        let chunks = forgetting(7, "minecraft:overworld", vec![(1, 2)]);
        assert_eq!(chunks, [(1, 2)]);
        assert!(take().is_empty());
    }

    #[test]
    fn a_listened_send_and_forget_are_recorded_in_order_and_handed_back() {
        let _guard = LOCK.lock().unwrap();
        set_listening(true);
        take();
        let mut kept = Vec::new();
        sent(9, "minecraft:the_nether", vec![(-3, 4), (0, 0)], &mut kept);
        assert_eq!(kept, [(-3, 4), (0, 0)]);
        assert_eq!(
            forgetting(9, "minecraft:the_nether", vec![(-3, 4)]),
            [(-3, 4)]
        );
        sent(10, "minecraft:overworld", Vec::new(), &mut kept);
        set_listening(false);
        let events = take();
        let seen: Vec<_> = events
            .iter()
            .map(|e| (e.player, e.dimension.as_str(), e.chunk, e.sent))
            .collect();
        assert_eq!(
            seen,
            [
                (9, "minecraft:the_nether", [-3, 4], true),
                (9, "minecraft:the_nether", [0, 0], true),
                (9, "minecraft:the_nether", [-3, 4], false),
            ]
        );
        assert!(take().is_empty());
    }
}
