use crate::core::{series::Series, step::Step, summary::Summary};

pub struct Solution {
    pub summary: Summary,

    /// The simulated working plan.
    pub steps: Series<Step>,
}
