//! Mod command paths that ask a permission node (design note "Command paths").
//!
//! Only while a custom permission handler is selected does a usage's node decide. Then the
//! usage's operator level is set aside, the node's answer decides who sees the path in their
//! command tree and who may run it, and an error refuses the run instead of falling back to a
//! level. With no custom handler a usage with a node keeps its level, as any command does.
//! Vanilla's commands and mod commands without a node never reach this module's decisions.

use mod_api::{Command, CommandSource, PermissionError, PermissionNode, PermissionValueType};
use std::collections::HashSet;

/// Something that answers a node for a player's UUID: the game's service in production.
pub(super) trait NodeAnswers {
    /// The node's answer for a player's `uuid`, online or offline.
    fn answer(&mut self, node: &PermissionNode, uuid: u128) -> Result<bool, PermissionError>;
}

impl NodeAnswers for crate::game::permissions::PermissionService {
    fn answer(&mut self, node: &PermissionNode, uuid: u128) -> Result<bool, PermissionError> {
        self.query_bool(node, Some(uuid), uuid)
    }
}

/// The runnable paths a usage names, each as the words and argument names the dispatcher's
/// nodes carry (`pay <player:player> <amount:integer>` is `pay`, `player`, `amount`). A path
/// ends where the next word is optional or the usage ends, so `balance [player:player]` runs at
/// `balance` and at `balance player`.
pub(super) fn runnable_paths(usage: &str) -> Vec<Vec<String>> {
    let words: Vec<&str> = usage.split_whitespace().collect();
    let mut names = Vec::new();
    let mut paths = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let name = word
            .trim_start_matches(|c: char| c == '<' || c == '[')
            .trim_end_matches(|c: char| c == '>' || c == ']')
            .split(':')
            .next()
            .unwrap_or_default();
        names.push(name.to_owned());
        let ends_here = words
            .get(index + 1)
            .is_none_or(|next| next.starts_with('['));
        if ends_here {
            paths.push(names.clone());
        }
    }
    paths
}

/// The usages a tree is built from, and the paths a node decides.
#[derive(Default)]
pub(super) struct NodeGate {
    /// Each gated runnable path and the node that answers it.
    paths: Vec<(Vec<String>, PermissionNode)>,
}

impl NodeGate {
    /// Whether no path asks a node.
    pub(super) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// The node that decides the path (dispatcher names, as [`path_names`] gives them).
    pub(super) fn node(&self, path: &[String]) -> Option<&PermissionNode> {
        self.paths
            .iter()
            .find(|(gated, _)| gated.as_slice() == path)
            .map(|(_, node)| node)
    }

    /// The gated paths the node refuses `uuid` for, which the player's tree leaves out. An
    /// error refuses the path too, and is logged.
    pub(super) fn denied_for<A: NodeAnswers>(&self, answers: &mut A, uuid: u128) -> Vec<Vec<String>> {
        self.paths
            .iter()
            .filter(|(_, node)| !allowed(answers, node, uuid))
            .map(|(path, _)| path.clone())
            .collect()
    }
}

/// Whether the node allows `uuid`; an error does not, and is logged with the node's name.
fn allowed<A: NodeAnswers>(answers: &mut A, node: &PermissionNode, uuid: u128) -> bool {
    match answers.answer(node, uuid) {
        Ok(answer) => answer,
        Err(error) => {
            log::warn!(
                "Horizon: permission node `{}` failed for a command ({error:?}); the path is refused",
                node.name
            );
            false
        }
    }
}

/// Whether a run of a gated path is refused for `origin`. Only a player has a UUID to ask about;
/// any other origin (console, command block, function) is refused, and the refusal is logged.
pub(super) fn refuses<A: NodeAnswers>(answers: &mut A, node: &PermissionNode, origin: &CommandSource) -> bool {
    let CommandSource::Player { player, .. } = origin else {
        log::warn!(
            "Horizon: permission node `{}` needs a player; a command from this source is refused",
            node.name
        );
        return true;
    };
    !allowed(answers, node, player.uuid())
}

/// The dispatcher's names along a parsed command (`Context::nodes`), without the root.
pub(super) fn path_names<R>(
    dispatcher: &horizon_client::command::Dispatcher,
    parsed: &[(usize, R)],
) -> Vec<String> {
    parsed
        .iter()
        .filter_map(|(id, _)| dispatcher.nodes.get(*id))
        .map(|node| node.name().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// Splits the mods' usages into the ones the tree is built from and the gate. With no custom
/// handler every usage goes on unchanged (its level decides) and the gate is empty. With one,
/// a usage with a node goes on at level 0 and its runnable paths are gated. Refused usages (a
/// node that is not Boolean, a path it shares with an ungated usage, or a path another node
/// gates) are left out and named in the returned problems.
pub(super) fn split_usages(usages: &[Command], custom_handler: bool) -> (Vec<Command>, NodeGate, Vec<String>) {
    let mut out = Vec::with_capacity(usages.len());
    let mut gate = NodeGate::default();
    let mut problems = Vec::new();
    let ungated: HashSet<Vec<String>> = usages
        .iter()
        .filter(|command| command.node.is_none())
        .flat_map(|command| runnable_paths(&command.usage))
        .collect();
    for command in usages {
        let Some(node) = &command.node else {
            out.push(command.clone());
            continue;
        };
        if !matches!(node.value_type, PermissionValueType::Boolean) {
            problems.push(format!(
                "`{}`: permission node `{}` is not a Boolean node",
                command.usage, node.name
            ));
            continue;
        }
        if !custom_handler {
            out.push(command.clone());
            continue;
        }
        let paths = runnable_paths(&command.usage);
        if let Some(path) = paths.iter().find(|path| ungated.contains(*path)) {
            problems.push(format!(
                "`{}`: its path `{}` is also an ungated usage's, so it is left out",
                command.usage,
                path.join(" ")
            ));
            continue;
        }
        if let Some(path) = paths.iter().find(|path| {
            gate.paths
                .iter()
                .any(|(gated, other)| gated == *path && other.name != node.name)
        }) {
            problems.push(format!(
                "`{}`: its path `{}` is gated by another permission node, so it is left out",
                command.usage,
                path.join(" ")
            ));
            continue;
        }
        for path in paths {
            if !gate.paths.iter().any(|(gated, _)| *gated == path) {
                gate.paths.push((path, node.clone()));
            }
        }
        out.push(Command::new(&command.usage));
    }
    (out, gate, problems)
}
