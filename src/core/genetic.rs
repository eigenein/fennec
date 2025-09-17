use std::{cmp::Ordering, collections::BinaryHeap};

use crate::prelude::*;

/// Mini version of a genetic algorithm to optimize the battery schedule.
pub struct Optimizer<D, const N: usize> {
    loss: fn(&D) -> Result<f64>,
    population: BinaryHeap<Solution<D>>,
}

impl<D, const N: usize> Optimizer<D, N> {
    pub fn new(initial: impl IntoIterator<Item = D>, loss: fn(&D) -> Result<f64>) -> Result<Self> {
        let population = initial
            .into_iter()
            .map(|dna| {
                let loss = loss(&dna)?;
                Ok::<_, Error>(Solution::new(dna, loss))
            })
            .collect::<Result<_>>()?;
        Ok(Self { loss, population })
    }
}

impl<D: Dna, const N: usize> Optimizer<D, N> {
    pub fn step(&mut self) -> Result {
        let mut child = {
            let parent_1 = fastrand::choice(&self.population).context("no parents available")?;
            let parent_2 = fastrand::choice(&self.population).context("no parents available")?;
            parent_1.dna.crossover_with(&parent_2.dna)
        };
        if fastrand::bool() {
            child.mutate();
        }
        let loss = (self.loss)(&child)?;
        self.population.push(Solution::new(child, loss));
        while self.population.len() > N {
            self.population.pop().unwrap();
        }
        Ok(())
    }
}

pub trait Dna {
    fn mutate(&mut self);

    fn crossover_with(&self, other: &Self) -> Self;
}

#[derive(derive_more::Constructor)]
struct Solution<DNA> {
    dna: DNA,
    loss: f64,
}

impl<DNA> PartialEq<Self> for Solution<DNA> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<DNA> Eq for Solution<DNA> {}

impl<DNA> PartialOrd<Self> for Solution<DNA> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<DNA> Ord for Solution<DNA> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.loss.total_cmp(&other.loss)
    }
}
