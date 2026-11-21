use super::*;

impl Game<'_> {
    /// Raid trigger: vanilla `Raids.createOrExtendRaid(ServerPlayer, BlockPos)` for a bad omen.
    pub(in crate::game) fn native_raid_omen_trigger(&mut self, dimension: &str, pos: [i32; 3], player: PlayerRef, canceled: &mut bool) {
        self.fire_raid_omen_trigger(dimension, pos, player, canceled);
    }
}
