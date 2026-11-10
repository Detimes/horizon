//! The chunk columns a server player is sent or told to forget (`player_chunk_sent`, `player_chunk_forgotten`),
//! and whether a mod hears them at all (`player_chunk_listened`).

/// The mods' listeners: the executable installs them (`entry.rs`) before the first server tick.
pub struct Listeners {
    pub sent: fn(u128, &str, [i32; 2]),
    pub forgotten: fn(u128, &str, [i32; 2]),
    pub listened: fn(&mut bool),
}

static LISTENERS: std::sync::OnceLock<Listeners> = std::sync::OnceLock::new();

/// Installs the listeners once; a second installation keeps the first.
pub fn install(listeners: Listeners) {
    let _ = LISTENERS.set(listeners);
}

fn player_chunk_sent(player: u128, dimension: &str, chunk: [i32; 2]) {
    if let Some(listeners) = LISTENERS.get() {
        (listeners.sent)(player, dimension, chunk);
    }
}

fn player_chunk_forgotten(player: u128, dimension: &str, chunk: [i32; 2]) {
    if let Some(listeners) = LISTENERS.get() {
        (listeners.forgotten)(player, dimension, chunk);
    }
}

fn player_chunk_listened(listened: &mut bool) {
    if let Some(listeners) = LISTENERS.get() {
        (listeners.listened)(listened);
    }
}

/// A chunk column the hub sent a server player (`ClientboundLevelChunkWithLightPacket`), once per column in the
/// batch that carried it, after the batch was sent. The player's name is their uuid. Listen only.
/// API @API:player_chunk_events@.
#[inline(never)]
pub fn native_player_chunk_sent(player: u128, dimension: &str, chunk: [i32; 2]) {
    player_chunk_sent(player, dimension, chunk);
}

/// A chunk column a server player's client was told to forget (`ClientboundForgetLevelChunkPacket`), when the view
/// moved past it, before the packet is sent. Listen only. API @API:player_chunk_events@.
#[inline(never)]
pub fn native_player_chunk_forgotten(player: u128, dimension: &str, chunk: [i32; 2]) {
    player_chunk_forgotten(player, dimension, chunk);
}

/// Whether a mod hears the chunk columns the hub sends a player and forgets (`native_player_chunk_sent`,
/// `native_player_chunk_forgotten`). Asked once at the end of each server tick, and the hub records those chunks
/// only while a mod says yes; a mod that hooks either sets `listened`. API @API:player_chunk_events@.
#[inline(never)]
pub fn native_player_chunk_listened(listened: &mut bool) {
    player_chunk_listened(listened);
}
