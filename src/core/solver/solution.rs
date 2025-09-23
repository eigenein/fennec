use crate::core::{
    series::Series,
    solver::{step::Step, summary::Summary},
};

pub struct Solution {
    pub summary: Summary,

    /// The simulated working plan.
    pub steps: Series<Step>,
}
