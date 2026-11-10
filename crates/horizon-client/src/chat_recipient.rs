//! Whether a chat line reaches one receiver, as a native mod decides it (`native_chat_recipient`).
use std::sync::OnceLock;

/// The mods' listener: the executable installs it (`entry.rs`) before any chat is sent.
type Listener = fn(u128, &str, &str, u128, &mut bool);

static LISTENER: OnceLock<Listener> = OnceLock::new();

/// Installs the listener once; a second installation keeps the first.
pub fn install(listener: Listener) {
    let _ = LISTENER.set(listener);
}

fn chat_recipient(
    sender: u128,
    sender_name: &str,
    message: &str,
    recipient: u128,
    hidden: &mut bool,
) {
    if let Some(listener) = LISTENER.get() {
        listener(sender, sender_name, message, recipient, hidden);
    }
}

/// A public chat line from `sender` (`sender_name`, `message` as the sender typed it) is about to reach `recipient`:
/// a player's chat, after `native_player_chat` let it through (one it dropped is not heard here), or a signed `/say`
/// or `/me`. Set `hidden` to keep the line from that receiver only: the others still get it, and the signed content
/// is not changed, so its signature still verifies for them. Heard once for each receiver, before the line is sent
/// to them. Server-side. API @API:chat_recipient@.
#[inline(never)]
pub fn native_chat_recipient(
    sender: u128,
    sender_name: &str,
    message: &str,
    recipient: u128,
    hidden: &mut bool,
) {
    chat_recipient(sender, sender_name, message, recipient, hidden);
}
