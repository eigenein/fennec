use std::ops::{AddAssign, Div};

use chrono::TimeDelta;
use derive_more::{AddAssign, Div};

use crate::{
    cli::battery::BatteryPowerLimits,
    quantity::{Quantity, energy::KilowattHours, power::Kilowatts},
};

/// Generic bidirectional energy flow.
#[must_use]
pub struct Flow<T> {
    import: T,
    export: T,
}

impl AddAssign for Flow<KilowattHours> {
    fn add_assign(&mut self, rhs: Self) {
        self.import += rhs.import;
        self.export += rhs.export;
    }
}

impl Div<TimeDelta> for Flow<KilowattHours> {
    type Output = Flow<Kilowatts>;

    fn div(self, time_delta: TimeDelta) -> Self::Output {
        Flow { import: self.import / time_delta, export: self.export / time_delta }
    }
}

#[must_use]
#[derive(AddAssign, Div)]
pub struct SystemFlow<T> {
    grid: Flow<T>,
    battery: Flow<T>,
}

impl SystemFlow<KilowattHours> {
    pub fn new(
        battery_power_limits: BatteryPowerLimits,
        time_delta: TimeDelta,
        net_deficit: KilowattHours,
    ) -> Self {
        let battery_net_import = (-net_deficit).clamp(
            -battery_power_limits.discharging * time_delta,
            battery_power_limits.charging * time_delta,
        );
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
