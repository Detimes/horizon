//! `entity_team`: the team an entity is on, read from the world's scoreboard.
use super::Game;
use mod_api::EntityRef;

impl Game<'_> {
    /// The team of the entity's UUID holder (`Scoreboard.getPlayersTeam`), None when it is on
    /// none. A stale entity or one with no UUID gives None (`entity_uuid`), and so does a client,
    /// whose scoreboard the server keeps.
    pub(super) fn entity_team(&mut self, entity: EntityRef) -> Option<String> {
        if self.remote.is_some() {
            return None;
        }
        let uuid = self.entity_uuid(entity)?;
        // A player is on the scoreboard by name (`Entity.getScoreboardName`, what `/team join` and
        // `team_join` with a player write); any other entity by UUID.
        let name = self
            .player(uuid)
            .filter(|player| player.entity() == entity)
            .and_then(|player| self.player_name(player));
        let key = horizon_protocol::crypto::uuid_string(uuid);
        name.iter()
            .chain(std::iter::once(&key))
            .find_map(|key| self.score.board.players_team(key))
            .map(|team| team.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_guest_turn_tests::with_guests;

    /// A player's entity reads the team its UUID is joined to, and no team for another player.
    #[test]
    #[ignore = "daily"]
    fn entity_team_reads_the_team_the_entity_joined() {
        with_guests(|game| {
            let a = game.player(12).unwrap();
            let b = game.player(13).unwrap();
            let uuid = game.entity_uuid(a.entity()).unwrap();
            assert_eq!(game.entity_team(a.entity()), None);
            game.score.board.add_team("red");
            game.score
                .board
                .add_to_team(&horizon_protocol::crypto::uuid_string(uuid), "red");
            assert_eq!(game.entity_team(a.entity()), Some("red".to_owned()));
            assert_eq!(game.entity_team(b.entity()), None);
            // A player joined by name, as `/team join` and `ScoreHolder::Player` do, reads its team too.
            game.score.board.add_team("blue");
            let name = game.player_name(b).unwrap();
            game.score.board.add_to_team(&name, "blue");
            assert_eq!(game.entity_team(b.entity()), Some("blue".to_owned()));
            let stale = mod_api::EntityRef::new(
                mod_api::WorldEpoch::new(a.entity().epoch().get() + 1),
                a.entity().id(),
            );
            assert_eq!(game.entity_team(stale), None);
        });
    }
}
