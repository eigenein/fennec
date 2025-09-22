use crate::units::{power_density::PowerDensity, rate::KilowattHourRate};

#[derive(Copy, Clone)]
pub struct Metrics {
    pub grid_rate: KilowattHourRate,
    pub solar_power_density: Option<PowerDensity>,
}
