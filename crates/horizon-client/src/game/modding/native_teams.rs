//! Scoreboard teams for server mods (`/team add|join|leave|remove|modify color`).
use super::Game;
use horizon_client::text::COLORS;

/// Whether the scoreboard's save keeps `text` as it is: the disk writer empties an NBT string whose modified
/// UTF-8 form (U+0000 as two bytes, a character past the basic plane as a surrogate pair of three bytes each)
/// exceeds 65535 bytes, so a team or a holder named by it could not be loaded under that name.
fn saves_intact(text: &str) -> bool {
    let encoded: usize = text
        .chars()
        .map(|c| match c as u32 {
            0 => 2,
            1..=0x7f => 1,
            0x80..=0x7ff => 2,
            0x800..=0xffff => 3,
            _ => 6,
        })
        .sum();
    encoded <= usize::from(u16::MAX)
}

impl Game<'_> {
    /// `Scoreboard.addPlayerTeam` and `PlayerTeam.setColor`: `name` made when missing, its
    /// colour one of the sixteen chat colour names or none. True when the call changed
    /// something (a team made or a colour changed). False on a client, for an empty name,
    /// one with a space or one the saved scoreboard could not keep, for an unknown colour,
    /// and when the team already has that colour.
    pub(super) fn team_set(&mut self, name: &str, color: Option<&str>) -> bool {
        if self.remote.is_some()
            || name.is_empty()
            || name.contains(char::is_whitespace)
            || !saves_intact(name)
        {
            return false;
        }
        if color.is_some_and(|color| !COLORS.iter().any(|(_, known, _)| *known == color)) {
            return false;
        }
        self.load_scoreboard();
        if self
            .score
            .board
            .team(name)
            .is_some_and(|team| team.color.as_deref() == color)
        {
            return false;
        }
        self.score.board.add_team(name).color = color.map(str::to_owned);
        true
    }
    /// `Scoreboard.addPlayerToTeam`: `holder` leaves its team and joins `team`. False on a
    /// client, for an empty holder or one the saved scoreboard could not keep, for an
    /// unknown team, and when the holder is already on that team.
    pub(super) fn team_join(&mut self, team: &str, holder: &str) -> bool {
        if self.remote.is_some() || holder.is_empty() || !saves_intact(holder) {
            return false;
        }
        self.load_scoreboard();
        if self.score.board.team(team).is_none() {
            return false;
        }
        if self
            .score
            .board
            .players_team(holder)
            .is_some_and(|current| current.name == team)
        {
            return false;
        }
        self.score.board.add_to_team(holder, team);
        true
    }
    /// `Scoreboard.removePlayerFromTeam`: true when `holder` was on a team.
    pub(super) fn team_leave(&mut self, holder: &str) -> bool {
        if self.remote.is_some() {
            return false;
        }
        self.load_scoreboard();
        self.score.board.remove_from_team(holder)
    }
    /// `Scoreboard.removePlayerTeam`: true when the team existed.
    pub(super) fn team_remove(&mut self, name: &str) -> bool {
        if self.remote.is_some() {
            return false;
        }
        self.load_scoreboard();
        if self.score.board.team(name).is_none() {
            return false;
        }
        self.score.board.remove_team(name);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_guest_turn_tests::with_guests;

    /// Teams are made, coloured, joined, left and removed in the world's scoreboard, which
    /// the next tick's sync sends to the players; refused input changes nothing.
    #[test]
    #[ignore = "daily"]
    fn teams_are_kept_in_the_worlds_scoreboard() {
        with_guests(|game| {
            assert!(game.team_set("red", Some("red")));
            assert_eq!(
                game.score.board.team("red").unwrap().color.as_deref(),
                Some("red")
            );
            assert!(!game.team_set("blue", Some("not_a_colour")));
            assert!(game.score.board.team("blue").is_none());
            assert!(!game.team_set("two words", None));
            // a name the saved scoreboard would empty is refused; one at the limit is kept
            assert!(!game.team_set(&"a".repeat(65_536), None));
            assert!(!game.team_set(&"\u{0}".repeat(32_768), None));
            assert!(game.team_set(&"b".repeat(65_535), None));
            assert!(game.score.board.team(&"b".repeat(65_535)).is_some());
            assert!(
                !game.team_set("red", Some("red")),
                "that colour is already set"
            );
            assert!(
                game.team_set("red", None),
                "an existing team takes the change"
            );
            assert_eq!(game.score.board.team("red").unwrap().color, None);
            assert!(!game.team_join("missing", "guest12"));
            assert!(game.team_join("red", "guest12"));
            assert!(!game.team_join("red", "guest12"), "already on that team");
            assert!(!game.team_join("red", &"h".repeat(65_536)));
            assert!(game.score.board.allied("guest12", "guest12"));
            assert_eq!(
                game.score.board.players_team("guest12").unwrap().name,
                "red"
            );
            assert!(game.team_leave("guest12"));
            assert!(!game.team_leave("guest12"));
            assert!(game.team_join("red", "guest13"));
            assert!(game.team_remove("red"));
            assert!(game.score.board.players_team("guest13").is_none());
            assert!(!game.team_remove("red"));
        });
    }
}
