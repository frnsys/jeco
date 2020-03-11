use rand::Rng;
use rand::rngs::StdRng;
use std::f32::consts::E;
use rand_distr::StandardNormal;
use std::collections::VecDeque;
use fnv::{FnvHashMap, FnvHashSet};
use std::hash::Hash;
use rand::seq::SliceRandom;
use nalgebra::{Matrix, Dynamic, U2, VecStorage, VectorN, RowVectorN};

// 2 so can be plotted in 2d
pub static VECTOR_SIZE: u32 = 2;
pub type Vector = VectorN<f32, U2>;

// Exponentially weighted moving average
pub static EWMA_ALPHA: f32 = 0.7;
pub fn ewma(mu: f32, prev: f32) -> f32 {
    EWMA_ALPHA * mu + (1. - EWMA_ALPHA) * prev
}

// Bayesian normal update
pub type Sample = Matrix<f32, Dynamic, U2, VecStorage<f32, Dynamic, U2>>;
pub type SampleRow = RowVectorN<f32, U2>;
static EPSILON: f32 = 1e-10;
pub fn bayes_update(prior: (Vector, Vector), sample: Sample) -> (Vector, Vector) {
    let (prior_mu, prior_var) = prior;
    let sample_mu = sample.row_mean().transpose();
    let sample_var = sample.row_variance().transpose().add_scalar(EPSILON);
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
    let mut dist = b - a;
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

static NORMAL_SCALE: f32 = 1.2;
static NORMAL_SCALE_TIGHT: f32 = 0.05;
pub fn normal_range(rng: &mut StdRng) -> f32 {
    let mut val: f32 = rng.sample(StandardNormal);
    val *= NORMAL_SCALE;
    clamp(val, -1., 1.)
}

pub fn normal_range_mu(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val: f32 = rng.sample(StandardNormal);
    val *= NORMAL_SCALE;
    val += mu;
    clamp(val, -1., 1.)
}

pub fn normal_range_mu_tight(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val: f32 = rng.sample(StandardNormal);
    val *= NORMAL_SCALE_TIGHT;
    val += mu;
    clamp(val, -1., 1.)
}

pub fn normal_p(rng: &mut StdRng) -> f32 {
    let mut val = rng.sample(StandardNormal);
    val *= NORMAL_SCALE;
    val = (val + 0.5) * 2.;
    clamp(val, 0., 1.)
}

pub fn normal_p_mu(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val = rng.sample(StandardNormal);
    val *= NORMAL_SCALE;
    val += mu;
    clamp(val, 0., 1.)
}

pub fn normal_p_mu_tight(mu: f32, rng: &mut StdRng) -> f32 {
    let mut val = rng.sample(StandardNormal);
    val *= NORMAL_SCALE_TIGHT;
    val += mu;
    clamp(val, 0., 1.)
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

// Quality, Ads
// ParamsKey is separate b/c f32s aren't hashable
pub type Params = (f32, f32);
pub type ParamsKey = (usize, usize);

#[derive(Debug)]
pub struct Learner {
    params: ParamsKey,
    history: FnvHashMap<ParamsKey, f32>,
}

static MIN_QUALITY: usize = 0;
static MAX_QUALITY: usize = 1;
static STEPS_QUALITY: usize = 10;

static MIN_ADS: usize = 0;
static MAX_ADS: usize = 10;
static STEPS_ADS: usize = 10;


impl Learner {
    pub fn new(rng: &mut StdRng) -> Learner {
        let keys: Vec<ParamsKey> = (0..STEPS_QUALITY+1)
            .flat_map(|i| (0..STEPS_ADS+1).map(move |j| (i, j))).collect();

        let mut history = FnvHashMap::default();
        for k in &keys {
            history.insert(*k, 0.);
        }

        let key = keys.choose(rng).unwrap();
        Learner {
            params: *key,
            history: history
        }
    }

    pub fn learn(&mut self, reward: f32) {
        // TODO ensure reward is revenue/cost, not just revenue
        let v = self.history.get_mut(&self.params).unwrap();
        *v = ewma(reward, *v);
    }

    pub fn decide(&mut self, rng: &mut StdRng) {
        let keys: Vec<&ParamsKey> = self.history.keys().collect();
        let key = keys.choose_weighted(rng, |k| f32::max(0., *self.history.get(k).unwrap()) + 1.).unwrap();
        self.params = **key;
    }

    pub fn get_params(&self) -> Params {
        self.to_params(&self.params)
    }

    fn to_params(&self, key: &ParamsKey) -> Params {
        let (i, j) = key;
        let quality = MIN_QUALITY as f32 + (MAX_QUALITY as f32)/(STEPS_QUALITY as f32) * *i as f32;
        let ads = MIN_ADS as f32 + (MAX_ADS as f32)/(STEPS_ADS as f32) * *j as f32;
        (quality, ads)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_limited_queue() {
        let mut q = LimitedQueue::new(5);
        for i in 0..8 {
            q.push(i);
        }
        assert_eq!(q.len(), 5);
        let q_: Vec<usize> = q.iter().cloned().collect();
        assert_eq!(q_, vec![7,6,5,4,3]);
    }

    #[test]
    fn test_limited_set() {
        let mut q = LimitedSet::new(5);
        for i in 0..8 {
            q.insert(i);
        }
        assert_eq!(q.len(), 5);
        let q_: Vec<usize> = q.iter().cloned().collect();
        assert_eq!(q_, vec![7,6,5,4,3]);
    }

    #[test]
    fn test_learner() {
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut learner = Learner::new(&mut rng);
        for i in 0..2000 {
            let (x, y) = learner.get_params();

            // Mock reward function
            let x_r = (-(4.*x-2.).powf(2.)+4.); // Peak should be at x=0.5
            let y_r = (-(4.*y/MAX_ADS as f32-2.).powf(2.)+4.); // Peak should be at y=5.0
            let reward = x_r + y_r;

            learner.learn(reward);
            if i % 2 == 0 {
                learner.decide(&mut rng);
            }
        }

        // Hack to get around f32 comparisons
        let best = learner.history.keys()
            .max_by_key(|k| (learner.history.get(k).unwrap() * 1000.) as isize)
            .unwrap();

        // for (k, v) in learner.history.iter() {
        //     println!("{:?}: {:?}", learner.to_params(k), v);
        // }

        let best = learner.to_params(best);
        println!("best:{:?}", best);
        assert_eq!(best, (0.5, 5.0));
    }

    #[test]
    fn test_normal_range_mu_tight() {
        // Check that normal sampler is tight enough
        let mu = 1.;
        let max_distance = 0.1;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut count = 0;
        let total = 100;
        for _ in 0..total {
            let v = normal_range_mu_tight(mu, &mut rng);
            if (mu - v).abs() <= max_distance {
                count += 1;
            }
            // println!("v={:?}", v);
        }
        let p_acceptable = count as f32/total as f32;
        // println!("{:?}", p_acceptable);
        assert!(p_acceptable >= 0.95);
    }

    #[test]
    fn test_normal_p_mu_tight() {
        // Check that normal sampler is tight enough
        let mu = 0.5;
        let max_distance = 0.1;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut count = 0;
        let total = 100;
        for _ in 0..total {
            let v = normal_p_mu_tight(mu, &mut rng);
            if (mu - v).abs() <= max_distance {
                count += 1;
            }
            // println!("v={:?}", v);
        }
        let p_acceptable = count as f32/total as f32;
        // println!("{:?}", p_acceptable);
        assert!(p_acceptable >= 0.95);
    }
}
