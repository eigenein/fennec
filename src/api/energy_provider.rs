use std::ops::Range;

use chrono::{DateTime, Local};

use crate::{core::series::Point, prelude::*, quantity::rate::KilowattHourRate};

pub trait EnergyProvider {
    async fn get_upcoming_rates(
        &self,
        since: DateTime<Local>,
    ) -> Result<impl Iterator<Item = Point<Range<DateTime<Local>>, KilowattHourRate>>>;
}
