//! Unit tests of the command paths a permission node decides (`permission_gate`).

use super::permission_gate::{NodeAnswers, NodeGate, refuses, runnable_paths, split_usages};
use mod_api::{Command, CommandSource, PermissionError, PermissionLevel, PermissionNode, PermissionValueType};

fn boolean(name: &str) -> PermissionNode {
    PermissionNode {
        name: name.to_owned(),
        value_type: PermissionValueType::Boolean,
    }
}

fn integer(name: &str) -> PermissionNode {
    PermissionNode {
        name: name.to_owned(),
        value_type: PermissionValueType::Integer,
    }
}

/// Uuid 1 is allowed, uuid 2 refused, every other uuid fails.
struct Answers;

impl NodeAnswers for Answers {
    fn answer(&mut self, _node: &PermissionNode, uuid: u128) -> Result<bool, PermissionError> {
        match uuid {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(PermissionError::HandlerFailed),
        }
    }
}

fn words(path: &[&str]) -> Vec<String> {
    path.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn a_required_argument_path_runs_only_at_its_end() {
    assert_eq!(
        runnable_paths("pay <player:player> <amount:integer>"),
        vec![words(&["pay", "player", "amount"])]
    );
}

#[test]
fn an_optional_argument_also_runs_at_the_prefix_before_it() {
    assert_eq!(
        runnable_paths("balance [player:player]"),
        vec![words(&["balance"]), words(&["balance", "player"])]
    );
}

#[test]
fn without_a_custom_handler_usages_keep_their_levels_and_nothing_is_gated() {
    let usages = [
        Command::op("eco give <player:player> <amount:integer>", 3),
        Command::new("balance").with_node(&boolean("economy.balance")),
    ];
    let (out, gate, problems) = split_usages(&usages, false);
    assert!(gate.is_empty());
    assert!(problems.is_empty());
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].usage, "eco give <player:player> <amount:integer>");
    assert_eq!(out[0].level, 3);
    assert_eq!(out[1].level, 0);
}

#[test]
fn with_a_custom_handler_a_gated_usage_has_no_level_and_its_path_is_gated() {
    let usages = [Command::op("warp set <amount:integer>", 4).with_node(&boolean("warp.set"))];
    let (out, gate, problems) = split_usages(&usages, true);
    assert!(problems.is_empty());
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].level, 0);
    let path = words(&["warp", "set", "amount"]);
    assert_eq!(gate.node(&path).map(|node| node.name.as_str()), Some("warp.set"));
    assert!(gate.node(&words(&["warp"])).is_none());
}

#[test]
fn a_node_that_is_not_boolean_leaves_its_usage_out_with_a_problem() {
    let usages = [Command::new("limit <value:integer>").with_node(&integer("limit.value"))];
    let (out, gate, problems) = split_usages(&usages, true);
    assert!(out.is_empty());
    assert!(gate.is_empty());
    assert_eq!(problems.len(), 1);
}

#[test]
fn a_gated_path_shared_with_an_ungated_usage_is_left_out() {
    let usages = [
        Command::new("balance"),
        Command::new("balance [player:player]").with_node(&boolean("economy.others")),
    ];
    let (out, gate, problems) = split_usages(&usages, true);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].usage, "balance");
    assert!(gate.is_empty());
    assert_eq!(problems.len(), 1);
}

#[test]
fn a_gated_path_already_gated_by_another_node_is_left_out() {
    let usages = [
        Command::new("warp set <amount:integer>").with_node(&boolean("warp.first")),
        Command::new("warp set <amount:integer>").with_node(&boolean("warp.second")),
    ];
    let (out, gate, problems) = split_usages(&usages, true);
    assert_eq!(out.len(), 1);
    assert_eq!(
        gate.node(&words(&["warp", "set", "amount"])).map(|node| node.name.as_str()),
        Some("warp.first")
    );
    assert_eq!(problems.len(), 1);
}

#[test]
fn denied_paths_are_the_ones_the_answer_refuses_or_fails() {
    let usages = [
        Command::new("warp set <amount:integer>").with_node(&boolean("warp.set")),
    ];
    let (_, gate, _) = split_usages(&usages, true);
    // Uuid 1 is allowed and keeps the path; uuid 2 and an uuid that fails both lose it.
    assert_eq!(gate.denied_for(&mut Answers, 1), Vec::<Vec<String>>::new());
    assert_eq!(
        gate.denied_for(&mut Answers, 2),
        vec![words(&["warp", "set", "amount"])]
    );
    assert_eq!(
        gate.denied_for(&mut Answers, 3),
        vec![words(&["warp", "set", "amount"])]
    );
}

#[test]
fn a_gated_run_from_a_console_is_refused() {
    let console = CommandSource::Console {
        permission: PermissionLevel::Owners,
    };
    assert!(refuses(&mut Answers, &boolean("warp.set"), &console));
}

#[test]
fn an_empty_gate_denies_nothing() {
    let gate = NodeGate::default();
    assert!(gate.denied_for(&mut Answers, 2).is_empty());
    assert!(gate.node(&words(&["warp"])).is_none());
}
