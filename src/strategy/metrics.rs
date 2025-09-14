use crate::{
    strategy::Point,
    units::{KilowattHourRate, PowerDensity},
};

pub struct Metrics {
    pub grid_rate: KilowattHourRate,
    pub solar_power_density: PowerDensity,
}

impl From<(Point<KilowattHourRate>, Point<PowerDensity>)> for Point<Metrics> {
    fn from(pair: (Point<KilowattHourRate>, Point<PowerDensity>)) -> Self {
        let (grid_rate_point, solar_power_density_point) = pair;
        assert_eq!(grid_rate_point.time, solar_power_density_point.time);
        Self {
            time: grid_rate_point.time,
            value: Metrics {
                grid_rate: grid_rate_point.value,
                solar_power_density: solar_power_density_point.value,
            },
        }
    }
}
