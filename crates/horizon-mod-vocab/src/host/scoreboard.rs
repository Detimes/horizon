//! Host method declarations; access defaults and game forwarders are generated.
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `scoreboard` area, implemented by the game and reached through [`ServerHost`].
#[horizon_mod_api_macros::host_area]
pub trait ScoreboardServerHost {
    /// Side: server.
    /// The name of the team the entity is on (`Entity.getTeam`, Paper's
    /// `Scoreboard#getEntryTeam`), None when it is on no team. The scoreboard keys an entity
    /// by its UUID, so this reads the entry `team_join` writes for it. Reads only: a stale
    /// entity, a client, or an entity whose UUID the server cannot give gives None.
    #[access(WorldOnly)]
    fn entity_team(&mut self, entity: EntityRef) -> Option<String>;
}
