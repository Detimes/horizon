use super::*;

impl Game<'_> {
    /// A server player was teleported, with the cause. Server-side.
    pub(in crate::game) fn native_player_teleport_cause(
        &mut self,
        call: mod_api::__private::PlayerTeleportCauseCall<'_>,
    ) {
        self.fire_player_teleport_cause(call);
    }
    /// `native_player_teleported`, then `native_player_teleport_cause` with the same teleport.
    pub(in crate::game) fn teleported_because(
        &mut self,
        player: u128,
        name: &str,
        from: [f64; 3],
        to: [f64; 3],
        dimension: &str,
        cause: &str,
    ) {
        self.native_player_teleported(player, name, from, to, dimension);
        self.native_player_teleport_cause(mod_api::__private::PlayerTeleportCauseCall {
            player,
            name,
            from,
            to,
            dimension,
            cause,
        });
    }
}
