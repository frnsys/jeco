use std::fmt::Debug;
use nalgebra::{VectorN, U8};
use super::content::Content;

type Topics = VectorN<f32, U8>;
type Values = VectorN<f32, U8>;

#[derive(Debug)]
pub struct Agent {
    pub id: usize,
    pub interests: Topics,
    pub values: Values,
    pub attention: f32,
}

impl Agent {
    pub fn new(id: usize) -> Agent {
        Agent {
            id: id,
            interests: Topics::new_random().normalize(),
            values: Values::new_random().normalize(),
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
