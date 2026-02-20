use std::ops::{Add, Div, Mul, Sub, SubAssign};

use derive_more::AddAssign;

use crate::{
    cli::battery::BatteryPowerLimits,
    core::{flow::Flow, working_mode::WorkingMode},
    quantity::{Zero, power::Watts},
};

#[must_use]
#[derive(Copy, Clone, AddAssign)]
pub struct EnergyBalance<T> {
    pub grid: Flow<T>,
    pub battery: Flow<T>,
}

impl<T: Zero> Zero for EnergyBalance<T> {
    const ZERO: Self = Self { grid: Flow::ZERO, battery: Flow::ZERO };
}

impl EnergyBalance<Watts> {
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
                import: grid_net_import.max(Watts::ZERO),
                export: (-grid_net_import).max(Watts::ZERO),
            },
            battery: Flow {
                import: battery_net_import.max(Watts::ZERO),
                export: (-battery_net_import).max(Watts::ZERO),
            },
        }
    }

    /// Re-distribute the power flow based on the working mode.
    pub fn with_working_mode(self, working_mode: WorkingMode, limits: BatteryPowerLimits) -> Self {
        self.with_battery_flow(match working_mode {
            WorkingMode::Idle => Flow::ZERO,
            WorkingMode::Harvest => Flow { import: self.battery.import, export: Watts::ZERO },
            WorkingMode::SelfUse => self.battery,
            WorkingMode::Charge => Flow { import: limits.charging, export: Watts::ZERO },
            WorkingMode::Discharge => Flow { import: Watts::ZERO, export: limits.discharging },
        })
    }
}

impl<T> EnergyBalance<T> {
    /// Change the battery flow and re-balance the resulting grid flow.
    fn with_battery_flow(mut self, battery_flow: Flow<T>) -> Self
    where
        T: Copy + Zero + PartialOrd + SubAssign,
        Flow<T>: Add<Output = Flow<T>> + Sub<Output = Flow<T>>,
    {
        self.grid = (self.grid + (self.battery - battery_flow).reversed()).normalized();
        self.battery = battery_flow;
        self
    }
}

impl<T: Mul<Rhs>, Rhs: Copy> Mul<Rhs> for EnergyBalance<T> {
    type Output = EnergyBalance<<T as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        EnergyBalance { grid: self.grid * rhs, battery: self.battery * rhs }
    }
}

impl<T: Div<Rhs>, Rhs: Copy> Div<Rhs> for EnergyBalance<T> {
    type Output = EnergyBalance<<T as Div<Rhs>>::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        EnergyBalance { grid: self.grid / rhs, battery: self.battery / rhs }
    }
}
