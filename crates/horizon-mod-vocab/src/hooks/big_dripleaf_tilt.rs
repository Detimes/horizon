//! The `big_dripleaf_tilt` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A big dripleaf at `pos` in `dimension` is about to tilt under `entity`: vanilla
    /// `BigDripleafBlock.canEntityTilt` answered yes. Set `canceled` (or return `Flow::Cancel`)
    /// to keep the leaf upright: no tilt, no tilt sound and no scheduled tilt-reset tick.
    /// Server-side. Beyond NeoForge: none.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "canceled",
        neoforge = "none (NeoForge has no such event)"
    )]
    fn big_dripleaf_tilt(dimension: &str, pos: [i32; 3], entity: EntityRef, canceled: &mut bool);
}
