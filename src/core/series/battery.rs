use chrono::TimeDelta;
use itertools::MultiUnzip;
use linfa::{DatasetBase, traits::Fit};
use linfa_linear::{FittedLinearRegression, LinearRegression};
use ndarray::{Array, Array2};

use crate::{
    api::home_assistant::battery::BatteryState,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

impl<T> TryEstimateBatteryParameters for T where T: ?Sized {}

pub trait TryEstimateBatteryParameters {
    /// Estimate the battery parameters from the time series of
    /// residual charge, import and export deltas.
    #[instrument(name = "Estimating the battery parameters…", skip_all, fields(len = self.size_hint().1))]
    fn try_estimate_battery_parameters(self) -> Result<BatteryParameters>
    where
        Self: Iterator<Item = (BatteryState<KilowattHours>, TimeDelta)> + Sized,
    {
        let (records, targets, weights): (Vec<_>, Vec<_>, Vec<_>) = self
            .map(|(energy_delta, time_delta)| {
                (
                    [
                        (energy_delta.attributes.total_import / time_delta).0,
                        // Negate the export as it acts against the charge:
                        -(energy_delta.attributes.total_export / time_delta).0,
                    ],
                    (energy_delta.residual_energy / time_delta).0,
                    // Assign weight according to the time delta:
                    time_delta.as_seconds_f32() / 3600.0,
                )
            })
            .multiunzip();

        info!("Regression analysis…", len = records.len());
        let dataset = DatasetBase::new(Array2::from(records), Array::from(targets))
            .with_weights(Array::from(weights));
        let model = LinearRegression::default()
            .fit(&dataset)
            .context("could not build a linear regression")?;
        let parameters = BatteryParameters::try_from(&model)
            .context("estimated parameters do not make sense")?;

        info!(
            "Done",
            parasitic_power = parameters.parasitic_power,
            charging_efficiency = format!("{:.1}%", 100.0 * parameters.charging_coefficient),
            discharging_efficiency = format!("{:.1}%", 100.0 / parameters.discharging_coefficient),
            round_trip = format!("{:.1}%", 100.0 * parameters.round_trip()),
        );
        if parameters.parasitic_power > Kilowatts::ZERO {
            warn!("Positive parasitic power is not real");
        }
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
    pub charging_coefficient: f64,

    /// Conversion coefficient of internal to external power while discharging.
    ///
    /// It should normally be greater than 1, meaning the battery needs to spend more than 1 kWH
    /// of its residual charge to produce 1 kWh of energy.
    pub discharging_coefficient: f64,

    /// Always active parasitic power – for example from the [BMS][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Battery_management_system
    pub parasitic_power: Kilowatts,
}

impl Default for BatteryParameters {
    /// Get some reasonable defaults for when the training data is not yet enough.
    fn default() -> Self {
        Self {
            charging_coefficient: 0.95,
            discharging_coefficient: 0.95,
            parasitic_power: Kilowatts::from(-0.02),
        }
    }
}

impl TryFrom<&FittedLinearRegression<f64>> for BatteryParameters {
    type Error = Error;

    fn try_from(model: &FittedLinearRegression<f64>) -> Result<Self> {
        let this = Self {
            parasitic_power: Kilowatts::from(model.intercept()),
            charging_coefficient: model.params()[0],
            discharging_coefficient: model.params()[1],
        };
        ensure!(this.parasitic_power.0.is_finite());
        ensure!(this.charging_coefficient.is_finite());
        ensure!(this.charging_coefficient <= 1.5);
        ensure!(this.charging_coefficient >= 0.5);
        ensure!(this.discharging_coefficient.is_finite());
        ensure!(this.discharging_coefficient <= 1.5);
        ensure!(this.discharging_coefficient >= 0.5);
        ensure!(this.discharging_coefficient > this.charging_coefficient);
        Ok(this)
    }
}

impl BatteryParameters {
    /// Get the round-trip efficiency – the energy production compared to the consumption.
    fn round_trip(&self) -> f64 {
        self.charging_coefficient / self.discharging_coefficient
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
                BatteryState {
                    residual_energy: KilowattHours::from(0.9),
                    attributes: BatteryStateAttributes {
                        total_import: KilowattHours::from(1.0),
                        total_export: KilowattHours::from(0.0),
                    },
                },
                TimeDelta::hours(1),
            ),
            (
                BatteryState {
                    residual_energy: KilowattHours::from(-1.3),
                    attributes: BatteryStateAttributes {
                        total_import: KilowattHours::from(0.0),
                        total_export: KilowattHours::from(1.0),
                    },
                },
                TimeDelta::hours(1),
            ),
            (
                BatteryState {
                    residual_energy: KilowattHours::from(-0.05),
                    attributes: BatteryStateAttributes {
                        total_import: KilowattHours::from(0.0),
                        total_export: KilowattHours::from(0.0),
                    },
                },
                TimeDelta::hours(1),
            ),
        ];
        let parameters = series.into_iter().try_estimate_battery_parameters()?;
        assert_abs_diff_eq!(parameters.parasitic_power.0, -0.05);
        assert_abs_diff_eq!(parameters.charging_coefficient, 0.95);
        assert_abs_diff_eq!(parameters.discharging_coefficient, 1.25);
        Ok(())
    }
}
