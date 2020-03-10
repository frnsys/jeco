use rand::Rng;
use rand::rngs::StdRng;
use std::f32::consts::E;
use rand_distr::StandardNormal;
use std::collections::VecDeque;
use fnv::FnvHashSet;
use std::hash::Hash;
use rand::seq::SliceRandom;
use bandit::{MultiArmedBandit, Identifiable, DEFAULT_BANDIT_CONFIG};
use bandit::softmax::{AnnealingSoftmax, AnnealingSoftmaxConfig};
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

static NORMAL_SCALE: f32 = 0.05;
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

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct LearnerArm {
    pub a: u32,
    pub b: u32
}

impl Identifiable for LearnerArm {
    fn ident(&self) -> String {
        format!("arm:{}:{}", self.a, self.b)
    }
}

#[derive(Debug)]
pub struct Learner {
    bandit: AnnealingSoftmax<LearnerArm>,
    pub arm: LearnerArm
}

impl Learner {
    pub fn new(mut rng: &mut StdRng) -> Learner {
        let arms: Vec<LearnerArm> = (0..10).flat_map(|i| (0..10).map(move |j| LearnerArm{a: i, b: j})).collect();
        let arm = arms.choose(&mut rng).unwrap().clone();
        Learner {
            bandit: AnnealingSoftmax::new(arms, DEFAULT_BANDIT_CONFIG.clone(), AnnealingSoftmaxConfig {
                cooldown_factor: 0.5
            }),
            arm: arm,
        }
    }

    pub fn learn(&mut self, reward: f64) {
        self.bandit.update(self.arm, reward);
        self.arm = self.bandit.select_arm();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use fnv::FnvHashMap;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn limited_queue() {
        let mut q = LimitedQueue::new(5);
        for i in 0..8 {
            q.push(i);
        }
        assert_eq!(q.len(), 5);
        let q_: Vec<usize> = q.iter().cloned().collect();
        assert_eq!(q_, vec![7,6,5,4,3]);
    }

    #[test]
    fn limited_set() {
        let mut q = LimitedSet::new(5);
        for i in 0..8 {
            q.insert(i);
        }
        assert_eq!(q.len(), 5);
        let q_: Vec<usize> = q.iter().cloned().collect();
        assert_eq!(q_, vec![7,6,5,4,3]);
    }

    #[test]
    fn learner() {
        let best_arm = LearnerArm{a: 5, b: 5};
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut learner = Learner::new(&mut rng);
        let mut arm_counts: FnvHashMap<LearnerArm, usize> = FnvHashMap::default();
        for _ in 0..1000 {
            if learner.arm == best_arm {
                learner.learn(1000.);
            } else {
                learner.learn(0.);
            }
            *arm_counts.entry(learner.arm).or_insert(0) += 1;
        }
        let (max_arm, _) = arm_counts.iter().fold((best_arm, 0), |(acc, c), (arm, count)| {
            if count > &c {
                (*arm, *count)
            } else {
                (acc, c)
            }
        });
        assert_eq!(best_arm, max_arm);

        let new_best_arm = LearnerArm{a: 0, b: 0};
        for _ in 0..5000 {
            if learner.arm == best_arm {
                learner.learn(-1000.);
            } else if learner.arm == new_best_arm {
                learner.learn(2000.);
            } else {
                learner.learn(0.);
            }
            *arm_counts.entry(learner.arm).or_insert(0) += 1;
        }
        let (new_max_arm, _) = arm_counts.iter().fold((best_arm, 0), |(acc, c), (arm, count)| {
            if count > &c {
                (*arm, *count)
            } else {
                (acc, c)
            }
        });
        assert_eq!(new_best_arm, new_max_arm);
    }

    #[test]
    fn normal_range_mu() {
        // Check that normal sampler is tight enough
        let mu = 1.;
        let max_distance = 0.1;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut count = 0;
        let total = 100;
        for _ in 0..total {
            let v = normal_range_mu(mu, &mut rng);
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
    fn normal_p_mu() {
        // Check that normal sampler is tight enough
        let mu = 0.5;
        let max_distance = 0.1;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut count = 0;
        let total = 100;
        for _ in 0..total {
            let v = normal_p_mu(mu, &mut rng);
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
