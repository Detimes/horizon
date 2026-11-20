//! `player_pong`: the client's answer to a ping is told to the game once, with its id, in order.
use super::pending_move_tests::{drain, standing};
use super::*;
use horizon_protocol::packets::{configuration::Ping, play::Serverbound as S};

fn pongs(events: &[HostEvent]) -> Vec<(u32, i32)> {
    events
        .iter()
        .filter_map(|e| match e {
            HostEvent::Pong { conn, id } => Some((*conn, *id)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_pong_is_told_to_the_game_in_order_with_its_id() {
    let mut hub = standing(
        false,
        vec![S::Pong(Ping { id: 7 }), S::Pong(Ping { id: -3 })],
    );
    hub.mods_loaded = true;
    let conn = hub.guests[0].conn;
    assert_eq!(pongs(&drain(&mut hub)), [(conn, 7), (conn, -3)]);
}

#[test]
fn with_no_mod_loaded_a_pong_queues_nothing() {
    let mut hub = standing(false, vec![S::Pong(Ping { id: 7 })]);
    assert!(pongs(&drain(&mut hub)).is_empty());
}
