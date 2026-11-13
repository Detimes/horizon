//! Ending a guest's connection from the server, and the cause it is kept under.
use super::{HostEvent, Hub};
use crate::lan::leave::LeaveCause;
use horizon_core::nbt::Tag;
use horizon_protocol::text::translatable;

impl Hub {
    /// Disconnect a guest with a reason (ServerGamePacketListenerImpl.disconnect).
    pub fn kick(&mut self, conn: u32, reason: Tag) {
        self.disconnect_for(conn, reason, LeaveCause::Kicked);
    }

    /// keepConnectionAlive's `disconnect.timeout`: a kick whose cause is the timeout.
    pub(super) fn time_out(&mut self, conn: u32) {
        self.disconnect_for(
            conn,
            translatable("disconnect.timeout", &[]),
            LeaveCause::TimedOut,
        );
    }

    fn disconnect_for(&mut self, conn: u32, reason: Tag, cause: LeaveCause) {
        // The log's "lost connection" line gives the reason, as vanilla's.
        let text = self.english(&reason);
        if let Some((player, name)) = self.update_guest(conn, |g| {
            g.left_reason = text.clone();
            g.left_for(cause);
            g.sender.disconnect(reason);
            (g.uuid, g.name.clone())
        }) {
            self.pending_kicks.push(HostEvent::Kicked {
                conn,
                player,
                name,
                reason: text,
            });
        }
    }
}
