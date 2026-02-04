use chrono::{Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;
use reqwest::Url;

use crate::{
    api::{foxess, heartbeat},
    cli::{battery::BatteryArgs, db::DbArgs, estimation::EstimationArgs, foxess::FoxEssApiArgs},
    core::{provider::Provider, solver::Solver, working_mode::WorkingMode},
    db::{battery::BatteryLog, consumption::ConsumptionLog},
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
        default_value = "backup,balance,charge",
    )]
    working_modes: Vec<WorkingMode>,

    /// Battery degradation rate per kilowatt-hour of the energy flow.
    #[clap(long, env = "DEGRADATION_RATE", default_value = "0")]
    degradation_rate: KilowattHourRate,

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

        let fox_ess = foxess::Api::new(self.fox_ess_api.api_key.clone())?;
        let working_modes = self.working_modes();

        let now = Local::now().with_nanosecond(0).unwrap();
        let grid_rates = self.provider.get_upcoming_rates(now).await?;

        ensure!(!grid_rates.is_empty());
        info!(len = grid_rates.len(), "fetched energy rates");

        let battery_state = self
            .battery
            .connection
            .connect()
            .await?
            .read_battery_state(self.battery.registers)
            .await?;
        let min_state_of_charge = battery_state.settings.min_state_of_charge;
        let max_state_of_charge = battery_state.settings.max_state_of_charge;

        let since = self.estimation.since();
        let battery_efficiency = {
            let battery_logs = db.find_logs::<BatteryLog>(since).await?;
            BatteryEfficiency::try_estimate(battery_logs, self.estimation.weight_mode)
                .await
                .inspect_err(|error| warn!("assuming an ideal battery: {error:#}"))
                .unwrap_or_default()
        };
        let consumption_statistics = {
            let consumption_logs = db.find_logs::<ConsumptionLog>(since).await?;
            ConsumptionStatistics::try_estimate(consumption_logs).await?
        };
        println!("{}", consumption_statistics.summary_table());

        let solution = Solver::builder()
            .grid_rates(&grid_rates)
            .consumption_statistics(&consumption_statistics)
            .working_modes(working_modes)
            .battery_state(battery_state)
            .battery_power_limits(self.battery.power_limits)
            .battery_efficiency(battery_efficiency)
            .purchase_fee(self.provider.purchase_fee())
            .now(now)
            .degradation_rate(self.degradation_rate)
            .solve()
            .context("no solution found, try allowing additional working modes")?;
        let steps = solution.backtrack().collect_vec();
        println!("{}", build_steps_table(&steps, self.battery.power_limits.discharging_power));

        let schedule =
            steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
        let time_slot_sequence = foxess::TimeSlotSequence::from_schedule(
            schedule,
            now,
            self.battery.power_limits,
            min_state_of_charge,
            max_state_of_charge,
        )?;
        println!("{}", &time_slot_sequence);

        if !self.scout {
            fox_ess
                .set_schedule(&self.fox_ess_api.serial_number, time_slot_sequence.as_ref())
                .await?;
        }

        heartbeat::Client::new(self.heartbeat_url).send().await;
        Ok(())
    }
}
