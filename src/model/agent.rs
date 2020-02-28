use fnv::{FnvHashMap, FnvHashSet};
use super::grid::{Position, hexagon_dist};
use super::util::{Vector, VECTOR_SIZE};
use super::publisher::PublisherId;
use super::platform::PlatformId;
use super::content::{Content, ContentId, ContentBody, SharedContent, SharerType};
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
    pub location: Position,
    pub relevancies: Vec<f32>, // Indexed by PublisherId

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
    // and steps since last seeing content from them
    // TODO would like to not use a RefCell if possible
    pub publishers: RefCell<FnvHashMap<PublisherId, (f32, usize)>>,

    // EWMA of trust
    // TODO would like to not use a RefCell if possible
    pub trust: RefCell<FnvHashMap<AgentId, f32>>,

    // An Agent's "reach" is the mean shared
    // count of their content per step
    pub reach: f32,

    // The content quality the Agent
    // aims for. Could be replaced with something
    // more sophisticated.
    pub quality: f32,

    // How many ads the Agent uses
    pub ads: f32,

    // Track most recent content
    pub content: util::LimitedQueue<Rc<Content>>,

    // Track recently encountered content
    pub seen_content: RefCell<util::LimitedSet<ContentId>>,

    // Params for estimating quality/ads mix
    theta: util::Params,
    observations: Vec<f32>,
    outcomes: Vec<f32>,
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
    pub fn new(id: AgentId, conf: &SimulationConfig, mut rng: &mut StdRng) -> Agent {
        let resources = util::normal_p(&mut rng);

        Agent {
            id: id,
            location: (0, 0),
            interests: random_topics(&mut rng),
            values: Cell::new(random_values(&mut rng)),
            reach: 100.,
            ads: rng.gen::<f32>() * 10.,
            quality: rng.gen::<f32>(),
            attention: conf.attention_budget,
            resources: resources,
            publishability: 1.,
            publishabilities: FnvHashMap::default(),
            publishers: RefCell::new(FnvHashMap::default()),
            subscriptions: RefCell::new(FnvHashSet::default()),
            trust: RefCell::new(FnvHashMap::default()),
            platforms: FnvHashSet::default(),
            content: util::LimitedQueue::new(10),
            seen_content: RefCell::new(util::LimitedSet::new(100)),
            theta: util::Params::new(rng.gen(), rng.gen()),
            observations: Vec::new(),
            outcomes: Vec::new(),
            relevancies: Vec::new()
        }
    }

    // Return content they create
    pub fn produce(&mut self, conf: &SimulationConfig, rng: &mut StdRng) -> Option<ContentBody> {
        if self.resources < self.quality { return None }

        // Agent produces depending on expected reach
        // and resources
        let p_produce = 0.1; // (self.resources + self.reach)/2.;
        let roll: f32 = rng.gen();

        if roll < p_produce {
            // Agent produces something around their own interests and values
            let topics = self.interests.map(|v| util::normal_p_mu(v, rng));
            let values = self.values.get().map(|v| util::normal_range_mu(v, rng));

            // ENH: Take other factors into account
            // Attention cost ranges from 0-100
            let cost = util::normal_p(rng) * conf.attention_budget;

            self.resources -= self.quality;

            Some(ContentBody {
                cost: cost,
                quality: self.quality,
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
        content: &Vec<(Option<&PlatformId>, &SharedContent)>,
        network: &Network,
        conf: &SimulationConfig,
        rng: &mut StdRng,
    ) -> (Vec<Rc<Content>>, (Vec<PublisherId>, Vec<PublisherId>), (FnvHashSet<AgentId>, FnvHashSet<AgentId>), FnvHashMap<PlatformId, f32>, FnvHashMap<(SharerType, usize), f32>) {
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
        let mut seen_publishers = FnvHashSet::default();
        let mut seen_content = self.seen_content.borrow_mut();

        // Data generated for platforms
        let mut data = FnvHashMap::default();

        // Ad revenue generated for publishers or agents
        let mut revenue = FnvHashMap::default();

        // ENH: Can make sure agents don't consume
        // content they've already consumed
        for (platform, sc) in content {
            let c = &sc.content;

            // Don't read own Content
            if c.author == self.id {
                continue;
            }

            // Requires too much attention
            if attention < c.body.cost {
                continue;
            }

            // Skip already-read content
            if seen_content.contains(&c.id) {
                continue;
            }
            seen_content.insert(c.id);

            let affinity = self.interests.dot(&c.body.topics);
            let alignment = (values.dot(&c.body.values) - 0.5) * 2.;

            // Generate data for platform
            match platform {
                Some(p_id) => {
                    let val = data.entry(**p_id).or_insert(0.);
                    *val += conf.data_per_consume;
                },
                None => {}
            }

            // Take the abs value
            // So if something is very polar to the person's values,
            // they "hateshare" it
            let mut reactivity = affinity * alignment.abs() * f32::max(c.body.quality, 1.);

            // Update publisher feeling/reputation
            // and collect ad revenue
            match c.publisher {
                Some(p_id) => {
                    let (v, _) = publishers.entry(p_id).or_insert((0.5, 0));
                    *v = util::ewma(affinity, *v);

                    seen_publishers.insert(p_id);

                    if c.ads > 0. {
                        revenue.insert((SharerType::Publisher, p_id), c.ads * conf.revenue_per_ad);
                    }

                    let relevancy = self.relevancies[p_id];
                    reactivity *= relevancy;
                },
                None => {
                    if c.ads > 0. {
                        revenue.insert((SharerType::Agent, c.author), c.ads * conf.revenue_per_ad);
                    }

                    // TODO? Can't access author location so
                    // kind of complicated
                    // Made distance less important here
                    // let dist = hexagon_dist(&self.location, &c.author);
                    // let relevance = 1. - util::sigmoid((dist-4) as f32);
                    // reactivity *= relevancy;
                }
            }

            // Do they share it?
            let roll: f32 = rng.gen();
            if roll < reactivity {
                to_share.push(c.clone());
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
                        if *trust < conf.unfollow_trust {
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
                        if trust < conf.unfollow_trust {
                            unfollows.insert(id);
                        } else if trust > conf.follow_trust {
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
                    publishers[&id].0/c.ads
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

        // Update which Publishers we've seen recently
        for &p_id in subscriptions.iter() {
            let (_, last_seen) = publishers.entry(p_id).or_insert((0.5, 0));
            if seen_publishers.contains(&p_id) {
                *last_seen = 0;
            } else {
                *last_seen += 1;
            }
        }

        // Decide on subscriptions
        // TODO consider costs of subscription
        for (p_id, (affinity, last_seen)) in publishers.iter() {
            if last_seen >= &conf.unsubscribe_lag {
                if subscriptions.contains(p_id) {
                    subscriptions.remove(p_id);
                    unsubs.push(*p_id);
                }
            } else if affinity > &conf.subscribe_trust {
                if !subscriptions.contains(p_id) {
                    subscriptions.insert(*p_id);
                    new_subs.push(*p_id);
                }
            } else if affinity < &conf.unsubscribe_trust {
                if subscriptions.contains(p_id) {
                    subscriptions.remove(p_id);
                    unsubs.push(*p_id);
                }
            }
        }

        (to_share, (new_subs, unsubs), (follows, unfollows), data, revenue)
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // TODO need to normalize?
        (self.interests.dot(&other.interests) + self.values.get().dot(&other.values.get())) / 2.
    }

    pub fn n_shares(&self) -> Vec<usize> {
        // -1 to account for reference in self.content
        // -1 to account for reference in Sim's self.content
        self.content.iter().map(|c| Rc::strong_count(c) - 2).collect()
    }

    pub fn update_reach(&mut self) {
        let shares = self.n_shares();
        if shares.len() == 0 {
            self.reach = 0.;
        } else {
            let mean_shares = shares.iter().fold(0, |acc, v| acc + v) as f32 / shares.len() as f32;
            self.reach = util::ewma(mean_shares, self.reach);
        }
    }

    pub fn learn(&mut self, revenue: f32, change_rate: f32) {
        // Assume reach has been updated
        self.outcomes.push(revenue * self.reach); // TODO more balanced mixture of the two?

        self.observations.push(self.quality);
        self.observations.push(self.ads);

        // TODO don't necessarily need to learn _every_ step.
        self.theta = util::learn_steps(&self.observations, &self.outcomes, self.theta);
        let steps: Vec<f32> = self.theta.into_iter().cloned().collect();
        self.quality += change_rate * steps[0];
        self.ads += change_rate * steps[1];
        self.ads = f32::max(0., self.ads);
        self.quality = f32::min(f32::max(0., self.quality), self.resources);
    }
}
