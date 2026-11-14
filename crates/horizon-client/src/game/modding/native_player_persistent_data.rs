//! The compound mods keep on a server player (NeoForge's `Entity.getPersistentData`), read and
//! replaced whole, for mods that carry a player between servers.
use super::{Game, native_server::PlayerRead};
use horizon_core::nbt::{Compound, Tag};
use mod_api::PlayerRef;

/// The key of the mods' compound in a player's saved data (NeoForge's `Entity.PERSISTED_NBT_TAG`).
const PERSISTENT_DATA: &str = "NeoForgeData";

/// The mods' compound in a player's saved data; empty when there is none.
fn persistent_data(saved: Option<&Tag>) -> Compound {
    match saved.and_then(|t| t.get(PERSISTENT_DATA)) {
        Some(Tag::Compound(data)) => data.clone(),
        _ => Compound::new(),
    }
}

/// Puts `data` in a player's saved data as the mods' compound; an empty one takes the key away.
fn set_persistent_data(saved: &mut Option<Tag>, data: Compound) {
    if !matches!(saved, Some(Tag::Compound(_))) {
        *saved = Some(Tag::Compound(Compound::new()));
    }
    let Some(Tag::Compound(root)) = saved else {
        return;
    };
    if data.is_empty() {
        root.remove(&PERSISTENT_DATA.into());
    } else {
        root.insert(PERSISTENT_DATA.into(), Tag::Compound(data));
    }
}

impl Game<'_> {
    /// The mods' compound on a player: the acting player's saved data is the level's for their
    /// turn, another's is their record's.
    pub(super) fn player_persistent_data(&self, player: PlayerRef) -> Option<Compound> {
        if self.remote.is_some() {
            return None;
        }
        Some(match self.player_handle_read(player)? {
            PlayerRead::Acting { .. } => persistent_data(self.level.player_extra.as_ref()),
            PlayerRead::Guest(g) => persistent_data(g.saved.as_ref()),
        })
    }

    /// Replaces the mods' compound on a living player, in their turn so it lands in the record
    /// that is saved. Data that would sit 512 levels deep or more in the player's file
    /// (`isTooDeep`, as `/data merge` refuses it) is refused with nothing written.
    pub(super) fn player_set_persistent_data(&mut self, player: PlayerRef, data: Compound) -> bool {
        if self.remote.is_some()
            // The compound sits one level under the root of the player's file.
            || data
                .values()
                .any(|tag| horizon_client::nbt_path::too_deep(tag, 2))
        {
            return false;
        }
        self.native_as_player_handle(player, |game| {
            set_persistent_data(&mut game.level.player_extra, data);
        })
    }
}
