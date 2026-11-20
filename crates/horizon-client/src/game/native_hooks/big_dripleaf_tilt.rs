use super::*;

impl Game<'_> {
    /// Big dripleaf tilt: vanilla `BigDripleafBlock.canEntityTilt` gates the tilt in `entityInside`.
    pub(in crate::game) fn native_big_dripleaf_tilt(&mut self, dimension: &str, pos: [i32; 3], entity: EntityRef, canceled: &mut bool) {
        self.fire_big_dripleaf_tilt(dimension, pos, entity, canceled);
    }
}
