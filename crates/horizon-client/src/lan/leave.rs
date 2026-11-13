//! Why a guest's connection ended: the cause the hub keeps for `player_logged_out`'s `reason`.

/// What ended a guest's connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveCause {
    /// The client closed it: it quit, its connection was reset, or it moved to another server.
    Quit,
    /// The server kicked the player (a kick, a ban, a duplicate login, a packet limit).
    Kicked,
    /// The keep-alive timeout: the client stopped answering.
    TimedOut,
    /// The connection failed: a malformed packet or a read error.
    Error,
}

impl LeaveCause {
    /// The cause as the mod API names it.
    pub fn quit_reason(self) -> mod_api::QuitReason {
        match self {
            Self::Quit => mod_api::QuitReason::Quit,
            Self::Kicked => mod_api::QuitReason::Kicked,
            Self::TimedOut => mod_api::QuitReason::TimedOut,
            Self::Error => mod_api::QuitReason::Error,
        }
    }
    /// What ended a connection the server did not end itself, from the reason the network gives.
    pub fn from_network(reason: &str) -> Self {
        if reason == "Disconnected" || reason == "server closed" {
            Self::Quit
        } else if reason == horizon_protocol::Error::timed_out().to_string() {
            Self::TimedOut
        } else {
            Self::Error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_connection_the_server_did_not_end_names_the_network_reason() {
        assert_eq!(LeaveCause::from_network("Disconnected"), LeaveCause::Quit);
        assert_eq!(LeaveCause::from_network("server closed"), LeaveCause::Quit);
        let timed_out = horizon_protocol::Error::timed_out().to_string();
        assert_eq!(LeaveCause::from_network(&timed_out), LeaveCause::TimedOut);
        assert_eq!(
            LeaveCause::from_network("Invalid packet 3: unexpected end of data"),
            LeaveCause::Error
        );
    }
}
