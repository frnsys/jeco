use rand::prelude::*;
use std::fmt::Debug;
use nalgebra::{VectorN, U8};
use super::content::Content;
use rand_distr::StandardNormal;

type Topics = VectorN<f32, U8>;
type Values = VectorN<f32, U8>;

#[derive(Debug)]
pub struct Agent {
    pub id: usize,
    pub interests: Topics,
    pub values: Values,
    pub attention: f32,
}

fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min { min }
    else if val > max { max }
    else { val }
}

impl Agent {
    pub fn new(id: usize) -> Agent {
        // Normal dist, -1 to 1
        let v_vec = (0..8).map(|_| {
            let mut val = thread_rng().sample(StandardNormal);
            val = (val - 0.5) * 2.;
            clamp(val, -1., 1.)
        }).collect();
        let values = Values::from_vec(v_vec);

        // Normal dist, 0 to 1
        let i_vec = (0..8).map(|_| {
            let val = thread_rng().sample(StandardNormal);
            clamp(val, 0., 1.)
        }).collect();
        let interests = Topics::from_vec(i_vec);

        Agent {
            id: id,
            interests: interests.normalize(),
            values: values.normalize(),
            attention: 100.0,
        }
    }

    // Return content they create
    pub fn produce(&self) -> Option<Content> {
        None
    }

    // Return content they decide to share
    pub fn consume(&self, content: Vec<&Content>) -> Vec<&Content> {
        // TODO gravity component
        vec![]
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // Assume these are normalized
        (self.interests.dot(&other.interests) + self.values.dot(&other.values))/2.
    }
}
