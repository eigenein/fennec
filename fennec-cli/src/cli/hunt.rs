use chrono::{Local, Timelike};
use itertools::Itertools;

use crate::{
    api::{foxess, modbus},
    cli::HuntArgs,
    core::{interval::Interval, solver::Solver},
    db::{
        battery_log::BatteryLogs,
        state::{HourlyStandByPower, States},
    },
    prelude::*,
    statistics::battery::BatteryEfficiency,
    tables::{build_steps_table, build_time_slot_sequence_table},
};

#[instrument(skip_all)]
pub async fn hunt(args: &HuntArgs) -> Result {
    let db = args.db.connect().await?;
    let statistics = States::from(&db).get::<HourlyStandByPower>().await?.unwrap_or_default();

    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key.clone())?;
    let working_modes = args.working_modes();

    let now = Local::now().with_nanosecond(0).unwrap();
    let grid_rates = args.provider.get_upcoming_rates(now).await?;

    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "fetched energy rates");

    let battery_state = modbus::Client::connect(&args.battery.connection)
        .await?
        .read_battery_state(args.battery.registers)
        .await?;
    let min_state_of_charge = battery_state.settings.min_state_of_charge;
    let max_state_of_charge = battery_state.settings.max_state_of_charge;

    let battery_efficiency = {
        let battery_logs = BatteryLogs::from(&db);
        BatteryEfficiency::try_estimate(
            battery_logs.find(Interval::try_since(args.estimation.duration())?).await?,
        )
        .await?
    };

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .hourly_stand_by_power(&statistics.into())
        .working_modes(working_modes)
        .battery_state(battery_state)
        .battery_power_limits(args.battery.power_limits)
        .battery_efficiency(battery_efficiency)
        .purchase_fee(args.provider.purchase_fee())
        .now(now)
        .degradation_rate(args.degradation_rate)
        .solve()
        .context("no solution found, try allowing additional working modes")?;
    let steps = solution.backtrack().collect_vec();
    println!("{}", build_steps_table(&steps, args.battery.power_limits.discharging_power));

    let schedule = steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
    let time_slot_sequence = foxess::TimeSlotSequence::from_schedule(
        schedule,
        now,
        args.battery.power_limits,
        min_state_of_charge,
        max_state_of_charge,
    )?;
    println!("{}", build_time_slot_sequence_table(&time_slot_sequence));

    if !args.scout {
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref()).await?;
    }

    args.heartbeat.send().await;
    Ok(())
}
