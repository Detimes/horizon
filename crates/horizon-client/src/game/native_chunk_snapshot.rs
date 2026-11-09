//! Read-only chunk snapshots for native server mods (docs/native-mods.md,
//! "Server hooks for plugin ports: world"): a web map or a backup tool takes
//! a loaded chunk as it is now and reads it later, off the server thread,
//! with the block map colours and biome tints to draw it.
use super::{Dim, Game};
use horizon_client::{biome::BiomeColors, blocks::MAP_COLORS, chunk::Chunk};
use horizon_core::{biome::BiomeZoom, position::BlockPos};
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

/// A loaded chunk as it was when `native_chunk_snapshot` took it: its block
/// states and biomes, shared with the level until the level changes them
/// (copy on write), so taking one is cheap and holding it doesn't stop the
/// level. It can be sent to another thread. Coordinates are the level's
/// block coordinates.
#[derive(Clone)]
pub struct NativeChunkSnapshot {
    chunk: Arc<Chunk>,
    pos: [i32; 2],
    min_y: i32,
    top_y: i32,
    ceiling: bool,
    zoom: BiomeZoom,
    /// Air, cave air and void air (`BlockState.isAir`).
    air: [Option<BlockStateId>; 3],
}

impl NativeChunkSnapshot {
    /// The chunk's position, `[x, z]` (block coordinates `>> 4`).
    pub fn chunk(&self) -> [i32; 2] {
        self.pos
    }
    /// The level's lowest block `y` and one above its highest
    /// (`getMinY`, `getMaxY` + 1).
    pub fn height(&self) -> (i32, i32) {
        (self.min_y, self.top_y)
    }
    /// Whether the level has a ceiling (`DimensionType.hasCeiling`: the Nether).
    pub fn has_ceiling(&self) -> bool {
        self.ceiling
    }
    /// The block state at `x, y, z` (`getBlockState`), as the state helpers
    /// take it (`native_state_block`); None outside this chunk or the level's
    /// height.
    pub fn block_state(&self, x: i32, y: i32, z: i32) -> Option<BlockStateId> {
        let inside =
            (x >> 4, z >> 4) == (self.pos[0], self.pos[1]) && (self.min_y..self.top_y).contains(&y);
        inside.then(|| self.chunk.block(x & 15, y, z & 15))
    }
    /// `getHeight(Heightmap.Types.WORLD_SURFACE, x, z)`: one above the
    /// highest block that isn't air, the level's lowest `y` in an empty
    /// column. None outside this chunk.
    pub fn surface(&self, x: i32, z: i32) -> Option<i32> {
        let air = self.air;
        ((x >> 4, z >> 4) == (self.pos[0], self.pos[1])).then(|| {
            self.chunk
                .world_surface(x & 15, z & 15, |v| air.contains(&Some(BlockStateId(v))))
        })
    }
    /// The biome of quart cell `qx, qy, qz` (block coordinates `>> 2`,
    /// `getNoiseBiome`), the `y` clamped to the level, as a biome id of
    /// `native_biome_name` and `native_biome_tint`. None for a cell outside
    /// this chunk.
    pub fn noise_biome(&self, qx: i32, qy: i32, qz: i32) -> Option<u16> {
        let (x, z) = (qx << 2, qz << 2);
        ((x >> 4, z >> 4) == (self.pos[0], self.pos[1])).then(|| self.chunk.biome(x, qy << 2, z))
    }
    /// The quart cell whose biome is the biome at block `x, y, z`
    /// (`BiomeManager.getBiome`'s fuzzy zoom with the world's seed): it can
    /// lie in a neighbouring chunk, so read it with `noise_biome` of that
    /// chunk's snapshot.
    pub fn biome_cell(&self, x: i32, y: i32, z: i32) -> [i32; 3] {
        self.zoom.quart_at_block(BlockPos::new(x, y, z))
    }
    /// The section at section `y` (`SectionPos` y: block `y >> 4`, so -4 is
    /// block `y` -64 to -49 in the overworld) with all its block states and
    /// biomes in one read: the values `block_state` and `noise_biome` give
    /// position by position. None for a section outside this chunk's height.
    pub fn section(&self, section_y: i32) -> Option<NativeSection> {
        let index = self.chunk.section_index(section_y.checked_mul(16)?)?;
        let section = &self.chunk.sections[index];
        let mut blocks = [0u32; 4096];
        section.unpack(&mut blocks);
        let biomes = (0..64).map(|i| section.biomes.get(i) as u16).collect();
        Some(NativeSection {
            blocks: blocks.to_vec(),
            biomes,
        })
    }
}

/// One section of a snapshot's chunk, as `NativeChunkSnapshot::section` gives
/// it: plain arrays in vanilla's index order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSection {
    /// The 4,096 block state ids, index `y << 8 | z << 4 | x` with each
    /// coordinate 0..16 inside the section (`block_state`'s states).
    pub blocks: Vec<u32>,
    /// The 64 biome ids of the section's 4x4x4 quart cells, index
    /// `y << 4 | z << 2 | x` with each coordinate 0..4 (`noise_biome`'s ids).
    pub biomes: Vec<u16>,
}

// Native mods use these; nothing in Horizon does.
impl Game<'_> {
    /// A snapshot of loaded chunk `chunk` (`[x, z]`) of the level `dimension`
    /// (`minecraft:the_nether`), in place or open with nobody in it. None off
    /// a server, for a level not open and for a chunk not loaded; nothing is
    /// loaded or generated for it. API @API:chunk_snapshot@.
    pub(super) fn native_chunk_snapshot(
        &self,
        dimension: &str,
        chunk: [i32; 2],
    ) -> Option<NativeChunkSnapshot> {
        if !self.native_authoritative() {
            return None;
        }
        let world = if dimension == self.dim.id() {
            &self.world
        } else {
            let dim = [Dim::Overworld, Dim::Nether, Dim::End]
                .into_iter()
                .find(|d| d.id() == dimension)?;
            self.levels.parked_world(dim)?
        };
        Some(NativeChunkSnapshot {
            chunk: world.chunks.get(&(chunk[0], chunk[1]))?.clone(),
            pos: chunk,
            min_y: world.shape.min_y,
            top_y: world.shape.max_y(),
            ceiling: world.shape.has_ceiling,
            zoom: self.streaming.biome_zoom,
            air: ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"]
                .map(|name| self.pack.assets.registry.default_state(name)),
        })
    }

    /// The map colour of block state `state` (`BlockState.getMapColor`):
    /// `(id, colour)`, the `MapColor` id and its `col` (0xRRGGBB), `(0, 0)`
    /// for `MapColor.NONE` (air, glass). None for an unknown state. API
    /// @API:chunk_snapshot@.
    pub(super) fn native_block_map_color(&self, state: BlockStateId) -> Option<(u8, u32)> {
        let known = usize::try_from(state.0).is_ok_and(|s| s < self.pack.assets.registry.len());
        if !known {
            return None;
        }
        let id = self.res.facts().get(state).map_color;
        Some((id, MAP_COLORS.get(usize::from(id)).copied().unwrap_or(0)))
    }

    /// The name of biome id `biome` (`minecraft:plains`), as a snapshot's
    /// `noise_biome` gives it. None for an unknown id. API
    /// @API:chunk_snapshot@.
    pub(super) fn native_biome_name(&self, biome: u16) -> Option<&str> {
        let biomes = world_biomes!(self);
        biomes.list.get(usize::from(biome)).map(|b| b.name.as_str())
    }

    /// The tints of biome id `biome` at block column `x, z`, each 0xRRGGBB:
    /// grass (`Biome.getGrassColor`, with the dark forest's and the swamp's
    /// modifiers), foliage, dry foliage and water, after
    /// `native_biome_colors`. The biome's own, not blended over the biome
    /// blend radius as the client draws them. None for an unknown id. API
    /// @API:chunk_snapshot@.
    pub(super) fn native_biome_tint(&self, biome: u16, x: i32, z: i32) -> Option<BiomeColors> {
        let biomes = world_biomes!(self);
        let b = biomes.list.get(usize::from(biome))?;
        Some(BiomeColors {
            grass: biomes.grass(biome, f64::from(x), f64::from(z)),
            foliage: b.foliage,
            dry_foliage: b.dry_foliage,
            water: b.water,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(chunk: Chunk) -> NativeChunkSnapshot {
        NativeChunkSnapshot {
            chunk: Arc::new(chunk),
            pos: [3, -2],
            min_y: -64,
            top_y: 320,
            ceiling: false,
            zoom: BiomeZoom::new(7),
            air: [None; 3],
        }
    }

    /// Chunk (3, -2) with random block states in the bottom, a middle and the
    /// top section, random biomes in every quart cell, and air elsewhere.
    fn filled() -> Chunk {
        let mut chunk = Chunk::empty(3, -2);
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for section_y in [-4, 0, 19] {
            for y in section_y * 16..section_y * 16 + 16 {
                for z in -32..-16 {
                    for x in 48..64 {
                        chunk.set(x, y, z, (next() % 40_000) as u32);
                    }
                }
            }
        }
        // Quart x 12..16, z -8..-4; the y index counts from the column's bottom.
        for qy in 0..96 {
            for qz in -8..-4 {
                for qx in 12..16 {
                    chunk.set_biome_cell(qx, qy, qz, (next() % 67) as u16);
                }
            }
        }
        chunk
    }

    #[test]
    fn section_matches_the_per_position_reads() {
        let snap = snapshot(filled());
        for section_y in -4..20 {
            let section = snap.section(section_y).expect("inside the chunk");
            assert_eq!(section.blocks.len(), 4096);
            assert_eq!(section.biomes.len(), 64);
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let (bx, by, bz) = (48 + x, section_y * 16 + y, -32 + z);
                        let want = snap.block_state(bx, by, bz).unwrap().0;
                        let got = section.blocks[(y << 8 | z << 4 | x) as usize];
                        assert_eq!(got, want, "block {bx} {by} {bz}");
                    }
                }
            }
            for qy in 0..4 {
                for qz in 0..4 {
                    for qx in 0..4 {
                        let want = snap
                            .noise_biome(12 + qx, section_y * 4 + qy, -8 + qz)
                            .unwrap();
                        let got = section.biomes[(qy << 4 | qz << 2 | qx) as usize];
                        assert_eq!(got, want, "biome {qx} {qy} {qz} of section {section_y}");
                    }
                }
            }
        }
    }

    #[test]
    fn section_is_not_a_uniform_copy_and_empty_sections_read_as_air() {
        let snap = snapshot(filled());
        let bottom = snap.section(-4).unwrap();
        let distinct: std::collections::HashSet<u32> = bottom.blocks.iter().copied().collect();
        assert!(distinct.len() > 1000);
        assert!(snap.section(10).unwrap().blocks.iter().all(|&b| b == 0));
    }

    #[test]
    fn section_is_none_outside_the_chunk() {
        let snap = snapshot(filled());
        for section_y in [-5, 20, i32::MIN, i32::MAX, 1 << 28] {
            assert!(snap.section(section_y).is_none(), "section {section_y}");
        }
    }
}
