//! Commands that require a permission node (`Command::with_node`).
use super::native_guest_turn_tests::with_guests;
use super::native_permissions_tests::set_permission;
use mod_api::{Command, Host, Hooks, PlayerPermissionCheckEvent};
use std::sync::{Arc, Mutex};

/// Grants `test.granted`, denies `test.denied`, grants `test.guest12` to
/// guest 12 alone, and leaves every other node unset.
struct Provider(Arc<Mutex<Vec<String>>>);

#[mod_api::__subscriptions]
impl Hooks for Provider {
    fn player_permission_check(
        &self,
        _: &dyn Host,
        event: &mut PlayerPermissionCheckEvent<'_>,
    ) -> Option<bool> {
        match event.node {
            "test.granted" => Some(true),
            "test.denied" => Some(false),
            "test.guest12" if event.player == 12 && event.name == "guest12" => Some(true),
            _ => None,
        }
    }

    fn register_commands(
        &self,
        _: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::RegisterCommandsEvent<'_>,
    ) {
        event.commands.extend([
            Command::op("vip", 4).with_node("test.granted"),
            Command::op("vip extra", 4),
            Command::new("banned").with_node("test.denied"),
            Command::op("fallback", 2).with_node("test.unset"),
        ]);
    }

    fn run_mod_command(
        &self,
        _: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::RunModCommandEvent<'_>,
    ) -> mod_api::Flow {
        self.0.lock().unwrap().push(event.command.literals.join(" "));
        mod_api::Flow::Cancel
    }
}

fn with_provider(test: impl FnOnce(&mut super::Game<'_>, &Arc<Mutex<Vec<String>>>)) {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let mods = vec![mod_api::ModEntry::new(
        "permissions",
        Box::new(Provider(heard.clone())),
    )]
    .into();
    mod_api::testing::with_mods(mods, || with_guests(|game| test(game, &heard)));
}

/// A usage naming another node than its command's first usage is left out; the kept
/// node-gated usages are open to every level, and the node keeps the lowest level.
#[test]
fn a_command_keeps_one_node_and_its_lowest_level() {
    let (kept, nodes, problems) = super::native_server::command_nodes(vec![
        Command::op("home", 2).with_node("homes.use"),
        Command::new("home set").with_node("homes.use"),
        Command::new("home other").with_node("homes.other"),
        Command::op("home admin", 4),
        Command::op("plain", 3),
    ]);
    assert_eq!(
        kept.iter()
            .map(|c| (c.usage.as_str(), c.level))
            .collect::<Vec<_>>(),
        [("home", 0), ("home set", 0), ("plain", 3)]
    );
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes["home"], ("homes.use".to_owned(), 0));
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("home other"));
    assert!(problems[1].contains("home admin"));
}

/// A node-gated command runs and shows for a player the permissions mod grants it to,
/// whatever their level; a denied node refuses even an operator, and an unset one falls
/// back to the command's level.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_command_node_decides_who_runs_and_sees_the_command() {
    with_provider(|game, heard| {
        game.register_native_commands();
        let run = |game: &mut super::Game<'_>, conn: u32, permission: u8| {
            set_permission(game, conn, permission);
            game.as_guest(conn, |game| {
                game.ui.registries.permission = permission;
                for line in ["vip", "banned", "fallback", "vip extra"] {
                    game.run_command(line);
                }
            })
            .unwrap();
            std::mem::take(&mut *heard.lock().unwrap())
        };
        assert_eq!(run(game, 12, 0), ["vip"]);
        assert_eq!(run(game, 12, 2), ["vip", "fallback"]);
        assert_eq!(run(game, 12, 4), ["vip", "fallback"]);
        set_permission(game, 13, 0);
        game.native_filter_commands(Some(13));
        let hidden = |game: &super::Game<'_>, conn| {
            let host = game.lan.as_ref().unwrap();
            let mut hidden = host.hub.guest(conn).unwrap().hidden_commands.clone();
            hidden.sort();
            hidden
        };
        assert_eq!(hidden(game, 13), ["banned", "fallback"]);
        set_permission(game, 13, 2);
        game.native_filter_commands(Some(13));
        assert_eq!(hidden(game, 13), ["banned"]);
    });
}
