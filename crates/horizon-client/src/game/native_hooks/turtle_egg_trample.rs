use super::*;

impl Game<'_> {
    /// Turtle-egg trample: vanilla `TurtleEggBlock.canDestroyEgg` (stepOn, fallOn). `canceled` starts false.
    pub(in crate::game) fn native_turtle_egg_trample(
        &mut self,
        call: super::fire::TurtleEggTrampleCall<'_>,
    ) {
        self.fire_turtle_egg_trample(call);
    }
}
