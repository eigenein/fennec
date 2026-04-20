use std::iter::once;

use chrono::{TimeDelta, Timelike};
use fennec_modbus::contrib::mq2200::{
    schedule,
    schedule::{NaiveTime, WorkingMode},
};

use crate::{
    battery,
    cli::battery::BatteryPowerLimits,
    ops::{Interval, range},
    prelude::*,
    quantity::{Zero, power::Watts, ratios::Percentage},
};

#[instrument(skip_all)]
pub fn build(
    schedule: impl IntoIterator<Item = (Interval, battery::WorkingMode)>,
    charge_range: range::Inclusive<Percentage>,
    battery_power_limits: BatteryPowerLimits,
) -> schedule::Full {
    info!("building a Fox ESS schedule…");
    let mut schedule: Vec<_> = schedule
        .into_iter()
        .scan(None, |schedule_end, (mut interval, working_mode)| {
            if let Some(schedule_end) = *schedule_end {
                if interval.start >= schedule_end {
                    // Stop at first interval which starts outside the schedule.
                    return None;
                }
                if interval.end > schedule_end {
                    // Just in case the interval crosses the schedule boundary, cut it.
                    interval = interval.with_end(schedule_end);
                }
            } else {
                // On first interval, just define the schedule end time (exclusive boundary).
                // Fox Cloud only accepts a 24-hour schedule.
                *schedule_end = Some(interval.start + TimeDelta::days(1));
            }
            Some((interval, working_mode))
        })
        .flat_map(|(interval, working_mode)| {
            into_time_slots(interval)
                .flatten()
                .map(move |(start_time, end_time)| (working_mode, start_time, end_time))
        })
        .map(|(working_mode, start_time, end_time)| {
            let (working_mode, target_charge, feed_power) = match working_mode {
                battery::WorkingMode::Idle => {
                    // Forced charging at 0W is effectively idling:
                    (schedule::WorkingMode::ForceCharge, charge_range.max, Watts::ZERO)
                }
                battery::WorkingMode::Harness => {
                    (schedule::WorkingMode::BackUp, charge_range.max, battery_power_limits.charging)
                }
                battery::WorkingMode::Charge => (
                    schedule::WorkingMode::ForceCharge,
                    charge_range.max,
                    battery_power_limits.charging,
                ),
                battery::WorkingMode::SelfUse => (
                    schedule::WorkingMode::SelfUse,
                    charge_range.min,
                    battery_power_limits.discharging,
                ),
                battery::WorkingMode::Discharge => (
                    schedule::WorkingMode::ForceDischarge,
                    charge_range.min,
                    battery_power_limits.discharging,
                ),
                battery::WorkingMode::Compensate => (
                    schedule::WorkingMode::FeedInPriority,
                    charge_range.min,
                    battery_power_limits.discharging,
                ),
            };

            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_sign_loss)]
            schedule::Entry {
                is_enabled: true,
                start_time,
                end_time,
                working_mode,
                maximum_state_of_charge: fennec_modbus::contrib::Percentage(
                    charge_range.max.0 as u8,
                ),
                minimum_state_of_charge: fennec_modbus::contrib::Percentage(
                    charge_range.min.0 as u8,
                ),
                target_state_of_charge: fennec_modbus::contrib::Percentage(target_charge.0),
                power: fennec_modbus::contrib::Watts(feed_power.0 as u16),
                reserved_1: 0,
                reserved_2: 0,
                reserved_3: 0,
            }
        })
        .collect();

    // Actual contents should not matter, but set them to something reasonable anyway:
    #[expect(clippy::cast_possible_truncation)]
    let disabled_entry = schedule::Entry {
        is_enabled: false,
        start_time: NaiveTime { hour: 0, minute: 0 },
        end_time: NaiveTime { hour: 0, minute: 0 },
        working_mode: WorkingMode::SelfUse,
        maximum_state_of_charge: fennec_modbus::contrib::Percentage(charge_range.max.0 as u8),
        minimum_state_of_charge: fennec_modbus::contrib::Percentage(charge_range.min.0 as u8),
        target_state_of_charge: fennec_modbus::contrib::Percentage(100),
        power: fennec_modbus::contrib::Watts(0),
        reserved_1: 0,
        reserved_2: 0,
        reserved_3: 0,
    };
    schedule.extend(vec![disabled_entry; schedule::Entry::N_TOTAL - schedule.len()]);

    schedule.try_into().expect("invalid schedule entry count")
}

fn into_time_slots(interval: Interval) -> impl Iterator<Item = Option<(NaiveTime, NaiveTime)>> {
    #[expect(clippy::cast_possible_truncation)]
    let start_time =
        NaiveTime { hour: interval.start.hour() as u8, minute: interval.start.minute() as u8 };

    #[expect(clippy::cast_possible_truncation)]
    let end_time =
        NaiveTime { hour: interval.end.hour() as u8, minute: interval.end.minute() as u8 };

    if end_time.hour == 0 && end_time.minute == 0 {
        // FoxESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        return once(Some((start_time, NaiveTime::MAX))).chain(once(None));
    }

    if interval.start.date_naive() == interval.end.date_naive() {
        once(Some((start_time, end_time))).chain(once(None))
    } else {
        // Split cross-day time spans because we cannot have time slots like 22:00-02:00:
        once(Some((start_time, NaiveTime::MAX))).chain(once(Some((NaiveTime::MIN, end_time))))
    }
}
