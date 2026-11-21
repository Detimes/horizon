use super::*;

impl Game<'_> {
    /// Big dripleaf tilt: vanilla `BigDripleafBlock.canEntityTilt` gates the tilt in `entityInside`.
    pub(in crate::game) fn native_big_dripleaf_tilt(
        &mut self,
        call: super::fire::BigDripleafTiltCall<'_>,
    ) {
        self.fire_big_dripleaf_tilt(call);
    }
}
