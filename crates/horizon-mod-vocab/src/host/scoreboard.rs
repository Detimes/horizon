//! Host method declarations; access defaults and game forwarders are generated.
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `scoreboard` area, implemented by the game and reached through [`ServerHost`].
#[horizon_mod_api_macros::host_area]
pub trait ScoreboardServerHost {
    /// Side: server.
    /// The score a holder has on an objective (`Scoreboard.getOrCreateScore`'s value, Paper's
    /// `Score#getScore` through `Objective#getScore`). The holder is the scoreboard's key: a
    /// player's name, an entity's UUID as text, or a name as written (`/scoreboard players`).
    /// None when the holder has no score on that objective, the objective does not exist, or
    /// this is a client, whose scoreboard the server keeps.
    #[access(WorldOnly)]
    fn scoreboard_score(&self, holder: &str, objective: &str) -> Option<i32>;
}
