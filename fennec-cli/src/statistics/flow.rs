use std::ops::{Div, Mul};

use derive_more::{Add, AddAssign, Sub};

use crate::{
    cli::battery::BatteryPowerLimits,
    core::working_mode::WorkingMode,
    quantity::{energy::WattHours, power::Watts},
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

impl Flow<Watts> {
    pub const fn zero() -> Self {
        Self { import: Watts::zero(), export: Watts::zero() }
    }

    pub fn normalize(&mut self) {
        if self.import < Watts::zero() {
            self.export -= self.import;
            self.import = Watts::zero();
        }
        if self.export < Watts::zero() {
            self.import -= self.export;
            self.export = Watts::zero();
        }
    }
}

impl Default for Flow<WattHours> {
    fn default() -> Self {
        Self { import: WattHours::zero(), export: WattHours::zero() }
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

impl<T: Mul<Rhs>, Rhs: Copy> Mul<Rhs> for Flow<T> {
    type Output = Flow<<T as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Flow { import: self.import * rhs, export: self.export * rhs }
    }
}

impl<T: Div<Rhs>, Rhs: Copy> Div<Rhs> for Flow<T> {
    type Output = Flow<<T as Div<Rhs>>::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        Flow { import: self.import / rhs, export: self.export / rhs }
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

impl SystemFlow<Watts> {
    /// Re-distribute the power flow based on the working mode.
    pub fn with_working_mode(
        mut self,
        working_mode: WorkingMode,
        battery_power_limits: BatteryPowerLimits,
    ) -> Self {
        match working_mode {
            WorkingMode::Idle => {
                self.grid += self.battery.reversed();
                self.battery = Flow::zero();
            }
            WorkingMode::Harvest => {
                self.grid.import += self.battery.export;
                self.battery.export = Watts::zero();
            }
            WorkingMode::SelfUse => {
                // Nothing changes.
            }
            WorkingMode::Charge => {
                self.grid.import +=
                    battery_power_limits.charging + (self.battery.export - self.battery.import);
                self.grid.normalize();
                self.battery.import = battery_power_limits.charging;
                self.battery.export = Watts::zero();
            }
            WorkingMode::Discharge => {
                self.grid.export +=
                    battery_power_limits.discharging + (self.battery.import - self.battery.export);
                self.grid.normalize();
                self.battery.export = battery_power_limits.discharging;
                self.battery.import = Watts::zero();
            }
        }
        self
    }
}

impl<T: Mul<Rhs>, Rhs: Copy> Mul<Rhs> for SystemFlow<T> {
    type Output = SystemFlow<<T as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        SystemFlow { grid: self.grid * rhs, battery: self.battery * rhs }
    }
}

impl<T: Div<Rhs>, Rhs: Copy> Div<Rhs> for SystemFlow<T> {
    type Output = SystemFlow<<T as Div<Rhs>>::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        SystemFlow { grid: self.grid / rhs, battery: self.battery / rhs }
    }
}
