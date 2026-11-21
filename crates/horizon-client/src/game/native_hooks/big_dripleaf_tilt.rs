use super::*;

impl Game<'_> {
    /// Big dripleaf tilt: vanilla `BigDripleafBlock.canEntityTilt` gates the tilt in `entityInside`.
    pub(in crate::game) fn native_big_dripleaf_tilt(
        &mut self,
        call: mod_api::__private::BigDripleafTiltCall<'_>,
    ) {
        self.fire_big_dripleaf_tilt(call);
    }
}
