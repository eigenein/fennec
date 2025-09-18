use crate::units::{power_density::PowerDensity, rate::KilowattHourRate};

pub struct Metrics {
    pub grid_rate: KilowattHourRate,
    pub solar_power_density: Option<PowerDensity>,
}
