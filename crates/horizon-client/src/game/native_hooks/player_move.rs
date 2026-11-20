use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// Server player `player` (their UUID) would move from `from` to `to` (or turn from
    /// `from_rotation` to `to_rotation`), `on_ground` as the packet claims and `has_position` /
    /// `has_rotation` as it carries them; set `cancel` to veto the report. Before it is committed.
    /// Server-side.
    pub(in crate::game) fn native_player_move(
        &mut self,
        call: mod_api::__private::PlayerMoveCall<'_>,
    ) {
        self.fire_player_move(call);
    }
}
