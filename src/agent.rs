use rand::prelude::*;
use std::rc::Rc;
use std::cell::Cell;
use std::fmt::Debug;
use nalgebra::{VectorN, U2};
use super::content::{Content, ContentBody, SharedContent};
use super::network::{Network};
use rand_distr::StandardNormal;

// 2 so can be plotted in 2d
static VECTOR_SIZE: u32 = 2;
pub type Topics = VectorN<f32, U2>;
pub type Values = VectorN<f32, U2>;

#[derive(Debug)]
pub struct Agent {
    pub id: usize,
    pub interests: Topics,
    pub values: Cell<Values>,
    pub attention: f32,
}

fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min { min }
    else if val > max { max }
    else { val }
}

static NORMAL_SCALE: f32 = 0.2;

pub fn random_values() -> Values {
    // Normal dist, -1 to 1
    let v_vec = (0..VECTOR_SIZE).map(|_| {
        let mut val: f32 = thread_rng().sample(StandardNormal);
        val *= NORMAL_SCALE;
        clamp(val, -1., 1.)
    }).collect();
    Values::from_vec(v_vec)
}

pub fn random_topics() -> Topics {
    // Normal dist, 0 to 1
    let i_vec = (0..VECTOR_SIZE).map(|_| {
        let mut val = thread_rng().sample(StandardNormal);
        val *= NORMAL_SCALE;
        val = (val + 0.5) * 2.;
        clamp(val, 0., 1.)
    }).collect();
    Topics::from_vec(i_vec)
}

// horizontal stretching of gravity function
// higher values mean weaker influence at greater distances
static GRAVITY_STRETCH: f32 = 100.;

// maximum movement amount
static MAX_INFLUENCE: f32 = 0.01;

// Returns how much a moves towards b
pub fn gravity(a: f32, b: f32) -> f32 {
    let mut dist = a - b;
    let sign = dist.signum();
    dist = dist.abs();
    if dist == 0. {
        // Already here, no movement
        0.
    } else {
        let strength = (1./dist)/GRAVITY_STRETCH;
        let movement = strength/(strength + 1.) * MAX_INFLUENCE;
        f32::min(movement, dist) * sign
    }
}

impl Agent {
    pub fn new(id: usize) -> Agent {
        Agent {
            id: id,
            interests: random_topics(),
            values: Cell::new(random_values()),
            attention: 100.0,
        }
    }

    // Return content they create
    pub fn produce(&self) -> Option<ContentBody> {
        // TODO calculate
        let p_produce = 0.25;
        let roll: f32 = thread_rng().gen();
        if roll < p_produce {
            // TODO calculate cost
            Some(ContentBody {
                cost: 15.,
                topics: random_topics(),
                values: random_values(),
            })
        } else {
            None
        }
    }

    // Return content they decide to share
    pub fn consume<'a>(&'a self, content: Vec<&'a SharedContent>, network: &Network) -> Vec<Rc<Content>> {
        let mut attention = self.attention;
        let mut to_share = Vec::new();
        // println!("CONSUMING: {:?}", content.len());
        for sc in content {
            let c = &sc.content;
            let mut values = self.values.get();
            let affinity = self.interests.dot(&c.body.topics);
            let alignment = (values.dot(&c.body.values) - 0.5) * 2.;

            // Take the abs value
            // So if something is very polar to the person's values,
            // they "hateshare" it
            let reactivity = affinity * alignment.abs();

            // Do they share it?
            let roll: f32 = thread_rng().gen();
            if roll < reactivity {
                to_share.push(c.clone());
            }

            // Influence
            let trust = network.trust(self, &sc.sharer);
            values.zip_apply(&c.body.values, |v, v_| v + gravity(v, v_) * affinity * trust);
            self.values.set(values);

            // Assume that they fully consume
            // the content, e.g. spend its
            // total attention cost
            attention -= c.body.cost;
            if attention <= 0. {
                break;
            }
        }

        to_share
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // TODO need to normalize?
        (self.interests.dot(&other.interests) + self.values.get().dot(&other.values.get()))/2.
    }
}
