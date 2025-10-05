use rs_stats::regression::multiple_linear_regression::MultipleLinearRegression;

use crate::{api::home_assistant::battery::BatteryState, prelude::*, quantity::power::Kilowatts};

impl<K, T> TryEstimateBatteryParameters<K> for T where T: ?Sized {}

pub trait TryEstimateBatteryParameters<K> {
    /// Estimate the battery parameters from the time series of
    /// residual charge, import and export differentials.
    #[instrument(name = "Estimating the battery parameters…", skip_all)]
    fn try_estimate_battery_parameters(self) -> Result<BatteryParameters>
    where
        Self: Iterator<Item = (K, BatteryState<Kilowatts>)> + Sized,
    {
        let (xs, ys): (Vec<_>, Vec<_>) = self
            .map(|(_, state)| {
                (
                    // Negate the export since it discharges the battery:
                    vec![state.attributes.total_import.0, -state.attributes.total_export.0],
                    state.residual_energy.0,
                )
            })
            .unzip();

        let mut model = MultipleLinearRegression::<f64>::new();

        if let Err(message) = model.fit(&xs, &ys) {
            bail!("{message}");
        }

        let parameters = BatteryParameters {
            // The free term is the parasitic load and should be negative as it always discharges:
            parasitic_load: Kilowatts::from(-model.coefficients[0]),
            charge_coefficient: model.coefficients[1],
            discharge_coefficient: model.coefficients[2],
        };
        ensure!(
            parameters.parasitic_load > Kilowatts::ZERO,
            "non-positive parasitic load is impossible ({})",
            parameters.parasitic_load,
        );
        ensure!(parameters.charge_coefficient < parameters.discharge_coefficient);

        info!(
            "Done",
            parasitic_load = parameters.parasitic_load,
            charge_coefficient = format!("{:.3}", parameters.charge_coefficient),
            discharge_coefficient = format!("{:.3}", parameters.discharge_coefficient),
            round_trip =
                format!("{:.2}", parameters.charge_coefficient / parameters.discharge_coefficient),
            r_squared = format!("{:.2}", model.r_squared),
            adjusted_r_squared = format!("{:.2}", model.adjusted_r_squared),
        );
        Ok(parameters)
    }
}

#[must_use]
#[derive(Copy, Clone)]
pub struct BatteryParameters {
    /// Conversion coefficient of external power to internal power while charging.
    pub charge_coefficient: f64,

    /// Conversion coefficient of external power to internal power while discharging.
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
                BatteryState {
                    residual_energy: Kilowatts::from(0.9),
                    attributes: BatteryStateAttributes {
                        total_import: Kilowatts::from(1.0),
                        total_export: Kilowatts::from(0.0),
                    },
                },
            ),
            (
                2,
                BatteryState {
                    residual_energy: Kilowatts::from(-1.3),
                    attributes: BatteryStateAttributes {
                        total_import: Kilowatts::from(0.0),
                        total_export: Kilowatts::from(1.0),
                    },
                },
            ),
            (
                3,
                BatteryState {
                    residual_energy: Kilowatts::from(-0.05),
                    attributes: BatteryStateAttributes {
                        total_import: Kilowatts::from(0.0),
                        total_export: Kilowatts::from(0.0),
                    },
                },
            ),
        ];
        let parameters = series.into_iter().try_estimate_battery_parameters()?;
        assert_abs_diff_eq!(parameters.parasitic_load.0, 0.05);
        assert_abs_diff_eq!(parameters.charge_coefficient, 0.95);
        assert_abs_diff_eq!(parameters.discharge_coefficient, 1.25);
        Ok(())
    }
}
