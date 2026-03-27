use chrono::{DateTime, Days, Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;

use crate::{
    api::foxcloud,
    battery::WorkingMode,
    cli::{battery::BatteryArgs, db::DbArgs, foxcloud::FoxCloudApiArgs},
    db::power,
    energy,
    fmt::tables::build_steps_table,
    ops::Interval,
    prelude::*,
    quantity::{Quantum, energy::WattHours, price::KilowattHourPrice},
    solution::Solver,
};

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push schedules to FoxESS Cloud – only perform dry runs.
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    scout: bool,

    #[clap(long = "energy-provider", env = "ENERGY_PROVIDER")]
    energy_provider: energy::Provider,

    #[clap(
        long = "working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "idle,harness,charge,compensate",
    )]
    working_modes: Vec<WorkingMode>,

    #[clap(long = "quantum-watthours", env = "QUANTUM_WATTHOURS", default_value = "1")]
    quantum: WattHours,

    #[clap(flatten)]
    battery: BatteryArgs,

    #[clap(flatten)]
    fox_ess_api: FoxCloudApiArgs,

    #[clap(flatten)]
    db: DbArgs,
}

impl HuntArgs {
    #[must_use]
    pub fn working_modes(&self) -> EnumSet<WorkingMode> {
        self.working_modes.iter().copied().collect()
    }
}

impl HuntArgs {
    #[instrument(skip_all)]
    pub async fn run(self) -> Result {
        let db = self.db.connect().await?;
        let fox_ess = foxcloud::Api::new(self.fox_ess_api.api_key.clone())?;
        let working_modes = self.working_modes();
        let now = Local::now().with_nanosecond(0).unwrap();
        let energy_prices = self.get_prices(now).await?;

        let battery_state = self.battery.connection.connect().await?.read_full_state().await?;
        println!("{battery_state}");

        let balance_profile = {
            let power_logs = db.measurements::<power::Measurement>().await?;
            energy::BalanceProfile::try_estimate(
                self.battery.power_limits,
                self.energy_provider.time_step(),
                power_logs,
            )
            .await?
        };
        db.shutdown().await;

        let initial_energy_level = self.quantum.index(battery_state.energy.residual()).unwrap();
        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .balance_profile(&balance_profile)
            .working_modes(working_modes)
            .min_residual_energy(battery_state.min_residual_energy())
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                battery_state.max_residual_energy().max(battery_state.energy.residual()),
            )
            .battery_efficiency(self.battery.efficiency)
            .purchase_fee(self.energy_provider.purchase_fee())
            .now(now)
            .quantum(self.quantum)
            .battery_power_limits(self.battery.power_limits)
            .battery_degradation_cost(self.battery.degradation_cost)
            .build();
        let base_loss = solver.base_loss();
        let (summary, steps) = solver.solve().backtrack(initial_energy_level)?;
        println!("{}", build_steps_table(&steps));
        println!("{}", summary.into_table(base_loss));

        let schedule =
            steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
        // TODO: revert back to whole intervals instead of cutting at `now`, that drives FoxESS Cloud crazy:
        let groups = foxcloud::Groups::from_schedule(schedule, now, self.battery.power_limits);
        println!("{}", &groups);

        if !self.scout {
            fox_ess.set_schedule(&self.fox_ess_api.serial_number, groups.as_ref()).await?;
        }

        Ok(())
    }

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(&self, now: DateTime<Local>) -> Result<Vec<(Interval, KilowattHourPrice)>> {
        const ONE_DAY: Days = Days::new(1);

        let today = now.date_naive();
        let mut prices = self.energy_provider.get_prices(today).await?;
        ensure!(!prices.is_empty());

        let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
        prices.extend(self.energy_provider.get_prices(tomorrow).await?);

        prices.retain(|(interval, _)| interval.end > now);
        info!(len = prices.len(), "fetched energy prices");

        Ok(prices)
    }
}
