use super::*;
use mod_api::{BanListKind, BanRecord};

/// The hook point. Does nothing in Horizon.
impl Game<'_> {
    /// An entry was added to (`added`) or removed from one of the dedicated server's ban lists. Server-side.
    pub(in crate::game) fn native_ban_changed(
        &mut self,
        kind: BanListKind,
        added: bool,
        record: BanRecord,
    ) {
        self.fire_ban_changed(kind, added, record);
    }
}
