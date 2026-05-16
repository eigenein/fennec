use std::{f64::consts::LN_2, time::Duration};

#[must_use]
#[derive(Copy, Clone)]
pub struct Component {
    pub weight: f64,
    pub mean: f64,
    m2: f64,
}

impl Component {
    pub const fn new(mean: f64, weight: f64) -> Self {
        Self { weight, mean, m2: 0.0 }
    }

    /// Update the component using [Welford's online algorithm][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
    pub fn update(&mut self, value: f64, weight: f64) {
        let delta = value - self.mean;
        self.weight += weight;
        self.mean += (weight / self.weight) * delta;
        self.m2 += weight * delta * (value - self.mean);
    }

    /// Maximum-likelihood variance (floored to [`f64::MIN_POSITIVE`] for numerical safety).
    pub fn variance(self) -> f64 {
        if self.weight > f64::EPSILON {
            (self.m2 / self.weight).max(f64::MIN_POSITIVE)
        } else {
            f64::MIN_POSITIVE
        }
    }

    /// Multiply all weights by `factor`. Mean is invariant; variance is preserved.
    pub fn decay(&mut self, factor: f64) {
        self.weight *= factor;
        self.m2 *= factor;
    }

    pub fn log_responsibility(self, value: f64, prior_variance: f64) -> f64 {
        self.weight.ln()
            + normal_log_pdf(value, self.mean, self.predictive_variance(prior_variance))
    }

    /// Predictive variance: M2 regularized with one pseudo-observation
    /// drawn from the base measure (κ₀ = 1).
    fn predictive_variance(self, prior_variance: f64) -> f64 {
        ((self.m2 + prior_variance) / (self.weight + 1.0)).max(f64::MIN_POSITIVE)
    }
}

/// Online infinite Gaussian mixture model via the Dirichlet Process.
///
/// Internally uses an empirical Bayes base measure (exponentially-weighted data statistics)
/// and Bayesian variance regularization with κ₀ = 1 (one prior pseudo-observation).
#[must_use]
pub struct InfiniteGmm {
    /// Log-concentration – prior pseudo-count for unseen components.
    ///
    /// Higher → more components (overfitting risk), lower → fewer (underfitting risk).
    ln_alpha: f64,

    /// Exponential decay rate for non-stationary tracking (seconds⁻¹).
    decay_rate: f64,

    components: Vec<Component>,

    prior: Component,
}

impl InfiniteGmm {
    pub fn new(alpha: f64, half_life: Duration) -> Self {
        let half_life_secs = half_life.as_secs_f64();
        assert!(alpha > 0.0, "alpha must be positive");
        assert!(half_life_secs > 0.0, "half-life must be positive");
        Self {
            ln_alpha: alpha.ln(),
            decay_rate: LN_2 / half_life_secs,
            components: Vec::new(),
            prior: Component::new(0.0, 0.0),
        }
    }

    /// Feed a new observation.
    pub fn observe(&mut self, value: f64, elapsed: Duration) {
        let decay_factor = (-self.decay_rate * elapsed.as_secs_f64()).exp();
        for component in &mut self.components {
            component.decay(decay_factor);
        }
        self.prior.decay(decay_factor);

        // First observation: seed the model.
        if self.components.is_empty() {
            self.components.push(Component::new(value, 1.0));
            self.prior.update(value, 1.0);
            return;
        }

        let prior_variance = self.prior.variance();

        // Unnormalized log-responsibilities for existing components:
        let mut log_responsibilities: Vec<f64> = self
            .components
            .iter()
            .map(|component| component.log_responsibility(value, prior_variance))
            .collect();

        let log_new_responsibility =
            self.ln_alpha + normal_log_pdf(value, self.prior.mean, prior_variance);
        log_responsibilities.push(log_new_responsibility);

        let log_normalizer = log_sum_exp(&log_responsibilities);

        // Soft-assign to existing components:
        for (component, &log_responsibility) in
            self.components.iter_mut().zip(&log_responsibilities)
        {
            component.update(value, (log_responsibility - log_normalizer).exp());
        }

        let new_responsibility = (log_new_responsibility - log_normalizer).exp();
        if new_responsibility > f64::EPSILON {
            // Spawn a new component when the responsibility is non-negligible:
            self.components.push(Component::new(value, new_responsibility));
        }

        self.prior.update(value, 1.0);

        // Prune degenerate components:
        self.components.retain(|component| component.weight > f64::EPSILON);
    }
}

/// Log of the Gaussian probability density: ln N(x; μ, σ²).
fn normal_log_pdf(x: f64, mean: f64, variance: f64) -> f64 {
    const LN_TAU: f64 = 1.837_877_066_409_345_483_560_659_472_811_235_279_f64;

    let variance = variance.max(f64::MIN_POSITIVE);
    let delta = x - mean;

    // Equivalent to −ln(σ) − ½ln(2π) − (x−μ)²/2σ², with −ln(σ) = −½ln(σ²):
    -0.5 * (delta * delta / variance + variance.ln() + LN_TAU)
}

/// [Log-sum-exp trick][1] for numerical stability.
///
/// [1]: https://en.wikipedia.org/wiki/LogSumExp#log-sum-exp_trick_for_log-domain_calculations
fn log_sum_exp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    max + values.iter().map(|&value| (value - max).exp()).sum::<f64>().ln()
}
