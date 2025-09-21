use chrono::{DateTime, Local};

use crate::core::{step::Step, summary::Summary};

pub struct Solution {
    pub summary: Summary,

    /// The simulated working plan.
    ///
    /// Note, that I could not use [`crate::core::series::Series`] here to avoid the b-tree insertion penalty.
    pub steps: Vec<(DateTime<Local>, Step)>,
}
