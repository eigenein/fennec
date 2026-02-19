use std::ops::{Div, Mul};

use chrono::TimeDelta;
use derive_more::{Add, AddAssign, Sub};

use crate::{
    cli::battery::BatteryPowerLimits,
    core::working_mode::WorkingMode,
    quantity::{
        Quantity,
        energy::KilowattHours,
        power::{Kilowatts, Watts},
    },
};

/// Generic bidirectional energy flow.
#[must_use]
#[derive(Copy, Clone, Add, Sub, AddAssign)]
pub struct Flow<T> {
    /// Importing from grid or charging the battery.
    pub import: T,

    /// Exporting to the grid or discharging the battery.
    pub export: T,
}

impl From<BatteryPowerLimits> for Flow<Kilowatts> {
    fn from(limits: BatteryPowerLimits) -> Self {
        Self { import: limits.charging.into(), export: limits.discharging.into() }
    }
}

impl Default for Flow<Kilowatts> {
    fn default() -> Self {
        Self { import: Quantity::ZERO, export: Quantity::ZERO }
    }
}

impl Default for Flow<KilowattHours> {
    fn default() -> Self {
        Self { import: Quantity::ZERO, export: Quantity::ZERO }
    }
}

impl<T> Flow<T> {
    /// Get the reversed flow where the import becomes export and vice versa.
    ///
    /// This is used to off-load unserved battery flow onto the grid:
    ///
    /// - Unserved charge becomes grid export
    /// - Unserved discharge becomes grid import
    pub const fn reversed(&self) -> Self
    where
        T: Copy,
    {
        Self { import: self.export, export: self.import }
    }
}

impl<T: Mul<TimeDelta>> Mul<TimeDelta> for Flow<T> {
    type Output = Flow<<T as Mul<TimeDelta>>::Output>;

    fn mul(self, time_delta: TimeDelta) -> Self::Output {
        Flow { import: self.import * time_delta, export: self.export * time_delta }
    }
}

impl Div<TimeDelta> for Flow<KilowattHours> {
    type Output = Flow<Kilowatts>;

    fn div(self, time_delta: TimeDelta) -> Self::Output {
        Flow { import: self.import / time_delta, export: self.export / time_delta }
    }
}

#[must_use]
#[derive(Copy, Clone, AddAssign)]
pub struct SystemFlow<T> {
    pub grid: Flow<T>,
    pub battery: Flow<T>,
}

impl<T> Default for SystemFlow<T>
where
    Flow<T>: Default,
{
    fn default() -> Self {
        Self { grid: Flow::default(), battery: Flow::default() }
    }
}

impl SystemFlow<Watts> {
    /// Split the net household deficit into grid and battery energy flows.
    ///
    /// This allows to track not just the net deficit, but also how much the battery can actually
    /// compensate or absorb.
    pub fn new(battery_power_limits: BatteryPowerLimits, net_power: Watts) -> Self {
        let battery_net_import =
            (-net_power).clamp(-battery_power_limits.discharging, battery_power_limits.charging);
        let grid_net_import = net_power + battery_net_import;
        Self {
            grid: Flow {
                import: grid_net_import.max(Watts::zero()),
                export: (-grid_net_import).max(Watts::zero()),
            },
            battery: Flow {
                import: battery_net_import.max(Watts::zero()),
                export: (-battery_net_import).max(Watts::zero()),
            },
        }
    }
}

impl SystemFlow<Kilowatts> {
    /// Re-distribute the power flow based on the working mode.
    pub fn with_working_mode(
        mut self,
        working_mode: WorkingMode,
        battery_power_limits: Flow<Kilowatts>,
    ) -> Self {
        match working_mode {
            WorkingMode::Idle => {
                self.grid += self.battery.reversed();
                self.battery = Flow::default();
            }
            WorkingMode::Harvest => {
                self.grid.import += self.battery.export;
                self.battery.export = Quantity::ZERO;
            }
            WorkingMode::SelfUse => {
                // Nothing changes.
            }
            WorkingMode::Charge => {
                self.grid.import += battery_power_limits.import - self.battery.import;
                self.battery.import = battery_power_limits.import;
            }
            WorkingMode::Discharge => {
                self.grid.export += battery_power_limits.export - self.battery.export;
                self.battery.export = battery_power_limits.export;
            }
        }
        self
    }
}

impl<T: Mul<TimeDelta>> Mul<TimeDelta> for SystemFlow<T> {
    type Output = SystemFlow<<T as Mul<TimeDelta>>::Output>;

    fn mul(self, time_delta: TimeDelta) -> Self::Output {
        SystemFlow { grid: self.grid * time_delta, battery: self.battery * time_delta }
    }
}

impl Div<TimeDelta> for SystemFlow<KilowattHours> {
    type Output = SystemFlow<Kilowatts>;

    fn div(self, time_delta: TimeDelta) -> Self::Output {
        SystemFlow { grid: self.grid / time_delta, battery: self.battery / time_delta }
    }
}
