//! An example server plugin: `/hello` and a greeting when a player joins. Build it with
//! `cargo build --release -p hello`, copy `target/release/libhello.so` to the server's
//! `plugins/hello.so`, and `/plugin reload hello`.
use horizon_plugin_abi::{Cx, PlayerInfo, ServerPlugin, export_plugin};
use std::sync::atomic::{AtomicU32, Ordering};

const GREETING: &str = "Hello from plugin v1";

#[derive(Default)]
struct Hello {
    /// How often `/hello` ran since this copy was loaded: a reload starts again at 0.
    calls: AtomicU32,
}

impl ServerPlugin for Hello {
    const NAME: &'static str = "hello";
    const VERSION: &'static str = "1.0.0";

    fn commands(&self) -> Vec<(&'static str, u8)> {
        vec![("hello", 0)]
    }

    fn command(&self, cx: &mut Cx<'_>, sender: Option<&PlayerInfo<'_>>, line: &str) -> bool {
        let word = line.split(' ').next();
        if !self
            .commands()
            .iter()
            .any(|(usage, _)| word == Some(*usage))
        {
            return false;
        }
        let n = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        let who = sender.map_or("console", |s| s.name);
        cx.reply(&format!("{GREETING}, {who} (call {n} of this load)"));
        true
    }

    fn player_joined(&self, cx: &mut Cx<'_>, player: &PlayerInfo<'_>) {
        cx.tell(
            player.uuid,
            &format!("{GREETING}: welcome, {}!", player.name),
        );
        cx.log(&format!("greeted {}", player.name));
    }
}

impl Drop for Hello {
    fn drop(&mut self) {
        eprintln!(
            "[hello {}] unloaded after {} calls",
            Self::VERSION,
            self.calls.load(Ordering::Relaxed)
        );
    }
}

export_plugin!(Hello, Hello::default());
