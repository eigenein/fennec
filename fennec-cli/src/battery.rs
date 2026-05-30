pub mod efficiency;
mod simulator;
mod working_mode;

pub use self::{simulator::Simulator, working_mode::WorkingMode};
#[cfg(test)]
use crate::quantity::Zero;
use crate::quantity::power::Watts;

#[must_use]
#[derive(Copy, Clone)]
pub struct Efficiency {
    pub charging: f64,
    pub discharging: f64,
    pub parasitic_load: Watts,
}

impl Efficiency {
    #[cfg(test)]
    pub const IDEAL: Self = Self { charging: 1.0, discharging: 1.0, parasitic_load: Zero::ZERO };

    pub const fn round_trip(self) -> f64 {
        self.charging * self.discharging
    }
}
