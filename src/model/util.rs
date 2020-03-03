use rand::Rng;
use rand::rngs::StdRng;
use std::f32::consts::E;
use rand_distr::StandardNormal;
use std::collections::VecDeque;
use fnv::FnvHashSet;
use std::hash::Hash;
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

pub fn learn_steps(observations: &[f32], outcomes: &[f32], theta: Params) -> Params {
    let iterations = 100;
    let alpha = 0.0001; // needs to be quite small to avoid blowups

    let x: X = X::from_row_slice(observations);
    let y: Y = Y::from_row_slice(outcomes);
    gradient_descent(&x, &y, theta, alpha, iterations)
}

#[derive(Debug)]
pub struct LimitedQueue<T> {
    _vec: Vec<T>,
    capacity: usize,
}

impl<T> LimitedQueue<T> {
    pub fn new(capacity: usize) -> LimitedQueue<T> {
        LimitedQueue {
            capacity: capacity,
            _vec: Vec::with_capacity(capacity)
        }
    }

    pub fn push(&mut self, val: T) {
        self._vec.insert(0, val);
        self._vec.truncate(self.capacity);
    }

    pub fn extend(&mut self, vals: Vec<T>) {
        self._vec.extend(vals);
        self._vec.truncate(self.capacity);
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self._vec.iter()
    }

    pub fn len(&self) -> usize {
        self._vec.len()
    }

    pub fn as_slice(&self) -> &[T] {
        self._vec.as_slice()
    }
}


#[derive(Debug)]
pub struct LimitedSet<T: Eq + Hash + Clone> {
    _vec: VecDeque<T>,
    _set: FnvHashSet<T>,
    capacity: usize,
}

impl<T: Eq + Hash + Clone> LimitedSet<T> {
    pub fn new(capacity: usize) -> LimitedSet<T> {
        LimitedSet {
            capacity: capacity,
            _set: FnvHashSet::default(),
            _vec: VecDeque::with_capacity(capacity)
        }
    }

    pub fn insert(&mut self, val: T) {
        self._vec.push_front(val.clone());
        if self._vec.len() > self.capacity {
            match self._vec.pop_back() {
                Some(v) => {
                    self._set.remove(&v);
                },
                None => {}
            }
        }
        self._set.insert(val);
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self._vec.iter()
    }

    pub fn len(&self) -> usize {
        self._vec.len()
    }

    pub fn contains(&self, val: &T) -> bool {
        self._set.contains(val)
    }
}

#[derive(Debug)]
pub struct Learner {
    theta: Params,
    observations: LimitedQueue<f32>,
    outcomes: LimitedQueue<f32>,
    pub params: Params,
}

impl Learner {
    pub fn new(memory: usize, rng: &mut StdRng) -> Learner {
        let theta = Params::new(rng.gen(), rng.gen());
        Learner {
            theta: theta,
            observations: LimitedQueue::new(memory),
            outcomes: LimitedQueue::new(memory),
            params: theta.clone()
        }
    }

    pub fn learn(&mut self, observations: Vec<f32>, outcome: f32, change_rate: f32) {
        self.outcomes.push(outcome);
        self.observations.extend(observations);

        self.theta = learn_steps(&self.observations.as_slice(),
            &self.outcomes.as_slice(), self.theta);

        self.params.x += change_rate * self.theta.x;
        self.params.y += change_rate * self.theta.y;

        // Assuming params must be >= 0
        self.params.x = f32::max(0., self.params.x);
        self.params.y = f32::max(0., self.params.y);
    }
}
