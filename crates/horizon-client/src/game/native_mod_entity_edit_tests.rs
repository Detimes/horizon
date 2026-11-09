//! Run alone: installs a process-wide trait mod. A listener that cancels `mod_entity_edit` makes
//! `native_mod_entity_edit_allowed` false for every kind of edit, and an uncancelled one leaves it true.
use super::super::kill_die_vibration_tests::game;
use super::*;
use crate::mod_api::{self, Flow, Hooks, Host, ModEntityEditEvent};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

type Heard = (Option<u128>, String, String, String, [f64; 3]);

#[derive(Default)]
struct Probe {
    heard: Mutex<Vec<Heard>>,
    cancel: AtomicBool,
}

impl Hooks for Arc<Probe> {
    fn mod_entity_edit(&self, _: &mut dyn Host, e: &mut ModEntityEditEvent<'_>) -> Flow {
        self.heard.lock().unwrap().push((
            e.player,
            e.dimension.to_owned(),
            e.kind.to_owned(),
            e.entity_type.to_owned(),
            e.pos,
        ));
        if self.cancel.load(Ordering::Relaxed) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_cancelling_listener_makes_the_entity_edit_check_false() {
    mod_api::in_own_process(
        "game::native_entities::native_mod_entity_edit_tests::a_cancelling_listener_makes_the_entity_edit_check_false",
        a_cancelling_listener_makes_the_entity_edit_check_false_in_own_process,
    );
}

fn a_cancelling_listener_makes_the_entity_edit_check_false_in_own_process() {
    const OVERWORLD: &str = "minecraft:overworld";
    let probe = Arc::new(Probe::default());
    assert!(
        mod_api::MODS
            .set(Arc::from(vec![Box::new(probe.clone()) as Box<dyn Hooks>]))
            .is_ok(),
        "run this test alone"
    );
    let mut game = game();
    let pos = [1.5, 64.0, -2.0];
    let heard = |p: &Probe| std::mem::take(&mut *p.heard.lock().unwrap());
    // Not cancelled: the check is true and the listener hears the spawn with its arguments.
    assert!(game.native_mod_entity_edit_allowed(
        Some(7),
        OVERWORLD,
        NativeEntityEditKind::Spawn,
        "minecraft:pig",
        pos,
    ));
    assert_eq!(
        heard(&probe),
        [(
            Some(7),
            OVERWORLD.to_owned(),
            "spawn".to_owned(),
            "minecraft:pig".to_owned(),
            pos
        )]
    );
    // Cancelled: false for each kind, with no player too, and each edit is heard.
    probe.cancel.store(true, Ordering::Relaxed);
    for (action, kind) in [
        (NativeEntityEditKind::Spawn, "spawn"),
        (NativeEntityEditKind::Remove, "remove"),
        (NativeEntityEditKind::Edit, "edit"),
    ] {
        assert!(!game.native_mod_entity_edit_allowed(
            None,
            OVERWORLD,
            action,
            "minecraft:pig",
            pos
        ));
        assert_eq!(
            heard(&probe),
            [(
                None,
                OVERWORLD.to_owned(),
                kind.to_owned(),
                "minecraft:pig".to_owned(),
                pos
            )]
        );
    }
}
