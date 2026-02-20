use std::ops::{Add, Div, Mul, Sub, SubAssign};

use derive_more::AddAssign;

use crate::{
    cli::battery::BatteryPowerLimits,
    core::{flow::Flow, working_mode::WorkingMode},
    quantity::{Zero, power::Watts},
};

#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, AddAssign)]
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

    #[cfg(test)]
    pub fn invariant(self) -> T
    where
        T: Add<Output = T> + Sub<Output = T>,
    {
        self.grid.import - self.grid.export + self.battery.export - self.battery.import
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_zero_battery_flow() {
        let initial = EnergyBalance {
            grid: Flow { import: Watts(100.0), export: Watts(50.0) },
            battery: Flow { import: Watts(10.0), export: Watts(20.0) },
        };
        let expected = EnergyBalance {
            grid: Flow {
                // Battery used to power the household at 20W, but now the grid has to take over.
                import: Watts(120.0),
                // Battery used to get 10W worth of free PV power, but now that has to go to the grid.
                export: Watts(60.0),
            },
            battery: Flow::ZERO,
        };
        assert_eq!(initial.invariant(), expected.invariant());
        assert_eq!(initial.with_battery_flow(Flow::ZERO), expected);
    }

    #[test]
    fn with_partial_battery_flow_reduction() {
        let initial = EnergyBalance {
            battery: Flow { import: Watts(50.0), export: Watts(500.0) },
            grid: Flow { import: Watts(100.0), export: Watts(200.0) },
        };
        let expected = EnergyBalance {
            // The battery is exporting 300W less:
            battery: Flow { import: Watts(50.0), export: Watts(200.0) },
            // Hence, we have to import these:
            grid: Flow { import: Watts(400.0), export: Watts(200.0) },
        };
        assert_eq!(initial.invariant(), expected.invariant());
        assert_eq!(initial.with_battery_flow(expected.battery), expected);
    }

    #[test]
    fn battery_import_beyond_grid_export() {
        let initial = EnergyBalance {
            // Battery discharges 50W into the house:
            battery: Flow { import: Watts::ZERO, export: Watts(50.0) },
            // Grid covers the remaining 100W:
            grid: Flow { import: Watts(100.0), export: Watts::ZERO },
        };
        let expected = EnergyBalance {
            // Now we force 300W discharge – 250W more than before:
            battery: Flow { import: Watts::ZERO, export: Watts(300.0) },
            // That's 150W beyond what the grid was importing, so it flips to export:
            grid: Flow { import: Watts::ZERO, export: Watts(150.0) },
        };
        assert_eq!(initial.invariant(), expected.invariant());
        assert_eq!(initial.with_battery_flow(expected.battery), expected);
    }

    #[test]
    fn battery_export_beyond_grid_import() {
        let initial = EnergyBalance {
            battery: Flow::ZERO,
            // Grid has a small export surplus:
            grid: Flow { import: Watts(200.0), export: Watts(100.0) },
        };
        let expected = EnergyBalance {
            // Force 200W charging – that's 100W beyond grid export:
            battery: Flow { import: Watts(200.0), export: Watts::ZERO },
            // Grid export goes to 100 - 200 = -100, normalize flips to extra import:
            grid: Flow { import: Watts(300.0), export: Watts::ZERO },
        };
        assert_eq!(initial.invariant(), expected.invariant());
        assert_eq!(initial.with_battery_flow(expected.battery), expected);
    }
}
