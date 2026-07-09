use std::range::RangeInclusive;

use chrono::{DateTime, Local, Timelike};
use fennec_modbus::contrib::{
    mini_qube::{schedule, schedule::NaiveTime},
    types,
};

use crate::{
    battery,
    ops::interval::Interval,
    quantity::{Zero, power::Watts, ratios::Percentage},
};

/// Get the schedule slot index corresponding to the interval.
#[must_use]
pub fn index_of(interval: Interval<DateTime<Local>>) -> u8 {
    let start = interval.start();
    (start.hour() * 4 + start.minute() / 15).try_into().unwrap()
}

pub fn slot_interval(index: u8) -> (NaiveTime, NaiveTime) {
    let start = NaiveTime { hour: index / 4, minute: (index % 4) * 15 };
    let end = if u16::from(index) == schedule::Slot::N_TOTAL - 1 {
        // Fox ESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        NaiveTime::MAX
    } else {
        NaiveTime { hour: (index + 1) / 4, minute: ((index + 1) % 4) * 15 }
    };
    (start, end)
}

/// Make the battery schedule entry according the working mode and schedule limits.
pub fn make_slot(
    slot_index: u8,
    working_mode: battery::WorkingMode,
    allowed_soc: RangeInclusive<Percentage>,
    power_limits: battery::PowerLimits,
) -> schedule::Slot {
    let (start_time, end_time) = slot_interval(slot_index);
    let (working_mode, target_charge, feed_power) = match working_mode {
        battery::WorkingMode::Idle => {
            (schedule::WorkingMode::ForceCharge, allowed_soc.last, Watts::ZERO)
        }
        battery::WorkingMode::Harness => {
            (schedule::WorkingMode::BackUp, allowed_soc.last, power_limits.charging)
        }
        battery::WorkingMode::Charge => {
            (schedule::WorkingMode::ForceCharge, allowed_soc.last, power_limits.charging)
        }
        battery::WorkingMode::SelfUse => {
            (schedule::WorkingMode::SelfUse, allowed_soc.start, power_limits.discharging)
        }
        battery::WorkingMode::Discharge => {
            (schedule::WorkingMode::ForceDischarge, allowed_soc.start, power_limits.discharging)
        }
        battery::WorkingMode::Compensate => {
            (schedule::WorkingMode::FeedInPriority, allowed_soc.start, power_limits.discharging)
        }
    };

    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    schedule::Slot {
        is_enabled: true,
        start_time,
        end_time,
        working_mode,
        max_state_of_charge: allowed_soc.last.into(),
        min_state_of_charge: allowed_soc.start.into(),
        target_state_of_charge: target_charge.into(),
        power: types::Watts(feed_power.0 as u16),
        reserved_1: 0,
        reserved_2: 0,
        reserved_3: 0,
    }
}
