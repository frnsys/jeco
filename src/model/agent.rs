use fnv::{FnvHashMap, FnvHashSet};
use super::grid::{Position};
use super::util::{Vector, VECTOR_SIZE, Learner};
use super::publisher::PublisherId;
use super::platform::PlatformId;
use super::content::{Content, ContentId, ContentBody, SharedContent, SharerType};
use super::config::{SimulationConfig, AgentConfig};
use super::motive::Motive;
use rand::rngs::StdRng;
use rand::Rng;
use std::fmt::Debug;
use std::sync::Arc;
use super::util;

pub type Topics = Vector;
pub type Values = Vector;
pub type AgentId = usize;

#[derive(Debug)]
pub struct Agent {
    pub id: AgentId,
    pub interests: Topics,
    pub values: Values,
    pub motive: Motive,
    pub attention_budget: f32,
    pub resources: f32,
    pub expenses: f32,
    pub media_literacy: f32,
    pub location: Position,
    pub relevancies: Vec<f32>, // Indexed by PublisherId

    // Publishers the Agent is subscribed to
    pub subscriptions: FnvHashSet<PublisherId>,

    // Agent's estimate of how likely
    // a Publisher is to publish their content
    pub publishability: f32,
    pub publishabilities: FnvHashMap<PublisherId, f32>,

    // Track trust/feelings towards Publishers
    // and steps since last seeing content from them
    pub publishers: FnvHashMap<PublisherId, (f32, usize)>,

    // EWMA of trust
    // TODO would like to not use a RefCell if possible
    pub trust: FnvHashMap<AgentId, f32>,

    // An Agent's "reach" is the mean shared
    // count of their content per step
    pub reach: f32,

    // The content quality the Agent
    // aims for. Could be replaced with something
    // more sophisticated.
    pub depth: f32,
    pub spectacle: f32,

    // How many ads the Agent uses
    pub ads: f32,

    // How long the Agent' pieces are
    pub attention: f32,

    // Track most recent content
    pub content: util::LimitedQueue<Arc<Content>>,

    // Track recently encountered content
    pub seen_content: util::LimitedSet<ContentId>,

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
    pub fn new(id: AgentId, conf: &AgentConfig, mut rng: &mut StdRng) -> Agent {
        let resources = util::normal_p(&mut rng);
        let learner = Learner::new(&mut rng);
        let params = learner.get_params();

        Agent {
            id: id,
            location: (0, 0),
            interests: random_topics(&mut rng),
            values: random_values(&mut rng),
            motive: rng.gen(),
            reach: 100.,
            depth: params[0],
            spectacle: params[1],
            ads: params[2],
            attention: params[3],
            learner: learner,
            attention_budget: conf.attention_budget,
            resources: resources * 100.,
            expenses: 0.,
            media_literacy: util::normal_p(&mut rng),
            publishability: 1.,
            publishabilities: FnvHashMap::default(),
            publishers: FnvHashMap::default(),
            subscriptions: FnvHashSet::default(),
            trust: FnvHashMap::default(),
            content: util::LimitedQueue::new(10),
            seen_content: util::LimitedSet::new(100),
            relevancies: Vec::new()
        }
    }

    pub fn produce(&self, max_attention: f32, rng: &mut StdRng) -> ContentBody {
        // Agent produces something around their own interests and values
        let topics = self.interests.map(|v| util::normal_p_mu_tight(v, rng));
        let values = self.values.map(|v| util::normal_range_mu_tight(v, rng));

        // ENH: Take other factors into account
        // Attention cost ranges from 0-10
        let attn_cost = util::normal_p(rng) * max_attention;

        ContentBody {
            cost: attn_cost,
            depth: self.depth,
            spectacle: self.spectacle,
            topics: topics,
            values: values,
        }
    }

    // Return content they create
    pub fn try_produce(&mut self, population: usize, conf: &SimulationConfig, rng: &mut StdRng) -> Option<ContentBody> {
        let cost = (self.depth + self.spectacle) * conf.cost_per_quality;
        if self.resources < cost { return None }

        // Agent produces depending on expected reach
        // and resources
        let roll: f32 = rng.gen();
        if roll < p_produce(self.reach/population as f32) {
            let body = self.produce(conf.agent.attention_budget, rng);
            self.resources -= cost;
            self.expenses += cost;

            Some(body)
        } else {
            None
        }
    }

    // Return content they decide to share
    pub fn consume(
        &mut self,
        content: &Vec<(Option<&PlatformId>, &SharedContent)>,
        conf: &SimulationConfig,
        rng: &mut StdRng
    ) -> (Vec<Arc<Content>>, (Vec<PublisherId>, Vec<PublisherId>), (FnvHashSet<AgentId>, FnvHashSet<AgentId>), FnvHashMap<PlatformId, f32>, FnvHashMap<(SharerType, usize), f32>) {
        let mut attention = self.attention_budget;
        let mut to_share = Vec::new();
        let mut new_subs = Vec::new();
        let mut unsubs = Vec::new();
        let mut unfollows = FnvHashSet::default();
        let mut follows = FnvHashSet::default();
        let mut seen_publishers = FnvHashSet::default();

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
            if self.seen_content.contains(&c.id) {
                continue;
            }
            self.seen_content.insert(c.id);

            let affinity = similarity(&self.interests, &c.body.topics);
            let align = alignment(&self.values, &c.body.values);
            let appeal = (self.media_literacy * c.body.depth) + ((1. - self.media_literacy) * c.body.spectacle);
            let mut react = reactivity(affinity, align, appeal);

            // Update publisher feeling/reputation/trust
            // and collect ad revenue
            match c.publisher {
                Some(p_id) => {
                    let relevancy = self.relevancies[p_id];
                    let (v, _) = self.publishers.entry(p_id).or_insert((conf.default_trust, 0));
                    // println!("update: {:?} {:?} {:?} {:?} {:?}", p_id, update_trust(affinity * relevancy, align)/(c.ads + 1.), affinity, relevancy, align);
                    // *v = f32::max(0., util::ewma(update_trust(affinity * relevancy, align)/(c.ads + 1.), *v));
                    // println!("af:{:?} re:{:?} al:{:?}", affinity, relevancy, align);
                    *v = f32::max(0., util::ewma(update_trust((affinity+relevancy)/2., align)/(c.ads/10. + 1.), *v));
                    // *v = f32::max(0., util::ewma(1., *v));

                    seen_publishers.insert(p_id);

                    if c.ads > 0. {
                        revenue.insert((SharerType::Publisher, p_id), c.ads * conf.revenue_per_ad);
                    }

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
                        let trust = self.trust.entry(id).or_insert(conf.default_trust);
                        let old_trust = trust.clone(); // TODO meh
                        // println!("update: {:?} {:?} {:?} {:?}", id, update_trust(affinity, align), affinity, align);
                        *trust = f32::max(0., util::ewma(update_trust(affinity, align), *trust));

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
                        let author_trust = self.trust.entry(c.author).or_insert(conf.default_trust);
                        trust = (trust + *author_trust)/2.;
                        *author_trust = f32::max(0., util::ewma(update_trust(affinity, align)/(c.ads + 1.), *author_trust));

                        // For the author, we don't know
                        // if they're already following or not.
                        if *author_trust < conf.unfollow_trust {
                            unfollows.insert(c.author);
                        } else if *author_trust > conf.follow_trust {
                            follows.insert(c.author);
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
                    self.publishers[&id].0
                }
            };
            // println!("affinity: {:?}, trust: {:?}", affinity, trust);
            self.be_influenced(&c.body.values, conf.gravity_stretch, conf.max_influence, affinity * trust);

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
        for &p_id in self.subscriptions.iter() {
            let (_, last_seen) = self.publishers.entry(p_id).or_insert((conf.default_trust, 0));
            if seen_publishers.contains(&p_id) {
                *last_seen = 0;
            } else {
                *last_seen += 1;
            }
        }

        // Decide on subscriptions
        // TODO consider costs of subscription
        for (p_id, (affinity, last_seen)) in self.publishers.iter() {
            // println!("{:?} {:?}", p_id, affinity);
            if last_seen >= &conf.unsubscribe_lag {
                if self.subscriptions.contains(p_id) {
                    self.subscriptions.remove(p_id);
                    unsubs.push(*p_id);
                }
            } else if affinity > &conf.subscribe_trust {
                if !self.subscriptions.contains(p_id) {
                    self.subscriptions.insert(*p_id);
                    new_subs.push(*p_id);
                }
            } else if affinity < &conf.unsubscribe_trust {
                if self.subscriptions.contains(p_id) {
                    self.subscriptions.remove(p_id);
                    unsubs.push(*p_id);
                }
            }
        }

        (to_share, (new_subs, unsubs), (follows, unfollows), data, revenue)
    }

    pub fn be_influenced(&mut self, other: &Values, gravity_stretch: f32, max_influence: f32, trust: f32) {
        self.values.zip_apply(other, |a_v, c_v| {
            a_v + util::gravity(a_v, c_v, gravity_stretch, max_influence) * trust
        });
    }

    pub fn similarity(&self, other: &Agent) -> f32 {
        // TODO need to normalize?
        (self.interests.dot(&other.interests) + self.values.dot(&other.values)) / 2.
    }

    pub fn n_shares(&self) -> Vec<usize> {
        self.content.iter().map(|c| Arc::strong_count(c)).collect()
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

    pub fn learn(&mut self, revenue: f32, update: bool, rng: &mut StdRng) {
        // Assume reach has been updated
        // Assume expenses are correct
        let profit = revenue - self.expenses;
        let reward = match self.motive {
            Motive::Profit => profit,
            Motive::Civic => self.reach * self.depth + f32::min(0., profit),
            Motive::Influence => self.reach + f32::min(0., profit),
        };
        self.learner.learn(reward);
        if update {
            self.learner.decide(rng);
            let params = self.learner.get_params();
            self.depth = params[0];
            self.spectacle = params[1];
            self.ads = params[2];
            self.attention = params[3];
        }
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

pub fn update_trust(affinity: f32, alignment: f32) -> f32 {
    affinity * alignment
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
        let a = Values::from_vec(vec![0., 0.]);
        let b = Values::from_vec(vec![1., 1.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 0.);

        let a = Values::from_vec(vec![0., 0.]);
        let b = Values::from_vec(vec![-1., -1.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 0.);

        let a = Values::from_vec(vec![0., 0.]);
        let b = Values::from_vec(vec![0., 0.]);
        let align = alignment(&a, &b);
        assert_eq!(align, 1.);

        let a = Values::from_vec(vec![1., 1.]);
        let b = Values::from_vec(vec![-1., -1.]);
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
        // Strong affinity and strong alignment
        assert!(update_trust(1., 1.) > 0.);

        // Strong affinity and opposite alignment
        assert!(update_trust(1., -1.) < 0.);

        // Weak affinity and strong alignment
        assert!(update_trust(0., 1.) == 0.);

        // Weak affinity and opposite alignment
        assert!(update_trust(0., -1.) == 0.);
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
