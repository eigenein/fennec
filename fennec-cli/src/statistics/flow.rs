use std::ops::{Div, Mul};

use chrono::TimeDelta;
use derive_more::{Add, AddAssign, Sub};

use crate::{
    cli::battery::BatteryPowerLimits,
    core::working_mode::WorkingMode,
    quantity::{Quantity, energy::KilowattHours, power::Kilowatts},
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

impl Default for Flow<KilowattHours> {
    fn default() -> Self {
        Self { import: KilowattHours::ZERO, export: KilowattHours::ZERO }
    }
}

impl<T: Copy> Flow<T> {
    /// Get the reversed flow where the import becomes export and vice versa.
    ///
    /// This is used to off-load unserved battery flow onto the grid:
    ///
    /// - Unserved charge becomes grid export
    /// - Unserved discharge becomes grid import
    pub const fn reversed(&self) -> Self {
        Self { import: self.export, export: self.import }
    }
}

impl Mul<TimeDelta> for Flow<Kilowatts> {
    type Output = Flow<KilowattHours>;

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

impl SystemFlow<KilowattHours> {
    /// Split the net household deficit into grid and battery energy flows based on the battery working mode.
    pub fn new(
        battery_power_limits: BatteryPowerLimits,
        working_mode: WorkingMode,
        time_delta: TimeDelta,
        net_deficit: KilowattHours,
    ) -> Self {
        let battery_net_import = match working_mode {
            WorkingMode::Idle => Quantity::ZERO,
            WorkingMode::Harvest => {
                (-net_deficit).clamp(Quantity::ZERO, battery_power_limits.charging * time_delta)
            }
            WorkingMode::SelfUse => (-net_deficit).clamp(
                -battery_power_limits.discharging * time_delta,
                battery_power_limits.charging * time_delta,
            ),
            WorkingMode::Charge => battery_power_limits.charging * time_delta,
            WorkingMode::Discharge => -battery_power_limits.discharging * time_delta,
        };
        let grid_net_import = net_deficit + battery_net_import;
        Self {
            grid: Flow {
                import: grid_net_import.max(Quantity::ZERO),
                export: (-grid_net_import).max(Quantity::ZERO),
            },
            battery: Flow {
                import: battery_net_import.max(Quantity::ZERO),
                export: (-battery_net_import).max(Quantity::ZERO),
            },
        }
    }
}

impl Div<TimeDelta> for SystemFlow<KilowattHours> {
    type Output = SystemFlow<Kilowatts>;

    fn div(self, time_delta: TimeDelta) -> Self::Output {
        SystemFlow { grid: self.grid / time_delta, battery: self.battery / time_delta }
    }
}
