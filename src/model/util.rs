use rand::Rng;
use rand::rngs::StdRng;
use std::f32::consts::E;
use rand_distr::StandardNormal;
use nalgebra::{Matrix, Dynamic, U1, U2, VecStorage, ArrayStorage, Vector2, VectorN, RowVectorN};

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
    let sample_mu = sample.row_mean().transpose();
    let sample_var = sample.row_variance().transpose();
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

static NORMAL_SCALE: f32 = 0.8;
pub fn normal_range(rng: &mut StdRng) -> f32 {
    let mut val: f32 = rng.sample(StandardNormal);
    val *= NORMAL_SCALE;
    clamp(val, -1., 1.)
}

pub fn normal_range_mu(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val: f32 = rng.sample(StandardNormal);
    val += mu;
    val *= NORMAL_SCALE;
    clamp(val, -1., 1.)
}

pub fn normal_p(rng: &mut StdRng) -> f32 {
    let mut val = rng.sample(StandardNormal);
    val = (val + 0.5) * 2.;
    val *= NORMAL_SCALE;
    clamp(val, 0., 1.)
}

pub fn normal_p_mu(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val = rng.sample(StandardNormal);
    val += mu;
    val *= NORMAL_SCALE;
    clamp(val, 0., 1.)
}


type X = Matrix<f32, Dynamic, U2, VecStorage<f32, Dynamic, U2>> ;
type Y = Matrix<f32, Dynamic, U1, VecStorage<f32, Dynamic, U1>> ;
pub type Params = Vector2<f32>;
pub fn gradient_descent(
    x: &X,
    y: &Y,
    mut theta: Matrix<f32, U2, U1, ArrayStorage<f32, U2, U1>>,
    alpha: f32,
    iterations: i32,
) -> Matrix<f32, U2, U1, ArrayStorage<f32, U2, U1>> {
    let m = y.len();
    let scalar = 1.0 / m as f32;
    // Vectorized gradient calculation
    let mut prod;
    let mut grad;
    let mut update;
    for _i in 0..iterations {
        prod = x * theta;
        grad = scalar * (x.transpose() * (prod - y));
        update = alpha * grad;
        theta -= update;
    }

    theta
}

pub fn learn_steps(observations: &Vec<f32>, outcomes: &Vec<f32>, theta: Params) -> Params {
    let iterations = 100;
    let alpha = 0.0001; // needs to be quite small to avoid blowups

    let x: X = X::from_row_slice(observations);
    let y: Y = Y::from_row_slice(outcomes);
    gradient_descent(&x, &y, theta, alpha, iterations)
}
