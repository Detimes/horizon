#![cfg(test)]
//! The separate join hook of the spawn follow-up PR, heard through its native function with
//! literal values: the listener sees the values, and its cancel comes back.
use super::*;
use crate::game::native_entity_refs::tests::with_game;
use mod_api::{EntityJoinLevelReasonEvent, Flow, Hooks, ModEntry, ServerHost};

struct Probe;
#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn entity_join_level_reason(
        &self,
        _: &mut dyn ServerHost,
        event: &mut EntityJoinLevelReasonEvent,
    ) -> Flow {
        assert_eq!(event.dimension, "minecraft:overworld");
        assert_eq!(event.entity, 77);
        assert_eq!(event.reason, Some(mod_api::SpawnReason::Spawner));
        Flow::Cancel
    }
}

#[test]
#[ignore = "daily"]
fn the_cancel_of_the_join_reason_hook_comes_back() {
    let mods: mod_api::Mods = vec![ModEntry::new("probe", Box::new(Probe))].into();
    mod_platform::testing::with_mods(mods, || {
        with_game(|game| {
            let mut canceled = false;
            game.native_entity_join_level_reason(mod_api::__private::EntityJoinLevelReasonCall {
                dimension: "minecraft:overworld",
                entity: 77,
                kind: "minecraft:zombie",
                pos: [0.5, 64.0, 0.5],
                loaded_from_disk: false,
                reason: Some(mod_api::SpawnReason::Spawner),
                canceled: &mut canceled,
            });
            assert!(canceled);
        });
    });
}
