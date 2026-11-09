//! Read-only block and tag queries for native server hooks.
use super::{Dim, Game};
use horizon_client::blocks::BlockFacts;
use horizon_client::mod_api::NativeLifecycleContext;
use horizon_client::{
    items::{ItemStack, Items},
    world::World,
};
use horizon_world::{
    chunk::BlockState,
    fluid::FluidState,
    registry::{BlockRegistry, BlockStateId},
};

/// Why a server block or tag cannot be read.
#[derive(Debug, PartialEq, Eq)]
pub enum NativeBlockReadError {
    /// The context is stale or the call is outside its server tick hook.
    InactiveContext,
    /// Tag queries require an authoritative server.
    NotServer,
    /// The world has not loaded its tag tables yet.
    TagsUnavailable,
    /// The dimension is unknown or is not currently open on the server.
    UnavailableDimension,
    OutOfBounds,
    Unloaded,
    InvalidState,
}

impl Game<'_> {
    /// The world lock is installed after its tag tables are built.
    fn native_tag_items(&self) -> Result<&Items, NativeBlockReadError> {
        if self.options.execution_role.is_client() {
            return Err(NativeBlockReadError::NotServer);
        }
        if self.session_lock.is_none() {
            return Err(NativeBlockReadError::TagsUnavailable);
        }
        Ok(self.items())
    }

    /// Test a block state's name in an active server tick hook. API 28.
    #[allow(dead_code, reason = "native mod API (docs/native-mods.md)")]
    pub(super) fn native_block_in_tag(
        &self,
        context: &NativeLifecycleContext,
        state: &BlockState,
        tag: &str,
    ) -> Result<bool, NativeBlockReadError> {
        if !self.native_lifecycle.permits_read(context) {
            return Err(NativeBlockReadError::InactiveContext);
        }
        self.native_server_block_in_tag(state, tag)
    }

    /// Test an item stack in an active server tick hook. API 28.
    #[allow(dead_code, reason = "native mod API (docs/native-mods.md)")]
    pub(super) fn native_item_in_tag(
        &self,
        context: &NativeLifecycleContext,
        stack: &ItemStack,
        tag: &str,
    ) -> Result<bool, NativeBlockReadError> {
        if !self.native_lifecycle.permits_read(context) {
            return Err(NativeBlockReadError::InactiveContext);
        }
        self.native_server_item_in_tag(stack, tag)
    }

    /// Test a block state's name against the server's merged block tags.
    pub(super) fn native_server_block_in_tag(
        &self,
        state: &BlockState,
        tag: &str,
    ) -> Result<bool, NativeBlockReadError> {
        Ok(self.native_tag_items()?.block_in_tag(tag, &state.name))
    }

    /// Test an item stack against the server's merged item tags.
    pub(super) fn native_server_item_in_tag(
        &self,
        stack: &ItemStack,
        tag: &str,
    ) -> Result<bool, NativeBlockReadError> {
        let items = self.native_tag_items()?;
        let item = if stack.count == 0 {
            "minecraft:air"
        } else {
            &stack.item
        };
        Ok(items.item_in_tag(tag, item))
    }

    /// Whether block `block` (`minecraft:oak_log`, as `Host::block` names
    /// one) is in block tag `tag` (`minecraft:logs`, or a mod's
    /// `create:fan_transparent`) of the server's merged tags, as
    /// `BlockState.is(TagKey)`: what `native_server_block_in_tag` answers for
    /// a state of that block, from any server hook. API @API:host_bridge@.
    pub(super) fn native_block_name_in_tag(
        &self,
        block: &str,
        tag: &str,
    ) -> Result<bool, NativeBlockReadError> {
        Ok(self.native_tag_items()?.block_in_tag(tag, block))
    }

    /// Read a loaded block in a server tick hook, without loading a chunk.
    /// The returned name and properties are owned and remain valid after the hook.
    #[allow(dead_code, reason = "native mod API (docs/native-mods.md)")]
    pub(super) fn native_server_block(
        &self,
        context: &NativeLifecycleContext,
        dimension: &str,
        pos: [i32; 3],
    ) -> Result<BlockState, NativeBlockReadError> {
        if !self.native_lifecycle.permits_read(context) {
            return Err(NativeBlockReadError::InactiveContext);
        }
        let dim = match dimension {
            "minecraft:overworld" => Dim::Overworld,
            "minecraft:the_nether" => Dim::Nether,
            "minecraft:the_end" => Dim::End,
            _ => return Err(NativeBlockReadError::UnavailableDimension),
        };
        let world = if dim == self.dim {
            Some(&self.world)
        } else {
            self.levels.parked_world(dim)
        }
        .ok_or(NativeBlockReadError::UnavailableDimension)?;
        read_block(world, self.pack.assets.registry, pos)
    }

    /// The current of the fluid at `pos` in `dimension`
    /// (`FluidState.getFlow(level, pos)`), normalised: the push an item in it
    /// gets, a water wheel's speed from the water around it. Zero where there
    /// is no fluid or it doesn't move; none where the chunk isn't loaded or
    /// the dimension isn't available (a client has only the level it is in,
    /// a server also its parked ones), so a caller can keep its last answer.
    /// API @API:fluid_flow@.
    pub(super) fn native_fluid_flow(&self, dimension: &str, pos: [i32; 3]) -> Option<[f64; 3]> {
        let dim = match dimension {
            "minecraft:overworld" => Dim::Overworld,
            "minecraft:the_nether" => Dim::Nether,
            "minecraft:the_end" => Dim::End,
            _ => return None,
        };
        let world = if dim == self.dim {
            &self.world
        } else {
            self.levels.parked_world(dim)?
        };
        if !world.loaded(pos[0], pos[2]) {
            return None;
        }
        Some(horizon_client::entities::fluid_flow(
            world,
            &self.res.motion().fluids,
            pos,
        ))
    }
}

/// What a block state is, from the game's block tables (no world lookup).
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBlockStateProps {
    /// `minecraft:air`, `cave_air` or `void_air`.
    pub is_air: bool,
    /// The `replaceable` flag: `Block.canBeReplaced` for a plain placement.
    pub replaceable: bool,
    /// The state's block is an `EntityBlock` (vanilla's `block_entity` flag).
    pub has_block_entity: bool,
    /// Its block entity type id (`minecraft:chest`). A sign's form decides
    /// first, then the vanilla name table; a mod block's type is its own name.
    /// None without a block entity, or for a type Horizon does not know.
    pub block_entity_type: Option<String>,
    /// Its fluid: the fluid id (`minecraft:flowing_water`), the amount 1..=8
    /// (8 for a source and a falling column) and whether it is a source.
    /// None for no fluid.
    pub fluid: Option<(String, u8, bool)>,
}

#[allow(dead_code)]
fn block_state_props(
    facts: &BlockFacts,
    registry: &BlockRegistry,
    id: BlockStateId,
) -> Option<NativeBlockStateProps> {
    let state = registry.state(id)?;
    let info = facts.get(id);
    let has_block_entity = info.block_entity;
    let block_entity_type = has_block_entity
        .then(|| {
            info.sign_form()
                .map(|form| form.block_entity_type())
                .or_else(|| horizon_client::lan::chunks::block_entity_type(&state.name))
                .map(str::to_owned)
                .or_else(|| (!state.name.starts_with("minecraft:")).then(|| state.name.clone()))
        })
        .flatten();
    let fluid = FluidState::from_block(state)
        .ok()
        .filter(|fluid| *fluid != FluidState::EMPTY)
        .map(|fluid| {
            (
                fluid.kind().name().to_owned(),
                fluid.amount(),
                fluid.is_source(),
            )
        });
    Some(NativeBlockStateProps {
        is_air: facts.is_air(id),
        replaceable: info.replaceable,
        has_block_entity,
        block_entity_type,
        fluid,
    })
}

impl Game<'_> {
    /// The block state `state`'s facts (`NativeBlockStateProps`): what a
    /// set-block tool needs per state. Read-only and cheap; none for a state
    /// the registry does not have. Same answer on a client and a server.
    /// API @API:block_state_props@.
    #[allow(dead_code)]
    pub(super) fn native_block_state_props(&self, state: u32) -> Option<NativeBlockStateProps> {
        block_state_props(
            self.res.facts(),
            self.pack.assets.registry,
            BlockStateId(state),
        )
    }
}

/// Resolve an existing world state, preserving unloaded and invalid states.
fn read_block(
    world: &World,
    registry: &BlockRegistry,
    [x, y, z]: [i32; 3],
) -> Result<BlockState, NativeBlockReadError> {
    if !(-30_000_000..30_000_000).contains(&x)
        || !(-30_000_000..30_000_000).contains(&z)
        || !(world.shape.min_y..world.shape.max_y()).contains(&y)
    {
        return Err(NativeBlockReadError::OutOfBounds);
    }
    if !world.loaded(x, z) {
        return Err(NativeBlockReadError::Unloaded);
    }
    registry
        .state(world.block(x, y, z))
        .cloned()
        .ok_or(NativeBlockReadError::InvalidState)
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::chunk::Chunk;
    use horizon_world::registry::BlockStateId;
    use std::sync::Arc;

    #[test]
    fn reads_only_loaded_in_bounds_states() {
        let facts = br#"{"defaults":{"minecraft:air":0,"test:pillar":1},"states":[
            ["air",true,false,"","minecraft:air"],
            ["pillar",false,false,"",{"id":"test:pillar","properties":{"axis":"x"}}]
        ]}"#;
        let registry = BlockRegistry::from_facts_json(&facts[..]).unwrap();
        let mut world = World::new(BlockStateId(0), 0);
        world.chunks.insert((-1, 0), Arc::new(Chunk::empty(-1, 0)));
        world.set(-1, 64, 1, BlockStateId(1)).unwrap();
        // The Nether's build range starts at y 0 (its level's shape).
        let mut nether = World::new(BlockStateId(0), 0);
        nether.shape = horizon_client::chunk::LevelShape::NETHER;
        nether.chunks.insert((-1, 0), Arc::new(Chunk::empty(-1, 0)));
        let block = read_block(&world, &registry, [-1, 64, 1]).unwrap();
        assert_eq!(block.name, "test:pillar");
        assert_eq!(block.properties["axis"], "x");
        assert_eq!(
            read_block(&world, &registry, [-1, 65, 1]).unwrap().name,
            "minecraft:air"
        );
        assert_eq!(
            read_block(&world, &registry, [0, 64, 1]),
            Err(NativeBlockReadError::Unloaded)
        );
        assert_eq!(
            read_block(&nether, &registry, [-1, -1, 1]),
            Err(NativeBlockReadError::OutOfBounds)
        );
        assert_eq!(
            read_block(&world, &registry, [30_000_000, 64, 1]),
            Err(NativeBlockReadError::OutOfBounds)
        );
        world.set(-1, 64, 1, BlockStateId(99)).unwrap();
        assert_eq!(
            read_block(&world, &registry, [-1, 64, 1]),
            Err(NativeBlockReadError::InvalidState)
        );
        assert_eq!(world.chunks.len(), 1, "reads must not load chunks");
    }

    #[test]
    fn block_state_props_name_block_entities_and_fluids() {
        let facts = br#"{"defaults":{"minecraft:air":0,"minecraft:stone":1,"minecraft:chest":2,
            "minecraft:water":3,"test:heater":5},"states":[
            ["air",true,false,"",{"id":"minecraft:air"}],
            ["stone",false,false,"",{"id":"minecraft:stone"}],
            ["chest",false,false,"",{"id":"minecraft:chest","properties":{"type":"single"}}],
            ["water",false,false,"",{"id":"minecraft:water","properties":{"level":"0"}}],
            ["water",false,false,"",{"id":"minecraft:water","properties":{"level":"3"}}],
            ["heater",false,false,"",{"id":"test:heater"}]
        ]}"#;
        let registry = BlockRegistry::from_facts_json(&facts[..]).unwrap();
        let root =
            std::env::temp_dir().join(format!("horizon-block-state-props-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let gameplay = serde_json::json!({"states": [
            {"id": 0, "replaceable": true},
            {"id": 1},
            {"id": 2, "block_entity": true},
            {"id": 3},
            {"id": 4},
            {"id": 5, "block_entity": true},
        ]});
        std::fs::write(root.join("gameplay-facts.json"), gameplay.to_string()).unwrap();
        let behaviours = serde_json::json!({
            "states": [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
            "blocks": [
                {"name": "minecraft:air", "class": ["Block"]},
                {"name": "minecraft:stone", "class": ["Block"]},
                {"name": "minecraft:chest", "class": ["Block"]},
                {"name": "minecraft:water", "class": ["Block"]},
                {"name": "test:heater", "class": ["Block"]}
            ],
        });
        let block_facts =
            BlockFacts::load_with_behaviour_json(&root, &registry, &behaviours).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        let props = |state: u32| block_state_props(&block_facts, &registry, BlockStateId(state));
        let chest = props(2).unwrap();
        assert!(chest.has_block_entity);
        assert_eq!(chest.block_entity_type.as_deref(), Some("minecraft:chest"));
        assert_eq!(chest.fluid, None);
        assert!(!chest.is_air);
        let stone = props(1).unwrap();
        assert!(!stone.has_block_entity);
        assert_eq!(stone.block_entity_type, None);
        assert_eq!(stone.fluid, None);
        assert!(!stone.replaceable);
        let air = props(0).unwrap();
        assert!(air.is_air && air.replaceable);
        assert_eq!(
            props(3).unwrap().fluid,
            Some(("minecraft:water".to_owned(), 8, true)),
            "level 0 is the source"
        );
        assert_eq!(
            props(4).unwrap().fluid,
            Some(("minecraft:flowing_water".to_owned(), 5, false)),
            "level 3 is flowing with 8 - 3 blocks"
        );
        assert_eq!(
            props(5).unwrap().block_entity_type.as_deref(),
            Some("test:heater"),
            "a mod block entity is typed by its block"
        );
        assert_eq!(props(6), None, "a state the registry lacks");
    }
}
