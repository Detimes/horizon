use super::*;

impl Game<'_> {
    /// Cancel a natural spawn at a position for the player it is for
    pub(in crate::game) fn native_spawn_position_check(&mut self, dimension: &str, pos: [f64; 3], player: PlayerRef, canceled: &mut bool) {
        self.fire_spawn_position_check(dimension, pos, player, canceled);
    }
}
