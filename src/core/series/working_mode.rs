use crate::core::{series::Series, working_mode::WorkingMode};

impl<I: Copy> Series<WorkingMode, I> {
    const MODES: [WorkingMode; 4] = [
        WorkingMode::Idle,
        WorkingMode::Balancing,
        WorkingMode::Charging,
        WorkingMode::Discharging,
    ];

    pub fn mutate(&mut self) -> (Mutation<WorkingMode>, Mutation<WorkingMode>) {
        let len = self.0.len();
        assert!(len >= 2);

        let index_1 = fastrand::usize(0..(len - 1));
        let mutation_1 = Mutation { index: index_1, old_value: self[index_1] };

        let index_2 = fastrand::usize(index_1..len);
        let mutation_2 = Mutation { index: index_2, old_value: self[index_2] };

        (self[index_1], self[index_2]) = loop {
            let new_1 = fastrand::choice(Self::MODES).unwrap();
            let new_2 = fastrand::choice(Self::MODES).unwrap();
            if new_1 != self[index_1] || new_2 != self[index_2] {
                break (new_1, new_2);
            }
        };

        (mutation_1, mutation_2)
    }
}

pub struct Mutation<V> {
    pub index: usize,
    pub old_value: V,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_mutate() -> Result {
        let mut series = Series::from_iter([
            (1, WorkingMode::default()),
            (2, WorkingMode::default()),
            (3, WorkingMode::default()),
        ]);
        let original = series.clone();

        let (mutation_1, mutation_2) = series.mutate();
        assert_ne!(series, original, "the mutated series must differ from the original");

        series[mutation_1.index] = mutation_1.old_value;
        series[mutation_2.index] = mutation_2.old_value;
        assert_eq!(series, original, "the restored series must equal to the original");
        Ok(())
    }
}
