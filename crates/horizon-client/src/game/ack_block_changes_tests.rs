//! `native_ack_block_changes`: outside a hook it keeps the highest sequence on the guest for the
//! tick to send; inside the player's own hook it queues the ack with the turn's packets; a
//! negative sequence or a player who isn't on returns false and changes nothing.
use crate::game::{Game, lan::LanSettings};
use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
use horizon_protocol::packets::{configuration::ClientInformation, play::Clientbound as C};
use std::sync::{Arc, atomic::AtomicI32, mpsc};

const PLAYER: u128 = 1;

fn fixture() -> Game<'static> {
    let mut game = crate::game::native_machines::tests::server_game();
    game.build_registries();
    game.lan_host(
        LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        },
        NetConfig {
            announce: false,
            ..NetConfig::loopback(0, "ack-test".into(), 1)
        },
        HubConfig {
            default_game_mode: 0,
            guest_commands: false,
            force_game_mode: false,
            hardcore: false,
            view_distance: 2,
            simulation_distance: 2,
            dimension: "minecraft:overworld".into(),
            allow_flight: false,
            seed: 0,
            max_players: 1,
            integrated: false,
            levels: Default::default(),
        },
    )
    .unwrap();
    let (tx, _rx) = mpsc::channel();
    let mut guest = Guest::new(
        Joined {
            conn: 1,
            uuid: PLAYER,
            name: "AckGuest".into(),
            properties: Vec::new(),
            info: ClientInformation::new(2),
            brand: "test".into(),
            sender: PlayerSender::new(tx, Arc::default()),
            latency: Arc::new(AtomicI32::new(0)),
            address: "127.0.0.1:25565".parse().unwrap(),
            expected: false,
            configured: Default::default(),
        },
        1,
        [0.5, 80.0, 0.5],
        2,
    );
    guest.placed = true;
    game.lan.as_mut().unwrap().hub.guests.push(guest);
    game
}

fn kept(game: &Game<'_>) -> i32 {
    game.lan.as_ref().unwrap().hub.guests[0].ack_up_to.get()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn ack_keeps_the_highest_sequence_and_refuses_negative_or_absent() {
    let mut game = fixture();
    assert!(!game.native_ack_block_changes(PLAYER, -1));
    assert_eq!(kept(&game), -1, "a negative sequence changes nothing");
    assert!(!game.native_ack_block_changes(2, 5));
    assert_eq!(kept(&game), -1, "a player who isn't on changes nothing");
    assert!(game.native_ack_block_changes(PLAYER, 7));
    assert!(game.native_ack_block_changes(PLAYER, 3));
    assert_eq!(kept(&game), 7, "the highest sequence is kept for the tick");
    assert!(game.guest_packets.is_empty());
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn ack_inside_the_players_own_hook_goes_with_the_turns_packets() {
    let mut game = fixture();
    game.lan_acting_uuid = Some(PLAYER);
    assert!(game.native_ack_block_changes(PLAYER, 4));
    assert!(!game.native_ack_block_changes(PLAYER, -2));
    assert!(matches!(
        game.guest_packets.as_slice(),
        [C::BlockChangedAck(a)] if a.sequence == 4
    ));
    assert_eq!(kept(&game), -1, "taken on only when the turn returns");
}
