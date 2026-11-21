use super::*;

impl Game<'_> {
    /// Raid trigger: vanilla `Raids.createOrExtendRaid(ServerPlayer, BlockPos)` for a bad omen.
    pub(in crate::game) fn native_raid_omen_trigger(
        &mut self,
        call: super::fire::RaidOmenTriggerCall<'_>,
    ) {
        self.fire_raid_omen_trigger(call);
    }
}
