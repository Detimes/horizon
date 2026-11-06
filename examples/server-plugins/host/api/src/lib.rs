//! Plugin host: a trait mod that loads server plugins from the server's `plugins/` folder
//! (`horizon-plugin-loader`) and reloads them while the server runs
//! (docs/design/server-plugin-reload.md). `/plugin list|load|unload|reload <name>` for operators
//! of level 4 and the console.
use horizon_client::mod_api::{
    Flow, Hooks, Host, NativeCommand, PerformCommandEvent, PerformConsoleCommandEvent,
    PlayerCommandEvent, PlayerLoggedInEvent, RegisterCommandsEvent, ServerStoppingEvent, Text,
};
use horizon_plugin_loader::{Calls, Plugins, Sender, root};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(PluginHost::default())
}

#[derive(Default)]
struct PluginHost {
    plugins: OnceLock<Plugins>,
}

/// Where a plugin's answer to a command goes.
enum Sink<'a> {
    /// A player's command before the game reads it (`player_command`).
    Replies(&'a mut Vec<Text>),
    /// A registered command's answer (`perform_command`, the console).
    Feedback,
    /// No command: the log.
    Log,
}

struct HostCalls<'a, 'h> {
    host: &'a mut (dyn Host + 'h),
    sink: Sink<'a>,
}

impl Calls for HostCalls<'_, '_> {
    fn log(&mut self, plugin: &str, line: &str) {
        log::info!("[{plugin}] {line}");
    }
    fn reply(&mut self, plugin: &str, line: &str) -> bool {
        let text = Text::literal(line.to_owned());
        match &mut self.sink {
            Sink::Replies(lines) => lines.push(text),
            Sink::Feedback => self.host.feedback(text),
            Sink::Log => log::info!("[{plugin}] {line}"),
        }
        true
    }
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.host.tell(player, Text::literal(line.to_owned()))
    }
    fn broadcast(&mut self, line: &str) {
        self.host.broadcast(Text::literal(line.to_owned()))
    }
    fn run_command(&mut self, line: &str) -> bool {
        self.host.run_command(line)
    }
}

impl PluginHost {
    /// The plugins in `plugins/` beside `server.properties`, loaded the first time a hook asks.
    fn plugins(&self, host: &dyn Host) -> &Plugins {
        self.plugins.get_or_init(|| {
            let base = host
                .config_dir()
                .ok()
                .and_then(|c| c.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."));
            let plugins = Plugins::new(base.join("plugins"));
            for line in plugins.load_all() {
                log::info!("Plugin host: {line}");
            }
            plugins
        })
    }

    fn admin(&self, host: &mut dyn Host, line: &str) -> Flow {
        let answer = self.plugins(host).admin(line);
        log::info!("Plugin host: {answer}");
        host.feedback(Text::literal(answer));
        Flow::Cancel
    }
}

fn taken(took: bool) -> Flow {
    if took { Flow::Cancel } else { Flow::Continue }
}

impl Hooks for PluginHost {
    fn register_commands(&self, host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        let plugins = self.plugins(host);
        event
            .commands
            .push(NativeCommand::op("plugin <action:word> [name:word]", 4));
        let commands = plugins.commands();
        plugins.mark_registered(
            commands
                .iter()
                .map(|(u, _)| root(u).to_owned())
                .chain(["plugin".to_owned()]),
        );
        for (usage, level) in commands {
            event.commands.push(NativeCommand::op(&usage, level));
        }
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        if root(event.command) == "plugin" {
            return self.admin(host, event.command);
        }
        let p = event.player;
        let sender = Sender {
            uuid: p.uuid,
            name: &p.name,
            permission: p.permission,
        };
        let plugins = self.plugins(host);
        let mut calls = HostCalls {
            host,
            sink: Sink::Feedback,
        };
        taken(plugins.command(&mut calls, Some(sender), event.command))
    }

    fn perform_console_command(
        &self,
        host: &mut dyn Host,
        event: &mut PerformConsoleCommandEvent<'_>,
    ) -> Flow {
        if root(event.command) == "plugin" {
            return self.admin(host, event.command);
        }
        let plugins = self.plugins(host);
        let mut calls = HostCalls {
            host,
            sink: Sink::Feedback,
        };
        taken(plugins.command(&mut calls, None, event.command))
    }

    /// A command missing from the tree (a plugin loaded after start added it): runs at level 0
    /// only, since this hook does not know the player's level.
    fn player_command(&self, host: &mut dyn Host, event: &mut PlayerCommandEvent<'_>) -> Flow {
        let plugins = self.plugins(host);
        if plugins.is_registered(root(event.command)) {
            return Flow::Continue;
        }
        let sender = Sender {
            uuid: event.player,
            name: event.name,
            permission: 0,
        };
        let mut calls = HostCalls {
            host,
            sink: Sink::Replies(&mut *event.replies),
        };
        taken(plugins.command(&mut calls, Some(sender), event.command))
    }

    fn player_logged_in(&self, host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        let plugins = self.plugins(host);
        let mut calls = HostCalls {
            host,
            sink: Sink::Log,
        };
        let player = Sender {
            uuid: event.player,
            name: event.name,
            permission: 0,
        };
        plugins.player_joined(&mut calls, player);
    }

    fn server_stopping(&self, _host: &mut dyn Host, _event: &mut ServerStoppingEvent) {
        if let Some(plugins) = self.plugins.get() {
            plugins.unload_all();
        }
    }
}
