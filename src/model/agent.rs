use fnv::{FnvHashMap, FnvHashSet};
use super::util::{Vector, VECTOR_SIZE};
use super::publisher::PublisherId;
use super::platform::PlatformId;
use super::content::{Content, ContentBody, SharedContent, SharerType};
use super::network::Network;
use super::config::SimulationConfig;
use rand::rngs::StdRng;
use rand::Rng;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;
use super::util;

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

    // Platforms the Agent is on
    pub platforms: FnvHashSet<PlatformId>,

    // Agent's estimate of how likely
    // a Publisher is to publish their content
    pub publishability: f32,
    pub publishabilities: FnvHashMap<PublisherId, f32>,

    // Track trust/feelings towards Publishers
    // TODO would like to not use a RefCell if possible
    pub publishers: RefCell<FnvHashMap<PublisherId, f32>>,

    // EWMA of trust
    // TODO would like to not use a RefCell if possible
    pub trust: RefCell<FnvHashMap<AgentId, f32>>,
}


pub fn random_values(rng: &mut StdRng) -> Values {
    // Normal dist, -1 to 1
    let v_vec = (0..VECTOR_SIZE)
        .map(|_| util::normal_range(rng))
        .collect();
    Values::from_vec(v_vec)
}

pub fn random_topics(rng: &mut StdRng) -> Topics {
    // Normal dist, 0 to 1
    let i_vec = (0..VECTOR_SIZE)
        .map(|_| util::normal_p(rng))
        .collect();
    Topics::from_vec(i_vec)
}

impl Agent {
    pub fn new(id: AgentId, mut rng: &mut StdRng) -> Agent {
        let resources = util::normal_p(&mut rng);

        Agent {
            id: id,
            interests: random_topics(&mut rng),
            values: Cell::new(random_values(&mut rng)),
            attention: 100.0, // TODO
            resources: resources,
            publishability: 1.,
            publishabilities: FnvHashMap::default(),
            publishers: RefCell::new(FnvHashMap::default()),
            subscriptions: RefCell::new(FnvHashSet::default()),
            trust: RefCell::new(FnvHashMap::default()),
            platforms: FnvHashSet::default()
        }
    }

    // Return content they create
    pub fn produce(&self, rng: &mut StdRng) -> Option<ContentBody> {
        // ENH: Take other factors into account for
        // when an agent produces.
        let p_produce = self.resources;
        let roll: f32 = rng.gen();

        if roll < p_produce {
            let topics = self.interests.map(|v| util::normal_p_mu(v, rng));
            let values = self.values.get().map(|v| util::normal_range_mu(v, rng));

            // ENH: Take other factors into account
            // Attention cost ranges from 0-100
            let cost = util::normal_p(rng) * 100.;

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
        content: Vec<(Option<&PlatformId>, &SharedContent)>,
        network: &Network,
        conf: &SimulationConfig,
        rng: &mut StdRng,
    ) -> (Vec<Rc<Content>>, (Vec<PublisherId>, Vec<PublisherId>), (FnvHashSet<AgentId>, FnvHashSet<AgentId>), FnvHashMap<PlatformId, f32>) {
        let mut attention = self.attention;
        let mut to_share = Vec::new();
        let mut values = self.values.get();
        let mut publishers = self.publishers.borrow_mut();
        let mut subscriptions = self.subscriptions.borrow_mut();
        let mut trusts = self.trust.borrow_mut();
        let mut new_subs = Vec::new();
        let mut unsubs = Vec::new();
        let mut unfollows = FnvHashSet::default();
        let mut follows = FnvHashSet::default();

        // Data generated for platforms
        let mut data = FnvHashMap::default();

        // ENH: Can make sure agents don't consume
        // content they've already consumed
        for (platform, sc) in content {
            let c = &sc.content;
            let affinity = self.interests.dot(&c.body.topics);
            let alignment = (values.dot(&c.body.values) - 0.5) * 2.;

            // Generate data for platform
            match platform {
                Some(p_id) => {
                    let val = data.entry(*p_id).or_insert(0.);
                    *val += conf.data_per_consume;
                },
                None => {}
            }

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
                    *v = util::ewma(affinity, *v);
                },
                None => {}
            }

            // Influence on Agent's values
            let trust = match sc.sharer {
                (SharerType::Agent, id) => {
                    // network.trust(&self.id, &id); // TODO this is redundant?

                    let mut trust = {
                        let trust = trusts.entry(id).or_insert(0.);
                        let old_trust = trust.clone(); // TODO meh
                        *trust = util::ewma(affinity * alignment, *trust);

                        // If platform is not None,
                        // we are already following the sharer.
                        // Decide to unfollow or not.
                        // TODO parameterize this
                        if *trust < 0.1 {
                            unfollows.insert(id);
                        }
                        old_trust
                    };

                    // Get author as well
                    if c.author != id {
                        let author_trust = trusts.entry(c.author).or_insert(0.);
                        trust = (trust + *author_trust)/2.;
                        *author_trust = util::ewma(affinity * alignment, *author_trust);

                        // For the author, we don't know
                        // if they're already following or not.
                        // TODO parameterize this
                        if trust < 0.1 {
                            unfollows.insert(id);
                        } else if trust > 0.9 {
                            follows.insert(id);
                        }
                    }

                    // TODO as it stands right now, Agents' new follows
                    // will only be authors. How do they encounter new sharers as well?
                    // There could be a "retweet" like function
                    // or we have random background changes to their offline networks
                    // that manifest as follows on platforms too

                    trust
                },
                (SharerType::Publisher, id) => {
                    publishers[&id]
                }
            };
            values.zip_apply(&c.body.values, |v, v_| {
                v + util::gravity(v, v_, conf.gravity_stretch, conf.max_influence) * affinity * trust
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

        (to_share, (new_subs, unsubs), (follows, unfollows), data)
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // TODO need to normalize?
        (self.interests.dot(&other.interests) + self.values.get().dot(&other.values.get())) / 2.
    }
}
