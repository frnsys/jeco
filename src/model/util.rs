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

