use rs_stats::regression::multiple_linear_regression::MultipleLinearRegression;

use crate::{
    api::home_assistant::energy::EnergyState,
    core::series::Series,
    prelude::*,
    quantity::power::Kilowatts,
};

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

impl Series<EnergyState<Kilowatts>> {
    /// Estimate the battery parameters from the time series of
    /// residual charge, import and export differentials.
    #[instrument(
        name = "Estimating the battery parameters…",
        skip_all,
        fields(n_points = self.len()),
    )]
    #[deprecated = "implement as a trait"]
    pub fn try_estimate_battery_parameters(&self) -> Result<BatteryParameters> {
        let (xs, ys): (Vec<_>, Vec<_>) = self
            .iter()
            .map(|(_, state)| {
                (
                    // Negate the export since it discharges the battery:
                    vec![state.battery.total_import.0, -state.battery.total_export.0],
                    state.battery.residual_energy.0,
                )
            })
            .unzip();

        let mut model = MultipleLinearRegression::<f64>::new();
        if let Err(message) = model.fit(&xs, &ys) {
            bail!(message);
        }

        let parameters = BatteryParameters {
            // The free term is the parasitic load and should be negative as it always discharges:
            parasitic_load: Kilowatts::from(-model.coefficients[0]),
            charge_coefficient: model.coefficients[1],
            discharge_coefficient: model.coefficients[2],
        };
        ensure!(parameters.parasitic_load > Kilowatts::ZERO);
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
