use fnv::{FnvHashMap, FnvHashSet};
use super::grid::{Position};
use super::util::{Vector, VECTOR_SIZE, Learner};
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
    learner: Learner,
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
        let learner = Learner::new(&mut rng);

        Agent {
            id: id,
            location: (0, 0),
            interests: random_topics(&mut rng),
            values: Cell::new(random_values(&mut rng)),
            reach: 100.,
            ads: learner.arm.a as f32,
            quality: learner.arm.b as f32,
            learner: learner,
            attention: conf.attention_budget,
            resources: resources * 100.,
            publishability: 1.,
            publishabilities: FnvHashMap::default(),
            publishers: RefCell::new(FnvHashMap::default()),
            subscriptions: RefCell::new(FnvHashSet::default()),
            trust: RefCell::new(FnvHashMap::default()),
            platforms: FnvHashSet::default(),
            content: util::LimitedQueue::new(10),
            seen_content: RefCell::new(util::LimitedSet::new(100)),
            relevancies: Vec::new()
        }
    }

    // Return content they create
    pub fn produce(&mut self, conf: &SimulationConfig, rng: &mut StdRng) -> Option<ContentBody> {
        let cost = self.quality * conf.cost_per_quality;
        if self.resources < cost { return None }

        // Agent produces depending on expected reach
        // and resources
        let roll: f32 = rng.gen();
        if roll < p_produce(self.reach/conf.population as f32) {
            // Agent produces something around their own interests and values
            let topics = self.interests.map(|v| util::normal_p_mu(v, rng));
            let values = self.values.get().map(|v| util::normal_range_mu(v, rng));

            // ENH: Take other factors into account
            // Attention cost ranges from 0-100
            let attn_cost = util::normal_p(rng) * conf.attention_budget;

            self.resources -= cost;

            Some(ContentBody {
                cost: attn_cost,
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

            let affinity = similarity(&self.interests, &c.body.topics);
            let align = alignment(&values, &c.body.values);
            let mut react = reactivity(affinity, align, c.body.quality);

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
                    react *= relevancy;
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
                    // react *= relevancy;
                }
            }

            // Do they share it?
            let roll: f32 = rng.gen();
            if roll < react {
                to_share.push(c.clone());
            }

            // Influence on Agent's values
            let trust = match sc.sharer {
                (SharerType::Agent, id) => {
                    // network.trust(&self.id, &id); // TODO this is redundant?

                    let mut trust = {
                        let trust = trusts.entry(id).or_insert(0.);
                        let old_trust = trust.clone(); // TODO meh
                        *trust = update_trust(*trust, affinity, align);

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
                        *author_trust = update_trust(*author_trust, affinity, align);

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
            values.zip_apply(&c.body.values, |a_v, c_v| {
                a_v + util::gravity(a_v, c_v, conf.gravity_stretch, conf.max_influence) * affinity * trust
            });
            self.values.set(values);

            // Generate data for platform
            match platform {
                Some(p_id) => {
                    let val = data.entry(**p_id).or_insert(0.);
                    *val += conf.data_per_consume;
                },
                None => {}
            }

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

    pub fn learn(&mut self, revenue: f32) {
        // Assume reach has been updated
        // TODO more balanced mixture of the two?
        let outcome = revenue * self.reach;
        self.learner.learn(outcome as f64);
        self.ads = self.learner.arm.a as f32;
        self.quality = self.learner.arm.b as f32;
    }
}

pub fn distance(a: &Vector, b: &Vector) -> f32 {
    ((a.x - b.x).powf(2.) + (a.y - b.y).powf(2.)).sqrt()
}

static MAX_TOPIC_DISTANCE: f32 = 1.4142135623730951; // sqrt(2.)
pub fn similarity(a: &Vector, b: &Vector) -> f32 {
    1. - distance(a, b)/MAX_TOPIC_DISTANCE
}

static MAX_VALUE_DISTANCE: f32 = 2.8284271247461903; // sqrt(8.)
pub fn alignment(a: &Vector, b: &Vector) -> f32 {
    ((1. - distance(a, b)/MAX_VALUE_DISTANCE) - 0.5) * 2.
}

static EASE_OF_TRUST: f32 = 1./100.;
pub fn update_trust(trust: f32, affinity: f32, alignment: f32) -> f32 {
    trust + affinity * alignment * EASE_OF_TRUST
}

pub fn reactivity(affinity: f32, alignment: f32, quality: f32) -> f32 {
    // Take the abs value
    // So if something is very polar to the person's values,
    // they "hateshare" it
    affinity * alignment.abs() * f32::min(quality, 1.)
}

pub fn p_produce(p_reach: f32) -> f32 {
    util::sigmoid(18.*(p_reach-0.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let mut a = Values::from_vec(vec![0., 0.]);
        let mut b = Values::from_vec(vec![1., 1.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 0.);

        a = Values::from_vec(vec![0., 0.]);
        b = Values::from_vec(vec![-1., -1.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 0.);

        let mut a = Values::from_vec(vec![0., 0.]);
        let mut b = Values::from_vec(vec![0., 0.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 1.);

        let mut a = Values::from_vec(vec![1., 1.]);
        let mut b = Values::from_vec(vec![-1., -1.]);
        let align = alignment(&a, &b);
        assert_eq!(align, -1.);
    }

    #[test]
    fn test_similarity() {
        let mut a = Topics::from_vec(vec![0., 0.]);
        let mut b = Topics::from_vec(vec![1., 1.]);
        let sim = similarity(&a, &b);
        assert_eq!(sim, 0.);

        a = Topics::from_vec(vec![1., 1.]);
        b = Topics::from_vec(vec![1., 1.]);
        let sim = similarity(&a, &b);
        assert_eq!(sim, 1.);

        a = Topics::from_vec(vec![0., 0.]);
        b = Topics::from_vec(vec![0., 0.]);
        let sim = similarity(&a, &b);
        assert_eq!(sim, 1.);
    }

    #[test]
    fn test_update_trust() {
        let trust = 0.;

        // Strong affinity and strong alignment
        assert!(update_trust(trust, 1., 1.) > trust);

        // Strong affinity and opposite alignment
        assert!(update_trust(trust, 1., -1.) < trust);

        // Weak affinity and strong alignment
        assert!(update_trust(trust, 0., 1.) == trust);

        // Weak affinity and opposite alignment
        assert!(update_trust(trust, 0., -1.) == trust);
    }

    #[test]
    fn test_reactivity() {
        // Strong affinity, strong alignment, high quality
        assert_eq!(reactivity(1., 1., 1.), 1.);

        // Strong affinity, opposite alignment, high quality
        assert_eq!(reactivity(1., -1., 1.), 1.);

        // Weak affinity, strong alignment, high quality
        assert_eq!(reactivity(0., 1., 1.), 0.);

        // Weak affinity, opposite alignment, high quality
        assert_eq!(reactivity(0., -1., 1.), 0.);

        // Strong affinity, strong alignment, low quality
        assert_eq!(reactivity(1., 1., 0.), 0.);

        // Strong affinity, opposite alignment, low quality
        assert_eq!(reactivity(1., -1., 0.), 0.);

        // Weak affinity, strong alignment, low quality
        assert_eq!(reactivity(0., 1., 0.), 0.);

        // Weak affinity, opposite alignment, low quality
        assert_eq!(reactivity(0., -1., 0.), 0.);
    }

    #[test]
    fn test_gravity() {
        let gravity_stretch = 100.;
        let max_influence = 0.1;

        let mut to_val = 1.;
        let mut from_val = 0.9;
        let gravity_closer = util::gravity(from_val, to_val, gravity_stretch, max_influence);
        assert!(gravity_closer > 0.);

        from_val = 0.8;
        let gravity_further = util::gravity(from_val, to_val, gravity_stretch, max_influence);
        assert!(gravity_closer > gravity_further);

        to_val = -1.;
        let gravity = util::gravity(from_val, to_val, gravity_stretch, max_influence);
        assert!(gravity < 0.);
    }


    #[test]
    fn test_p_produce() {
        let mut p = p_produce(0.);
        assert!(p < 0.1 && p > 0.);

        p = p_produce(0.2);
        assert_eq!(p, 0.5);

        p = p_produce(0.5);
        assert!(p < 1. && p > 0.95);
    }
}
