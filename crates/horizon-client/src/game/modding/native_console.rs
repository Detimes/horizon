//! The server's console for native mods: every logged line (`console_line`).
use super::Game;
use horizon_client::logging;

impl Game<'_> {
    /// `console_line` for each line logged since the last server tick, while
    /// a mod listens; the lines its listeners log are not kept.
    pub(super) fn native_console_lines(&mut self) {
        let listened = mod_api::__private::listened(mod_api::HookId::ConsoleLine);
        logging::set_tap(listened);
        if !listened {
            return;
        }
        for line in logging::take_tapped() {
            let level = match line.level {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN",
                _ => "INFO",
            };
            logging::untapped(|| self.native_console_line(level, &line.thread, &line.message));
        }
    }
}
