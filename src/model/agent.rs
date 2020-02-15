use fnv::{FnvHashMap, FnvHashSet};
use super::util::{Vector, VECTOR_SIZE};
use super::publisher::PublisherId;
use super::content::{Content, ContentBody, SharedContent, SharerType};
use super::network::Network;
use super::config::SimulationConfig;
use rand::rngs::StdRng;
use rand::Rng;
use rand_distr::StandardNormal;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;
use super::util::{ewma, clamp, gravity};

pub type Topics = Vector;
pub type Values = Vector;
pub type AgentId = usize;

#[derive(Debug)]
pub struct Agent {
    pub id: AgentId,
    pub interests: Topics,
    pub values: Cell<Values>,
    pub attention: f32,
    pub resources: f32,

    // Publishers the Agent is subscribed to
    // TODO would like to not use a RefCell if possible
    pub subscriptions: RefCell<FnvHashSet<PublisherId>>,

    // Agent's estimate of how likely
    // a Publisher is to publish their content
    pub publishability: f32,
    pub publishabilities: FnvHashMap<PublisherId, f32>,

    // Track trust/feelings towards Publishers
    // TODO would like to not use a RefCell if possible
    pub publishers: RefCell<FnvHashMap<PublisherId, f32>>,
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

impl Agent {
    pub fn new(id: AgentId, mut rng: &mut StdRng) -> Agent {
        let mut resources = rng.sample(StandardNormal);
        resources = (resources + 0.5) * 2.;
        resources = clamp(resources, 0., 1.);

        Agent {
            id: id,
            interests: random_topics(&mut rng),
            values: Cell::new(random_values(&mut rng)),
            attention: 100.0, // TODO
            resources: resources,
            publishability: 1.,
            publishabilities: FnvHashMap::default(),
            publishers: RefCell::new(FnvHashMap::default()),
            subscriptions: RefCell::new(FnvHashSet::default())
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
        conf: &SimulationConfig,
        rng: &mut StdRng,
    ) -> (Vec<Rc<Content>>, (Vec<PublisherId>, Vec<PublisherId>)) {
        let mut attention = self.attention;
        let mut to_share = Vec::new();
        let mut values = self.values.get();
        let mut publishers = self.publishers.borrow_mut();
        let mut subscriptions = self.subscriptions.borrow_mut();
        let mut new_subs = Vec::new();
        let mut unsubs = Vec::new();

        // ENH: Can make sure agents don't consume
        // content they've already consumed
        for sc in content {
            let c = &sc.content;
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

            // Update publisher feeling/reputation
            match c.publisher {
                Some(p_id) => {
                    let v = publishers.entry(p_id).or_insert(0.5);
                    *v = ewma(affinity, *v);
                },
                None => {}
            }

            // Influence on Agent's values
            let trust = match sc.sharer {
                (SharerType::Agent, id) => {
                    network.trust(&self.id, &id)
                },
                (SharerType::Publisher, id) => {
                    1. // TODO
                }
            };
            values.zip_apply(&c.body.values, |v, v_| {
                v + gravity(v, v_, conf.gravity_stretch, conf.max_influence) * affinity * trust
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

        // Decide on subscriptions
        // TODO consider costs of subscription
        for (p_id, affinity) in publishers.iter() {
            let roll: f32 = rng.gen();
            if !subscriptions.contains(p_id) {
                let p = affinity * conf.subscription_prob_weight;
                if roll < p {
                    subscriptions.insert(*p_id);
                    new_subs.push(*p_id);
                }
            } else {
                let p = (1. - affinity) * conf.subscription_prob_weight;
                if roll < p {
                    subscriptions.remove(p_id);
                    unsubs.push(*p_id);
                }
            }
        }

        (to_share, (new_subs, unsubs))
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // TODO need to normalize?
        (self.interests.dot(&other.interests) + self.values.get().dot(&other.values.get())) / 2.
    }
}
