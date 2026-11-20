//! Saved data from older versions upgraded for mods: one block entity's or entity's NBT through the datafixers the
//! chunk loaders run (`upgrade_block_entity_nbt`, `upgrade_entity_nbt`).
use super::Game;
use horizon_core::nbt::Compound;
use horizon_world::datafix::{self, UpgradeError};
use mod_api::{DataKind, NbtUpgradeError};

impl Game<'_> {
    /// A block entity's NBT saved at `data_version`, upgraded as the chunk loader upgrades it, or the refusal.
    pub(in crate::game) fn upgrade_block_entity_nbt(
        &self,
        data_version: i32,
        nbt: Compound,
    ) -> Result<Compound, NbtUpgradeError> {
        datafix::upgrade_block_entity(data_version, nbt).map_err(refusal)
    }

    /// One saved value of `kind` at `from_version`, upgraded up to the game's DataVersion, or the refusal.
    pub(in crate::game) fn data_fix(
        &self,
        kind: DataKind,
        nbt: Compound,
        from_version: i32,
        to_version: i32,
    ) -> Result<Compound, NbtUpgradeError> {
        if to_version != horizon_core::version::DATA_VERSION {
            return Err(NbtUpgradeError::TargetNotCurrent(to_version));
        }
        match kind {
            DataKind::BlockState => self.upgrade_block_state_nbt(from_version, nbt),
            DataKind::BlockEntity => self.upgrade_block_entity_nbt(from_version, nbt),
            DataKind::Entity => self.upgrade_entity_nbt(from_version, nbt),
        }
    }

    /// A block state's NBT saved at `data_version`, upgraded as the datafixers upgrade it, or the refusal.
    pub(in crate::game) fn upgrade_block_state_nbt(
        &self,
        data_version: i32,
        nbt: Compound,
    ) -> Result<Compound, NbtUpgradeError> {
        datafix::upgrade_block_state(data_version, nbt).map_err(refusal)
    }

    /// An entity's NBT saved at `data_version`, upgraded as the entity chunk loader upgrades it, or the refusal.
    pub(in crate::game) fn upgrade_entity_nbt(
        &self,
        data_version: i32,
        nbt: Compound,
    ) -> Result<Compound, NbtUpgradeError> {
        datafix::upgrade_entity(data_version, nbt).map_err(refusal)
    }
}

/// The API's name for the datafixer's refusal.
fn refusal(error: UpgradeError) -> NbtUpgradeError {
    match error {
        UpgradeError::NegativeVersion(v) => NbtUpgradeError::NegativeVersion(v),
        UpgradeError::FutureVersion(v) => NbtUpgradeError::FutureVersion(v),
        UpgradeError::TooDeep => NbtUpgradeError::TooDeep,
        UpgradeError::UpgradedTooDeep => NbtUpgradeError::UpgradedTooDeep,
    }
}
