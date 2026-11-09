//! What server mods send one player (docs/native-mods.md, "Server hooks for
//! plugin ports: per-player feedback"): sounds (`native_play_sound_to`),
//! particles (`native_send_particles_to`), compass targets
//! (`native_set_compass_target`) and client-only blocks (`native_show_block`,
//! `native_show_blocks`). The hub is out of the game while a guest's
//! turn runs, and these need it (registry ids, the player's position, the
//! compass target it keeps), so they wait here and leave at the tick's end,
//! from whichever hook sent them.
use super::{Game, lan::LanHost};
use horizon_client::particles::ParticleOptions;
use horizon_protocol::packets::play::{self, Clientbound as C};
use horizon_world::registry::BlockStateId;

/// One thing a mod sends one player.
pub(super) enum PlayerFx {
    /// At `at`, or where the player is when it leaves (None).
    Sound {
        sound: String,
        source: i32,
        at: Option<[f64; 3]>,
        volume: f32,
        pitch: f32,
    },
    Particles {
        options: ParticleOptions,
        state: Option<BlockStateId>,
        at: [f64; 3],
        spread: [f64; 3],
        speed: f64,
        count: i32,
        force: bool,
    },
    /// The compass target, or None for the world spawn again.
    Compass(Option<play::GlobalPos>),
    /// Blocks shown to the player alone (`native_show_blocks`): a position and a block state id each.
    Blocks(Vec<([i32; 3], u32)>),
}

/// What waits for the hub, by player UUID, in the order the mods sent it.
#[derive(Default)]
pub(super) struct NativePlayerFx {
    queued: Vec<(u128, PlayerFx)>,
}

impl Game<'_> {
    /// Whether a per-player helper takes something for `uuid`: on a server,
    /// for a player who is on; inside a guest's turn, where the other players
    /// are out of reach, for anyone (the flush drops it for a player who
    /// isn't on).
    pub(super) fn player_fx_reaches(&self, uuid: u128) -> bool {
        if self.remote.is_some() {
            return false;
        }
        if self.lan_acting_uuid == Some(uuid) {
            return true;
        }
        match &self.lan {
            Some(host) => host.hub.guests.iter().any(|g| g.placed && g.uuid == uuid),
            None => self.lan_acting.is_some(),
        }
    }

    pub(super) fn queue_native_player_fx(&mut self, uuid: u128, fx: PlayerFx) {
        self.native_player_fx.queued.push((uuid, fx));
    }

    /// What the mods sent single players this tick leaves.
    pub(super) fn flush_native_player_fx(&mut self, host: &mut LanHost) {
        if self.native_player_fx.queued.is_empty() {
            return;
        }
        let spawn = self.world_spawn_position();
        for (uuid, fx) in std::mem::take(&mut self.native_player_fx.queued) {
            let Some((conn, position, horizon)) = host
                .hub
                .guests
                .iter()
                .find(|g| g.placed && g.uuid == uuid)
                .map(|g| (g.conn, g.position, g.brand == "horizon"))
            else {
                continue;
            };
            let packet = match fx {
                PlayerFx::Blocks(blocks) => {
                    host.hub.show_blocks_to(conn, &blocks);
                    continue;
                }
                PlayerFx::Sound {
                    sound,
                    source,
                    at,
                    volume,
                    pitch,
                } => {
                    // As `flush_native_sounds`: an unknown `minecraft:` event is
                    // dropped, a mod's goes by name.
                    if sound.starts_with("minecraft:")
                        && host
                            .hub
                            .registry_id("minecraft:sound_event", &sound)
                            .is_none()
                    {
                        continue;
                    }
                    let seed = host.hub.next_sound_seed();
                    host.hub.sound_packet(
                        &sound,
                        at.unwrap_or(position),
                        volume,
                        pitch,
                        source,
                        seed,
                    )
                }
                PlayerFx::Particles {
                    options,
                    state,
                    at,
                    spread,
                    speed,
                    count,
                    force,
                } => {
                    // A vanilla client can't decode a mod's particle id
                    // (`mod_particle_only_for_horizon`).
                    if options.mod_particle.is_some() && !horizon {
                        continue;
                    }
                    // ServerLevel.sendParticles(player, ...): within 32 blocks
                    // of the centre of the player's block, 512 when forced.
                    let range: f64 = if force { 512.0 } else { 32.0 };
                    let distance: f64 = (0..3)
                        .map(|k| (position[k].floor() + 0.5 - at[k]).powi(2))
                        .sum();
                    if distance >= range * range {
                        continue;
                    }
                    let Some(particle) = Self::hub_particle(&host.hub, &options, state) else {
                        continue;
                    };
                    C::LevelParticles(play::LevelParticles {
                        particle,
                        override_limiter: force,
                        always_show: false,
                        pos: at,
                        dist: spread.map(|d| d as f32),
                        max_speed: [speed as f32; 3],
                        count,
                        randomization: 0,
                    })
                }
                PlayerFx::Compass(target) => {
                    match target {
                        Some(pos) => host.hub.compass_targets.insert(uuid, pos),
                        None => host.hub.compass_targets.remove(&uuid),
                    };
                    host.hub
                        .default_spawn(uuid, spawn, self.level.world_spawn_angle)
                }
            };
            host.hub.send_to(conn, &packet);
        }
    }
}

/// Blocks shown to one player alone (docs/native-mods.md, API @API:show_block@).
// Native mods use these; nothing in Horizon does.
#[allow(dead_code, reason = "native mod API (docs/native-mods.md)")]
impl Game<'_> {
    /// Shows that player the block state `state` at `pos` in their level, and nobody else, as Bukkit's
    /// `Player.sendBlockChange`. The world is not changed: the real block comes back when its chunk or
    /// block next updates, as in vanilla, so a mod re-sends it while it still wants the block shown.
    /// It leaves at the tick's end, so it works from any server hook; true means queued. False on a
    /// client, for a state id that isn't a block state, and for a player who isn't on (inside another
    /// player's hook, such a player just gets nothing). Bundles need "hooks": @API:show_block@.
    pub(super) fn native_show_block(&mut self, player: u128, pos: [i32; 3], state: u32) -> bool {
        self.native_show_blocks(player, &[(pos, state)])
    }

    /// `native_show_block` for several blocks at once: `blocks` is a list of (position, block state id).
    /// They go as one `BlockUpdate` for a section with a single block and one `SectionBlocksUpdate` for
    /// each section with more, sections in order; within a section a later entry for a position wins.
    /// Same answers as `native_show_block`; false too for an empty list. Bundles need "hooks":
    /// @API:show_block@.
    pub(super) fn native_show_blocks(&mut self, player: u128, blocks: &[([i32; 3], u32)]) -> bool {
        if blocks.is_empty() || !self.player_fx_reaches(player) {
            return false;
        }
        if blocks.iter().any(|&(_, state)| {
            self.pack
                .assets
                .registry
                .state(BlockStateId(state))
                .is_none()
        }) {
            return false;
        }
        self.queue_native_player_fx(player, PlayerFx::Blocks(blocks.to_vec()));
        true
    }
}
