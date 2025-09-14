use crate::{
    prelude::*,
    strategy::Point,
    units::{KilowattHourRate, PowerDensity},
};

pub struct Metrics {
    pub grid_rate: KilowattHourRate,
    pub solar_power_density: PowerDensity,
}

impl Point<Metrics> {
    pub fn try_from(zip: (Point<KilowattHourRate>, Point<PowerDensity>)) -> Result<Self> {
        let (grid_rate_point, solar_power_density_point) = zip;
        ensure!(grid_rate_point.time == solar_power_density_point.time);
        Ok(Self {
            time: grid_rate_point.time,
            value: Metrics {
                grid_rate: grid_rate_point.value,
                solar_power_density: solar_power_density_point.value,
            },
        })
    }
}
