use super::publisher::PublisherId;
use super::content::{Content, ContentBody, SharedContent, SharerType};
use super::network::Network;
use nalgebra::{VectorN, U2};
use rand::rngs::StdRng;
use rand::Rng;
use rand_distr::StandardNormal;
use std::cell::Cell;
use std::fmt::Debug;
use std::rc::Rc;

// 2 so can be plotted in 2d
static VECTOR_SIZE: u32 = 2;
pub type Topics = VectorN<f32, U2>;
pub type Values = VectorN<f32, U2>;

pub type AgentId = usize;

#[derive(Debug)]
pub struct Agent {
    pub id: AgentId,
    pub interests: Topics,
    pub values: Cell<Values>,
    pub attention: f32,
    pub resources: f32,
    pub subscriptions: Vec<PublisherId>,
}

fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

static NORMAL_SCALE: f32 = 0.8;

pub fn random_values(rng: &mut StdRng) -> Values {
    // Normal dist, -1 to 1
    let v_vec = (0..VECTOR_SIZE)
        .map(|_| {
            let mut val: f32 = rng.sample(StandardNormal);
            val *= NORMAL_SCALE;
            clamp(val, -1., 1.)
        })
        .collect();
    Values::from_vec(v_vec)
}

pub fn random_topics(rng: &mut StdRng) -> Topics {
    // Normal dist, 0 to 1
    let i_vec = (0..VECTOR_SIZE)
        .map(|_| {
            let mut val = rng.sample(StandardNormal);
            val = (val + 0.5) * 2.;
            val *= NORMAL_SCALE;
            clamp(val, 0., 1.)
        })
        .collect();
    Topics::from_vec(i_vec)
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

impl Agent {
    pub fn new(id: AgentId, mut rng: &mut StdRng) -> Agent {
        let mut resources = rng.sample(StandardNormal);
        resources = (resources + 0.5) * 2.;
        resources = clamp(resources, 0., 1.);

        Agent {
            id: id,
            interests: random_topics(&mut rng),
            values: Cell::new(random_values(&mut rng)),
            attention: 100.0,
            resources: resources,
            subscriptions: Vec::new()
        }
    }

    // Return content they create
    pub fn produce(&self, rng: &mut StdRng) -> Option<ContentBody> {
        let p_produce = self.resources;
        let roll: f32 = rng.gen();

        if roll < p_produce {
            let topics = self.interests.map(|v| {
                let mut val = rng.sample(StandardNormal);
                val += v;
                val *= NORMAL_SCALE;
                clamp(val, 0., 1.)
            });

            let values = self.values.get().map(|v| {
                let mut val = rng.sample(StandardNormal);
                val += v;
                val *= NORMAL_SCALE;
                clamp(val, -1., 1.)
            });

            // Attention cost ranges from 0-100
            let mut cost = rng.sample(StandardNormal);
            cost = (cost + 0.5) * 2.;
            cost = clamp(cost, 0., 1.);
            cost *= 100.;

            Some(ContentBody {
                cost: cost,
                topics: topics,
                values: values,
            })
        } else {
            None
        }
    }

    // Return content they decide to share
    pub fn consume<'a>(
        &'a self,
        content: Vec<&'a SharedContent>,
        network: &Network,
        gravity_stretch: f32,
        max_influence: f32,
        rng: &mut StdRng,
    ) -> Vec<Rc<Content>> {
        let mut attention = self.attention;
        let mut to_share = Vec::new();
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
            let roll: f32 = rng.gen();
            if roll < reactivity {
                to_share.push(c.clone());
            }

            // Influence
            let trust = match sc.sharer {
                (SharerType::Agent, id) => {
                    network.trust(&self.id, &id)
                },
                (SharerType::Publisher, id) => {
                    1. // TODO
                }
            };
            values.zip_apply(&c.body.values, |v, v_| {
                v + gravity(v, v_, gravity_stretch, max_influence) * affinity * trust
            });
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
        (self.interests.dot(&other.interests) + self.values.get().dot(&other.values.get())) / 2.
    }
}
