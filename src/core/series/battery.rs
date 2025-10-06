use chrono::TimeDelta;
use itertools::MultiUnzip;
use linfa::{DatasetBase, prelude::*};
use linfa_linear::LinearRegression;
use ndarray::{Array, Array2};

use crate::{
    api::home_assistant::battery::BatteryDifferentials,
    prelude::*,
    quantity::power::Kilowatts,
};

impl<K, T> TryEstimateBatteryParameters<K> for T where T: ?Sized {}

pub trait TryEstimateBatteryParameters<K> {
    /// Estimate the battery parameters from the time series of
    /// residual charge, import and export differentials.
    #[instrument(name = "Estimating the battery parameters…", skip_all, fields(len = self.size_hint().1))]
    fn try_estimate_battery_parameters(self) -> Result<BatteryParameters>
    where
        Self: Iterator<Item = (K, (TimeDelta, BatteryDifferentials<Kilowatts>))> + Sized,
    {
        let (records, targets, weights): (Vec<_>, Vec<_>, Vec<_>) = self
            .map(|(_, (time_delta, differentials))| {
                (
                    [
                        differentials.attributes.total_import.0,
                        -differentials.attributes.total_export.0, // negate as it acts against the charge
                    ],
                    differentials.residual_energy.0,
                    time_delta.as_seconds_f32() / 3600.0, // longer intervals have more weight
                )
            })
            .multiunzip();

        let dataset = DatasetBase::new(Array2::from(records), Array::from(targets))
            .with_weights(Array::from(weights));
        let model = LinearRegression::default().fit(&dataset)?;

        let parameters = BatteryParameters {
            // The free term is the parasitic load and should be negative as it always discharges:
            parasitic_load: Kilowatts::from(-model.intercept()),
            charge_coefficient: model.params()[0],
            discharge_coefficient: model.params()[1],
        };
        ensure!(
            parameters.parasitic_load > Kilowatts::ZERO,
            "non-positive parasitic load is impossible ({})",
            parameters.parasitic_load,
        );
        ensure!(parameters.charge_coefficient < 1.0, "the charging efficiency must be under 100%");
        ensure!(
            parameters.discharge_coefficient > 1.0,
            "the discharging efficiency must be under 100%",
        );

        info!(
            "Done",
            parasitic_load = parameters.parasitic_load,
            charge_efficiency = format!("{:.1}%", 100.0 * parameters.charge_coefficient),
            discharge_efficiency = format!("{:.1}%", 100.0 / parameters.discharge_coefficient),
            round_trip = format!("{:.1}%", 100.0 * parameters.round_trip()),
        );
        Ok(parameters)
    }
}

#[must_use]
#[derive(Copy, Clone)]
pub struct BatteryParameters {
    /// Conversion coefficient of external to internal power while charging.
    ///
    /// It should normally be lower than 1, meaning the battery needs to consume more than 1 kWH
    /// to increase its residual charge by 1 kWh.
    pub charge_coefficient: f64,

    /// Conversion coefficient of internal to external power while discharging.
    ///
    /// It should normally be greater than 1, meaning the battery needs to spend more than 1 kWH
    /// of its residual charge to produce 1 kWh of energy.
    pub discharge_coefficient: f64,

    /// Always active parasitic power – for example from the [BMS][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Battery_management_system
    pub parasitic_load: Kilowatts,
}

impl Default for BatteryParameters {
    /// Get some reasonable defaults for when the training data is not yet enough.
    fn default() -> Self {
        Self {
            charge_coefficient: 0.95,
            discharge_coefficient: 0.95,
            parasitic_load: Kilowatts::from(0.02),
        }
    }
}

impl BatteryParameters {
    /// Get the round-trip efficiency – the energy production compared to the consumption.
    fn round_trip(&self) -> f64 {
        self.charge_coefficient / self.discharge_coefficient
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;
    use crate::api::home_assistant::battery::BatteryStateAttributes;

    #[test]
    fn test_try_estimate_battery_parameters_ok() -> Result {
        let series = vec![
            (
                1,
                (
                    TimeDelta::hours(1),
                    BatteryDifferentials {
                        residual_energy: Kilowatts::from(0.9),
                        attributes: BatteryStateAttributes {
                            total_import: Kilowatts::from(1.0),
                            total_export: Kilowatts::from(0.0),
                        },
                    },
                ),
            ),
            (
                2,
                (
                    TimeDelta::hours(1),
                    BatteryDifferentials {
                        residual_energy: Kilowatts::from(-1.3),
                        attributes: BatteryStateAttributes {
                            total_import: Kilowatts::from(0.0),
                            total_export: Kilowatts::from(1.0),
                        },
                    },
                ),
            ),
            (
                3,
                (
                    TimeDelta::hours(1),
                    BatteryDifferentials {
                        residual_energy: Kilowatts::from(-0.05),
                        attributes: BatteryStateAttributes {
                            total_import: Kilowatts::from(0.0),
                            total_export: Kilowatts::from(0.0),
                        },
                    },
                ),
            ),
        ];
        let parameters = series.into_iter().try_estimate_battery_parameters()?;
        assert_abs_diff_eq!(parameters.parasitic_load.0, 0.05);
        assert_abs_diff_eq!(parameters.charge_coefficient, 0.95);
        assert_abs_diff_eq!(parameters.discharge_coefficient, 1.25);
        Ok(())
    }
}
