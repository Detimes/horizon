//! Why a guest leaves, for `player_logged_out_v2`: the timeout the hub decides on top of the kicks it records
//! (`Guest::kicked_for`) and the connection's own cause (`conn::quit_cause`). The first cause kept is the reason.
use super::hub::Hub;
use horizon_protocol::text::translatable;

impl Hub {
    /// Disconnects guest `conn` for a timeout (`disconnect.timeout`), as `kick` does, with the cause `TimedOut`
    /// unless an earlier one came first.
    pub fn time_out(&mut self, conn: u32) {
        if let Some(g) = self.guest_mut(conn) {
            g.quit.get_or_insert(crate::mod_api::NativeQuitReason::TimedOut);
        }
        self.kick(conn, translatable("disconnect.timeout", &[]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joined, Lan, NetConfig, PlayerSender, chunks::ChunkEncoder, conn::quit_cause, data,
        guest::Guest, hub::HubConfig,
    };
    use crate::mod_api::NativeQuitReason;
    use horizon_protocol::text::literal;
    use std::sync::{Arc, atomic::AtomicI32, mpsc::channel};

    fn hub() -> Hub {
        let sync = Arc::new(data::SyncData::empty());
        let lan = Lan::start_with(
            sync.clone(),
            NetConfig {
                announce: false,
                ..NetConfig::loopback(0, "QuitTest".into(), 4)
            },
        )
        .unwrap();
        let encoder = ChunkEncoder::new(
            [] as [(Arc<str>, bool); 0],
            &horizon_protocol::registry::Builtin::default(),
            &[],
            &[],
        );
        let config = HubConfig {
            default_game_mode: 0,
            guest_commands: false,
            force_game_mode: false,
            hardcore: false,
            view_distance: 4,
            simulation_distance: 4,
            dimension: "minecraft:overworld".into(),
            allow_flight: false,
            seed: 0,
            max_players: 4,
            integrated: false,
            levels: Default::default(),
        };
        Hub::new(lan, sync, encoder, config)
    }

    fn guest(conn: u32) -> Guest {
        let (tx, _rx) = channel();
        Guest::new(
            Joined {
                conn,
                uuid: conn.into(),
                name: "Quitter".into(),
                properties: Vec::new(),
                info: horizon_protocol::packets::configuration::ClientInformation::new(4),
                brand: "vanilla".into(),
                sender: PlayerSender::new(tx, Arc::default()),
                latency: Arc::new(AtomicI32::new(0)),
                address: "127.0.0.1:25565".parse().unwrap(),
                expected: false,
                configured: Default::default(),
            },
            conn as i32,
            [0.5, 64.0, 0.5],
            4,
        )
    }

    #[test]
    fn the_connection_ends_map_to_their_causes() {
        assert_eq!(quit_cause("Disconnected"), NativeQuitReason::Disconnected);
        let timed_out = horizon_protocol::Error::timed_out().to_string();
        assert_eq!(quit_cause(&timed_out), NativeQuitReason::TimedOut);
        assert_eq!(
            quit_cause("Invalid packet 3: unexpected end of data"),
            NativeQuitReason::Error("Invalid packet 3: unexpected end of data".into())
        );
    }

    #[test]
    fn a_guest_left_without_a_cause_is_a_plain_disconnect() {
        assert_eq!(guest(1).quit_reason(), NativeQuitReason::Disconnected);
    }

    #[test]
    fn the_first_cause_is_the_reason() {
        let mut g = guest(1);
        g.kicked_for("Kicked by an operator".into());
        let g = g.with_quit(NativeQuitReason::Disconnected);
        assert_eq!(
            g.quit_reason(),
            NativeQuitReason::Kicked("Kicked by an operator".into())
        );
        assert_eq!(g.left_reason, "Kicked by an operator");
    }

    #[test]
    fn a_silent_guest_times_out_and_a_kicked_one_keeps_its_kick() {
        let mut hub = hub();
        hub.guests.push(guest(1));
        hub.guests.push(guest(2));
        hub.time_out(1);
        hub.kick(2, literal("Bye"));
        hub.time_out(2);
        assert_eq!(
            hub.guest_mut(1).unwrap().quit_reason(),
            NativeQuitReason::TimedOut
        );
        assert_eq!(
            hub.guest_mut(2).unwrap().quit_reason(),
            NativeQuitReason::Kicked("Bye".into())
        );
    }
}
