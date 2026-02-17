use chrono::{Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;
use reqwest::Url;

use crate::{
    api::{foxcloud, heartbeat},
    cli::{battery::BatteryArgs, db::DbArgs, estimation::EstimationArgs, foxess::FoxEssApiArgs},
    core::{
        energy_level::Quantum,
        provider::Provider,
        solution::SolutionSummary,
        solver::Solver,
        working_mode::WorkingMode,
    },
    db::{battery, consumption},
    prelude::*,
    quantity::rate::KilowattHourRate,
    statistics::{battery::BatteryEfficiency, consumption::ConsumptionStatistics},
    tables::build_steps_table,
};

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    scout: bool,

    #[clap(long = "provider", env = "PROVIDER", default_value = "next-energy")]
    provider: Provider,

    #[clap(
        long = "working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "harvest,self-use,charge",
    )]
    working_modes: Vec<WorkingMode>,

    /// Battery degradation rate per kilowatt-hour of the energy flow.
    #[clap(long, env = "DEGRADATION_RATE", default_value = "0")]
    degradation_rate: KilowattHourRate,

    #[clap(long = "quantum-kilowatts", env = "QUANTUM_KILOWATTS", default_value = "0.001")]
    quantum: Quantum,

    #[clap(flatten)]
    battery: BatteryArgs,

    #[clap(flatten)]
    fox_ess_api: FoxEssApiArgs,

    #[clap(flatten)]
    estimation: EstimationArgs,

    #[clap(flatten)]
    db: DbArgs,

    #[clap(long = "heartbeat-url", env = "HUNT_HEARTBEAT_URL")]
    heartbeat_url: Option<Url>,
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
        let grid_rates = self
            .provider
            .get_rates_two_days(now.date_naive())
            .await?
            .into_iter()
            .filter(|(interval, _)| interval.end > now)
            .collect_vec();

        ensure!(!grid_rates.is_empty());
        info!(len = grid_rates.len(), "fetched energy rates");

        let battery_state = self.battery.connection.connect().await?.read().await?;
        info!(
            residual_energy = ?battery_state.energy.residual(),
            state_of_charge = ?battery_state.energy.state_of_charge,
            state_of_health = ?battery_state.energy.state_of_health,
        );

        let since = self.estimation.since();
        let battery_efficiency = {
            let battery_logs = db.find_logs::<battery::LogEntry>(since).await?;
            BatteryEfficiency::try_estimate(battery_logs, self.estimation.weight_mode)
                .await
                .inspect_err(|error| warn!("assuming an ideal battery: {error:#}"))
                .unwrap_or_default()
        };
        let consumption_statistics = {
            let consumption_logs = db.find_logs::<consumption::LogEntry>(since).await?;
            ConsumptionStatistics::try_estimate(self.battery.power_limits, consumption_logs).await?
        };
        db.shutdown().await;

        let initial_energy_level = self.quantum.quantize(battery_state.energy.residual());
        let solver = Solver::builder()
            .grid_rates(&grid_rates)
            .consumption_statistics(&consumption_statistics)
            .working_modes(working_modes)
            .min_residual_energy(battery_state.min_residual_energy())
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                battery_state.max_residual_energy().max(battery_state.energy.residual()),
            )
            .battery_efficiency(battery_efficiency)
            .purchase_fee(self.provider.purchase_fee())
            .now(now)
            .degradation_rate(self.degradation_rate)
            .quantum(self.quantum)
            .build();
        let base_loss = solver.base_loss();
        let (loss, steps) = solver.solve().backtrack(initial_energy_level)?;
        println!("{}", SolutionSummary { loss, base_loss });
        println!("{}", build_steps_table(&steps));

        let schedule =
            steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
        let groups = foxcloud::Groups::from_schedule(schedule, now, self.battery.power_limits);
        println!("{}", &groups);

        if !self.scout {
            fox_ess.set_schedule(&self.fox_ess_api.serial_number, groups.as_ref()).await?;
        }

        heartbeat::Client::new(self.heartbeat_url).send().await;
        Ok(())
    }
}
