use crate::units::{KilowattHourRate, PowerDensity};

#[derive(Copy, Clone)]
pub struct Forecast {
    pub grid_rate: KilowattHourRate,
    pub solar_power_density: PowerDensity,
}
