//! `break_block` and `left_click_block` name the acting guest: a guest's break and click carry
//! that guest's uuid, the game's own player's break carries none, and a cancelled click keeps
//! the block.
use super::native_entity_refs::tests::with_game;
use mod_api::{BreakBlockEvent, Flow, Hooks, LeftClickBlockEvent, ServerHost};
use std::sync::{Arc, Mutex};

/// Records each hooked event as (hook, the player it names), and cancels clicks when asked.
#[derive(Clone)]
struct Players {
    seen: Arc<Mutex<Vec<(&'static str, Option<u128>)>>>,
    cancel_clicks: bool,
}

#[mod_api::__subscriptions]
impl Hooks for Players {
    fn break_block(&self, _host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        self.seen.lock().unwrap().push(("break", event.player));
        Flow::Continue
    }

    fn left_click_block(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut LeftClickBlockEvent,
    ) -> Flow {
        self.seen
            .lock()
            .unwrap()
            .push(("click", Some(event.player)));
        if self.cancel_clicks {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

fn mods_with(seen: &Arc<Mutex<Vec<(&'static str, Option<u128>)>>>, cancel_clicks: bool) -> mod_api::Mods {
    vec![mod_api::ModEntry::new(
        "players",
        Box::new(Players {
            seen: seen.clone(),
            cancel_clicks,
        }),
    )]
    .into()
}

/// A guest's break names 123, the game's own player's break names nobody.
#[test]
#[ignore = "daily"]
fn break_block_names_the_guest_and_none_for_the_game_player() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    mod_platform::testing::with_mods(mods_with(&seen, false), || {
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            let stone = game.pack.assets.registry.default_state("minecraft:stone").unwrap();
            let (mut canceled, mut xp) = (false, None);
            game.native_break_block([1, 64, 1], stone, Some(123), &mut canceled, &mut xp);
            game.native_break_block([1, 64, 1], stone, None, &mut canceled, &mut xp);
            assert!(!canceled);
            assert_eq!(
                *seen.lock().unwrap(),
                vec![("break", Some(123)), ("break", None)]
            );
        });
    });
}

/// A guest's left click names 123, and a listener's cancel keeps the block (`canceled` true).
#[test]
#[ignore = "daily"]
fn left_click_block_names_the_clicking_guest_and_a_cancel_is_reported() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    mod_platform::testing::with_mods(mods_with(&seen, true), || {
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            let stone = game.pack.assets.registry.default_state("minecraft:stone").unwrap();
            let mut canceled = false;
            game.native_left_click_block([1, 64, 1], [0, 1, 0], stone, 123, &mut canceled);
            assert!(canceled);
            assert_eq!(*seen.lock().unwrap(), vec![("click", Some(123))]);
        });
    });
}
