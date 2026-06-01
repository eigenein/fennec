pub mod efficiency;
mod simulator;
mod working_mode;

pub use self::{simulator::Simulator, working_mode::WorkingMode};

#[must_use]
#[derive(Copy, Clone)]
pub struct Efficiency {
    pub charging: f64,
    pub discharging: f64,
}

impl Efficiency {
    #[cfg(test)]
    pub const IDEAL: Self = Self { charging: 1.0, discharging: 1.0 };

    pub const fn round_trip(self) -> f64 {
        self.charging * self.discharging
    }
}
