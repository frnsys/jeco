use std::f32::consts::E;
use nalgebra::{Matrix, Dynamic, U2, VecStorage, VectorN, RowVectorN};

// 2 so can be plotted in 2d
pub static VECTOR_SIZE: u32 = 2;
pub type Vector = VectorN<f32, U2>;

// Exponentially weighted moving average
pub static EWMA_ALPHA: f32 = 0.7;
pub fn ewma(mu: f32, prev: f32) -> f32 {
    EWMA_ALPHA * mu + (1. - EWMA_ALPHA) * prev
}

pub fn z_score(a: &Vector, params: &(Vector, Vector)) -> Vector {
    let (mu, var) = params;
    let std = var.map(|x| x.sqrt());
    (a - mu).abs().component_div(&std)
}

// Bayesian normal update
pub type Sample = Matrix<f32, Dynamic, U2, VecStorage<f32, Dynamic, U2>>;
pub type SampleRow = RowVectorN<f32, U2>;
pub fn bayes_update(prior: (Vector, Vector), sample: Sample) -> (Vector, Vector) {
    let (prior_mu, prior_var) = prior;
    let sample_mu = sample.column_mean();
    let sample_var = sample.column_variance();
    let denom = prior_var + &sample_var;
    let post_mu = (sample_var.component_mul(&prior_mu) + prior_var.component_mul(&sample_mu)).component_div(&denom);
    let post_var = sample_var.component_mul(&prior_var).component_div(&denom);
    (post_mu, post_var)
}

pub fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

// Returns how much a moves towards b
pub fn gravity(a: f32, b: f32, gravity_stretch: f32, max_influence: f32) -> f32 {
    let mut dist = a - b;
    let sign = dist.signum();
    dist = dist.abs();
    if dist == 0. {
        // Already here, no movement
        0.
    } else {
        let strength = (1. / dist) / gravity_stretch;
        let movement = strength / (strength + 1.) * max_influence;
        f32::min(movement, dist) * sign
    }
}

pub fn sigmoid(x: f32) -> f32 {
    1./(1.+E.powf(-x))
}
