//! Packets an acting guest's turn queues for that guest (`Game::guest_packets`), sent when the
//! turn's action returns.

use super::hub::Hub;
use horizon_protocol::packets::play::Clientbound as C;

impl Hub {
    /// Sends one packet of a guest's turn. A block-change ack is not sent: it becomes the guest's
    /// pending ack (`Hub::ack`), which `tick_level` sends after that tick's block changes, as
    /// `ServerGamePacketListenerImpl.tick` does.
    pub fn send_turn_packet(&self, conn: u32, packet: &C) {
        match packet {
            C::BlockChangedAck(ack) => self.ack(conn, ack.sequence),
            _ => self.send_to(conn, packet),
        }
    }
}
