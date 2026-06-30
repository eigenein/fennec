use std::{iter::once, range::RangeInclusive};

use chrono::{DateTime, Local, Timelike};
use fennec_modbus::{
    contrib,
    contrib::mini_qube::{schedule, schedule::NaiveTime},
};

use crate::{
    battery,
    ops::interval::Interval,
    prelude::*,
    quantity::{Zero, power::Watts, ratios::Percentage},
};

#[instrument(skip_all)]
pub fn build(
    schedule: impl IntoIterator<Item = (Interval<DateTime<Local>>, battery::WorkingMode)>,
    charge_limits: RangeInclusive<Percentage>,
    power_limits: battery::PowerLimits,
) -> schedule::Full {
    info!("building a Fox ESS schedule…");
    let mut schedule: Vec<_> = schedule
        .into_iter()
        .flat_map(|(interval, working_mode)| {
            into_time_slots(interval)
                .flatten()
                .map(move |(start_time, end_time)| (working_mode, start_time, end_time))
        })
        .take(schedule::Entry::N_TOTAL)
        .map(|(working_mode, start_time, end_time)| {
            let (working_mode, target_charge, feed_power) = match working_mode {
                battery::WorkingMode::Idle => {
                    // Forced charging at 0W is effectively idling:
                    (schedule::WorkingMode::ForceCharge, charge_limits.last, Watts::ZERO)
                }
                battery::WorkingMode::Harness => {
                    (schedule::WorkingMode::BackUp, charge_limits.last, power_limits.charging)
                }
                battery::WorkingMode::Charge => {
                    (schedule::WorkingMode::ForceCharge, charge_limits.last, power_limits.charging)
                }
                battery::WorkingMode::SelfUse => {
                    (schedule::WorkingMode::SelfUse, charge_limits.start, power_limits.discharging)
                }
                battery::WorkingMode::Discharge => (
                    schedule::WorkingMode::ForceDischarge,
                    charge_limits.start,
                    power_limits.discharging,
                ),
                battery::WorkingMode::Compensate => (
                    schedule::WorkingMode::FeedInPriority,
                    charge_limits.start,
                    power_limits.discharging,
                ),
            };

            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_sign_loss)]
            schedule::Entry {
                is_enabled: true,
                start_time,
                end_time,
                working_mode,
                maximum_state_of_charge: charge_limits.last.into(),
                minimum_state_of_charge: charge_limits.start.into(),
                target_state_of_charge: target_charge.into(),
                power: contrib::Watts(feed_power.0 as u16),
                reserved_1: 0,
                reserved_2: 0,
                reserved_3: 0,
            }
        })
        .collect();

    // Actual contents should not matter, but set them to something reasonable anyway:
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    let disabled_entry = schedule::Entry {
        is_enabled: false,
        start_time: NaiveTime::MIN,
        end_time: NaiveTime::MIN,
        working_mode: schedule::WorkingMode::SelfUse, // TODO: make configurable by user.
        maximum_state_of_charge: contrib::Percentage(charge_limits.last.0),
        minimum_state_of_charge: contrib::Percentage(charge_limits.start.0),
        target_state_of_charge: contrib::Percentage(100),
        power: contrib::Watts(power_limits.max_inverter_power.0 as u16),
        reserved_1: 0,
        reserved_2: 0,
        reserved_3: 0,
    };
    schedule.extend(vec![disabled_entry; schedule::Entry::N_TOTAL - schedule.len()]);

    schedule.try_into().expect("invalid schedule entry count")
}

fn into_time_slots(
    interval: Interval<DateTime<Local>>,
) -> impl Iterator<Item = Option<(NaiveTime, NaiveTime)>> {
    let start_time = NaiveTime {
        hour: interval.start().hour().try_into().unwrap(),
        minute: interval.start().minute().try_into().unwrap(),
    };
    let end_time = NaiveTime {
        hour: interval.end().hour().try_into().unwrap(),
        minute: interval.end().minute().try_into().unwrap(),
    };

    if end_time.hour == 0 && end_time.minute == 0 {
        // FoxESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        return once(Some((start_time, NaiveTime::MAX))).chain(once(None));
    }
    if interval.start().date_naive() == interval.end().date_naive() {
        // Same day, just emit the interval "as is".
        once(Some((start_time, end_time))).chain(once(None))
    } else {
        // Split cross-day time spans because we cannot have time slots like 22:00-02:00:
        once(Some((start_time, NaiveTime::MAX))).chain(once(Some((NaiveTime::MIN, end_time))))
    }
}
