use mod_api::{Hooks, ModEntry, PlayerTeleportCauseEvent, ServerHost};
use std::sync::{Arc, Mutex};

struct Probe(Arc<Mutex<Vec<(u128, [f64; 3], [f64; 3], String, String)>>>);
#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn player_teleport_cause(
        &self,
        _: &mut dyn ServerHost,
        event: &mut PlayerTeleportCauseEvent<'_>,
    ) {
        self.0.lock().unwrap().push((
            event.player,
            event.from,
            event.to,
            event.dimension.to_string(),
            event.cause.to_string(),
        ));
    }
}

/// A command teleport is told with its cause, along with `player_teleported`.
#[test]
#[ignore = "daily"]
fn a_command_teleport_is_told_with_its_cause() {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let mods: mod_api::Mods =
        vec![ModEntry::new("tp-probe", Box::new(Probe(Arc::clone(&heard))))].into();
    mod_platform::testing::with_mods(mods, || {
        crate::game::native_guest_turn_tests::with_guests(|game| {
            game.teleported_because(13, "guest13", [0.0, 64.0, 0.0], [10.0, 70.0, -4.0], "minecraft:overworld", "command");
            assert_eq!(
                *heard.lock().unwrap(),
                [(13, [0.0, 64.0, 0.0], [10.0, 70.0, -4.0], "minecraft:overworld".to_owned(), "command".to_owned())]
            );
        });
    });
}
