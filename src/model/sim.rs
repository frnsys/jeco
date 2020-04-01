use rand::Rng;
use fnv::{FnvHashMap, FnvHashSet};
use super::agent::{Agent, AgentId};
use super::policy::Policy;
use super::network::Network;
use super::platform::{Platform, PlatformId};
use super::publisher::Publisher;
use super::grid::{HexGrid, Position, hexagon_dist};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use super::content::{Content, ContentId, SharedContent, SharerType};
use super::util::{ewma, sigmoid};
use super::config::SimulationConfig;
use itertools::Itertools;
use rand_distr::{Distribution, Beta, Binomial};
use std::sync::Arc;
use std::cmp::max;

static MAX_FRIENDS: usize = 120;

pub struct Simulation {
    pub network: Network,
    pub agents: Vec<Agent>,
    content: Vec<Arc<Content>>,
    pub publishers: Vec<Publisher>,
    pub platforms: Vec<Platform>,
    pub ref_grid: HexGrid,
    pub grid: FnvHashMap<Position, Vec<AgentId>>,
    pub distances: FnvHashMap<Position, Vec<usize>>,

    // Stats
    pub n_produced: usize,
    pub n_pitched: usize,
    pub n_published: usize,

    // Policies
    advertising_tax: f32,
    subsidy: f32,

    // Content Agents will share in the next step.
    // Emptied each step.
    share_queues: Vec<Vec<SharedContent>>,

    // Store content the Publisher will
    // publish in the next step. Emptied each step.
    outboxes: Vec<Vec<SharedContent>>,

    // Agents and the platforms they're on
    agent_platforms: Vec<FnvHashSet<PlatformId>>,
}


impl Simulation {
    pub fn new(conf: &SimulationConfig, mut rng: &mut StdRng) -> Simulation {
        let mut agents: Vec<Agent> = (0..conf.population)
            .map(|i| Agent::new(i, &conf.agent, &mut rng))
            .collect();

        let mut agent_platforms = Vec::new();
        let mut share_queues = Vec::new();
        for _ in &agents {
            agent_platforms.push(FnvHashSet::default());
            share_queues.push(Vec::new());
        }

        let mut publishers: Vec<Publisher> = conf.publishers.iter()
            .enumerate()
            .map(|(i, sconf)| Publisher::from_config(i, &sconf, &conf.publisher, &mut rng))
            .collect();

        let more_publishers: Vec<Publisher> = (0..(conf.n_publishers - publishers.len()).max(0))
            .map(|i| Publisher::new(i+publishers.len(), &conf.publisher, &mut rng))
            .collect();
        publishers.extend(more_publishers);

        let mut outboxes = Vec::new();
        for _ in &publishers {
            outboxes.push(Vec::new());
        }

        let platforms: Vec<Platform> = (0..conf.n_platforms)
            .map(|i| Platform::new(i))
            .collect();

        let mut network = Network::new();
        network.preferential_attachment(&agents, MAX_FRIENDS, &mut rng);

        let ref_grid = HexGrid::new(conf.grid_size, conf.grid_size);
        let mut grid = FnvHashMap::default();
        for pos in ref_grid.positions() {
            grid.insert(pos, Vec::new());
        }

        distribute_agents(&mut agents, &mut grid, rng);

        // Randomly assign publishers by density
        let mut already_occupied: Vec<Position> = Vec::new();
        let radii: Vec<usize> = (0..ref_grid.rows.max(ref_grid.cols)).collect();
        let max_pop = grid.iter().fold(0, |acc, (_, agents)| agents.len().max(acc)) as f32;
        for publisher in &mut publishers {
            // If all locations have a Publisher,
            // reset to allow for multiple Publishers per location.
            if already_occupied.len() == grid.keys().len() {
                already_occupied.clear();
            }
            let weights: Vec<(Position, usize)> = grid.iter()
                .filter(|(pos, agents)| !already_occupied.contains(pos) && agents.len() > 0)
                .map(|(pos, agents)| (*pos, agents.len().pow(2)))
                .collect();
            // println!("weights {:?}", weights);
            let pos = weights.choose_weighted(&mut rng, |item| item.1).unwrap().0;
            publisher.location = pos;
            already_occupied.push(pos);

            // So that larger populations are more
            // likely to have larger radii
            let pop = grid[&pos].len() as f32;
            let radius_weights: Vec<(usize, f32)> = radii.iter().map(|r| {
                let v = pop/max_pop;
                let t = radii.len();
                if v >= (*r as f32/t as f32) {
                    let d = (t - r).pow(3) + 1;
                    (*r, v/d as f32)
                } else {
                    (*r, 0.)
                }
            }).collect();
            // println!("pop {:?}", pop);
            // println!("radius_weights {:?}", radius_weights);
            let radius = radius_weights.choose_weighted(&mut rng, |item| item.1).unwrap().0;
            publisher.radius = radius;
            // println!("radius {:?}", radius);
        }

        // Distance to a Publisher is
        // measured against the closest position
        // within its radius.
        let distances = compute_distances(
            &ref_grid,
            &publishers.iter()
                .map(|p| (p.location.clone(), p.radius))
                .collect());

        set_agent_relevancies(&distances, &mut agents);

        Simulation {
            grid: grid,
            ref_grid: ref_grid,
            distances: distances,
            network: network,
            content: Vec::new(),
            agents: agents,
            share_queues: share_queues,
            outboxes: outboxes,
            publishers: publishers,
            platforms: platforms,
            n_produced: 0,
            n_pitched: 0,
            n_published: 0,
            agent_platforms: agent_platforms,
            advertising_tax: 0.,
            subsidy: 0.,
        }
    }

    pub fn step(&mut self, conf: &SimulationConfig, mut rng: &mut StdRng) {
        self.produce(&conf, &mut rng);
        self.consume(&conf, &mut rng);
    }

    pub fn produce(&mut self, conf: &SimulationConfig, mut rng: &mut StdRng) {
        let mut n_pitched = 0;
        let mut n_published = 0;
        let mut new_content: FnvHashMap<(SharerType, usize), Vec<Content>> = FnvHashMap::default();
        for p in &mut self.publishers {
            p.n_ads_sold = 0.;
        }
        let population = self.agents.len();
        for mut a in &mut self.agents {
            match a.try_produce(population, &conf, &mut rng) {
                Some(body) => {
                    // People give up after not getting anything
                    // published
                    let mut published = false;
                    if a.publishability > 0.2 {
                        n_pitched += 1;
                        // Decide to pitch to publisher
                        let publishers = self.publishers.iter()
                            .map(|p| {
                                let prob = a.publishabilities.entry(p.id).or_insert(1.).clone();
                                // Publisher id, probability of acceptance, expected value
                                (p.id, prob, prob*p.reach)
                            })
                            .filter(|(_, p, _)| *p >= 0.1) // Minimum probability
                            .sorted_by(|(_, _, ev), (_, _, ev_)| ev_.partial_cmp(ev).unwrap());
                        for (pub_id, p, _) in publishers {
                            match self.publishers[pub_id].pitch(&body, &mut a, &conf, &mut rng) {
                                (Some(content), _) => {
                                    published = true;
                                    a.publishabilities.insert(pub_id, ewma(1., p));
                                    a.publishability = ewma(1., a.publishability);

                                    let val = new_content.entry((SharerType::Publisher, pub_id))
                                        .or_insert(Vec::new());
                                    (*val).push(content);
                                    break;
                                },
                                (None, could_afford) => {
                                    if could_afford {
                                        // Only consider it a rejection if
                                        // they had budget for the piece
                                        a.publishabilities.insert(pub_id, ewma(0., p));
                                    }
                                }
                            }
                        }
                    }

                    // Self-publish
                    if !published {
                        a.publishability = ewma(0., a.publishability);

                        let content = Content {
                            id: ContentId::new_v4(),
                            publisher: None,
                            author: a.id,
                            body: body,
                            ads: a.ads,
                        };
                        let val = new_content.entry((SharerType::Agent, a.id))
                            .or_insert(Vec::new());
                        (*val).push(content);
                    } else {
                        n_published += 1;
                    }

                    // Update reach
                    a.update_reach();
                },
                None => {}
            }
        }

        // Ad Market
        let n_new_content = new_content.values().fold(0, |acc, v| acc + v.len());
        ad_market(&mut new_content, &self.agents, &self.publishers, &self.platforms, &conf, &mut rng);
        for ((typ, id), contents) in new_content.into_iter() {
            for c in contents {
                let content = Arc::new(c);

                self.content.push(content.clone());

                // TODO
                let to_share = &mut self.share_queues[content.author];
                to_share.push(SharedContent {
                    content: content.clone(),
                    sharer: (SharerType::Agent, content.author)
                });
                self.agents[content.author].content.push(content.clone());
                match typ {
                    SharerType::Publisher => {
                        self.publishers[id].n_ads_sold += content.ads;
                        self.publishers[id].content.push(content.clone());
                        let to_share = &mut self.outboxes[id];
                        to_share.push(SharedContent {
                            content: content.clone(),
                            sharer: (SharerType::Publisher, id)
                        });
                    },
                    SharerType::Agent => {}
                }
            }
        }

        self.n_pitched = n_pitched;
        self.n_produced = n_new_content;
        self.n_published = n_published;
    }

    pub fn consume(&mut self,
                   conf: &SimulationConfig,
                   mut rng: &mut StdRng) {
        let mut sub_changes: Vec<isize> = vec![0; self.publishers.len()];

        // Note the way these are added to assumes that agents are iterated
        // over sequentially, i.e agent.id=0, agent.id=1, etc...
        // and that each agent adds to the vec.
        let mut new_to_share: Vec<Vec<SharedContent>> = Vec::with_capacity(self.agents.len());
        let mut follow_changes: Vec<(FnvHashSet<AgentId>, FnvHashSet<AgentId>)> = Vec::with_capacity(self.agents.len());

        let mut signups: FnvHashMap<AgentId, PlatformId> = FnvHashMap::default();

        let mut platforms: FnvHashMap<PlatformId, usize> = FnvHashMap::default();
        let mut all_data: FnvHashMap<PlatformId, f32> = FnvHashMap::default();
        let mut ad_revenue: FnvHashMap<(SharerType, usize), f32> = FnvHashMap::default();

        // Hack to mutably iterate
        let contact_rate = conf.contact_rate as f64;
        let mut agents: Vec<Agent> = self.agents.drain(..).collect();
        for mut a in agents.drain(..) {
            let mut to_read: Vec<(Option<&PlatformId>, &SharedContent)> = Vec::new();
            // let to_read = &mut shared;

            // Agent encounters shared content
            let following = self.network.following_ids(&a.id).clone();

            // "Offline" encounters
            to_read.clear();
            let n = following.len() as u64;
            let n_encounters = Binomial::new(n, contact_rate).unwrap().sample(rng);
            to_read.extend(
                following.choose_multiple(rng, n_encounters as usize)
                .flat_map(|a_id| self.share_queues[*a_id].iter().map(|sc| (None, sc))));

            // Subscribed publishers
            // ENH: Publishers on all platforms.
            // e.g. outbox.iter().flat_map(|sc| a.platforms.iter().map(|p_id| (p_id, sc.clone())))
            // Although maybe it's not worth the additional overhead?
            to_read.extend(a.subscriptions.iter()
                          .flat_map(|p_id| self.outboxes[*p_id].iter().map(|sc| (None, sc))));

            // Platforms
            // We basically assume that if someone shares something,
            // they share it across all platforms and increases the likelihood
            // that the Agent encounters that shared content.
            // Unlike offline encounters, we roll per shared content
            // rather than per agent.
            // ENH: Agents may develop a preference for a platform?
            to_read.extend(self.agent_platforms[a.id].iter()
                .flat_map(|p_id| self.platforms[*p_id].following_ids(&a.id).into_iter()
                          .map(move |a_id| (p_id, a_id)))
                .flat_map(|(p_id, a_id)| {
                    // "Algorithmic" rating based on Agent's trust of Agent B.
                    // ENH: Trust values should be platform-specific,
                    // to capture that platforms have incomplete/noisy information about
                    // "trust" between users.
                    let contact_rate = f32::min(1., conf.contact_rate + match a.trust.get(a_id) {
                        Some(v) => *v,
                        None => 0.
                    });

                    let to_share = &self.share_queues[*a_id];
                    let n_encounters = Binomial::new(
                        to_share.len() as u64,
                        contact_rate as f64).unwrap().sample(rng);

                    to_share.choose_multiple(rng, n_encounters as usize)
                        .map(move |sc| (Some(p_id), sc))
                    }));

            // Avoid ordering bias
            to_read.shuffle(&mut rng);
            to_read.truncate(conf.max_shared_content);

            // Only consider signing up to new platforms
            // if Agent is not platform-saturated
            if self.agent_platforms[a.id].len() < conf.max_platforms {
                for p in &self.platforms {
                    platforms.insert(p.id, 0);
                }

                // See what platforms friends are on
                following.iter()
                    .flat_map(|a_id| &self.agent_platforms[*a_id])
                    .fold(&mut platforms, |acc, p_id| {
                        // Only consider platforms the agent
                        // isn't already signed up to
                        if !self.agent_platforms[a.id].contains(p_id) {
                            *(acc.entry(*p_id).or_insert(0)) += 1;
                        }
                        acc
                    });

                // Get platform with most friends
                // If no friends, choose a random one
                if platforms.values().all(|v| *v == 0) {
                    let p_ids: Vec<&PlatformId> = platforms.keys().collect();
                    let p_id = p_ids.choose(&mut rng);
                    match p_id {
                        Some(p_id) => {
                            let roll: f32 = rng.gen();
                            if roll < conf.base_signup_rate {
                                signups.insert(a.id, **p_id);
                            }
                        },
                        None => {}
                    }
                } else {
                    match platforms.iter().max_by_key(|&(_, v)| v) {
                        Some((p_id, count)) => {
                            let roll: f32 = rng.gen();
                            if roll < (conf.base_signup_rate + (*count as f32)/(following.len() as f32)) {
                                signups.insert(a.id, *p_id);
                            }
                        },
                        None => {}
                    }
                }
            }

            let (will_share, (new_subs, unsubs), (follows, unfollows), data, revenue) = a.consume(&to_read, &conf, &mut rng);
            let shareable = will_share.iter().map(|content| {
                SharedContent {
                    sharer: (SharerType::Agent, a.id),
                    content: content.clone(),
                }
            }).collect();
            for pub_id in new_subs {
                sub_changes[pub_id] += 1;
            }
            for pub_id in unsubs {
                sub_changes[pub_id] -= 1;
            }

            follow_changes.push((follows, unfollows));

            // Aggregate generated data
            for (p_id, d) in data {
                let d_ = all_data.entry(p_id).or_insert(0.);
                *d_ += d;
            }

            // Aggregate ad revenue
            for (tid, r) in revenue {
                let r_ = ad_revenue.entry(tid).or_insert(0.);
                *r_ += r;
            }

            new_to_share.push(shareable);
            self.agents.push(a);
        }

        // Update share lists
        for (a_id, mut to_share_) in new_to_share.into_iter().enumerate() {
            let to_share = &mut self.share_queues[a_id];
            to_share.clear();
            to_share.append(&mut to_share_);
        }

        // Update follows
        // TODO this feels very messy
        for (a_id, (follows, unfollows)) in follow_changes.iter().enumerate() {
            if follows.len() > 0 || unfollows.len() > 0 {
                let p_ids: Vec<&PlatformId> = self.agent_platforms[a_id].iter().collect();
                for p_id in p_ids {
                    let pfrm = &mut self.platforms[*p_id];
                    for b_id in follows {
                        if pfrm.is_signed_up(b_id) {
                            pfrm.follow(&a_id, &b_id);
                        }
                    }
                    for b_id in unfollows {
                        if pfrm.is_signed_up(b_id) {
                            pfrm.unfollow(&a_id, &b_id);
                        }
                    }
                }
            }
        }

        for p in &mut self.publishers {
            p.audience_survey(conf.content_sample_size);
            p.update_reach();

            // Update subscribers
            p.subscribers = std::cmp::max(0, p.subscribers as isize + sub_changes[p.id]) as usize;

            p.n_last_published = self.outboxes[p.id].len();
            p.budget += p.regular_revenue() + self.subsidy;

            // ENH: Publisher pushes content
            // for multiple steps?
            self.outboxes[p.id].clear();
        }

        // Distribute ad revenue
        for ((typ, id), r) in ad_revenue {
            let update = rng.gen::<f32>() < 0.1;
            let r = r * (1.-self.advertising_tax);
            match typ {
                SharerType::Publisher => {
                    self.publishers[id].budget += r;
                    self.publishers[id].learn(r, update, &mut rng);
                    self.publishers[id].expenses = 0.;
                },
                SharerType::Agent => {
                    self.agents[id].resources += r;
                    self.agents[id].learn(r, update, &mut rng);
                    self.agents[id].expenses = 0.;
                }
            }
        }

        // Add data to platforms
        for p in &mut self.platforms {
            p.data += *all_data.entry(p.id).or_insert(0.);
            p.update_conversion_rate(conf.max_conversion_rate);
        }

        // Sign up agents and follow friends
        // ENH: Maybe not all friends should be followed
        for (a_id, p_id) in signups {
            if !self.platforms[p_id].is_signed_up(&a_id) {
                self.platforms[p_id].signup(self.agents[a_id].id);
                self.agent_platforms[a_id].insert(p_id);
                for b_id in self.network.following_ids(&self.agents[a_id].id) {
                    let platform = &mut self.platforms[p_id];
                    if platform.is_signed_up(b_id) {
                        platform.follow(&a_id, b_id);
                        platform.follow(b_id, &a_id);
                    }
                }
            }
        }
    }

    pub fn n_will_share(&self) -> usize {
        self.share_queues.iter().fold(0, |acc, v| acc + v.len())
    }

    pub fn n_shares(&self) -> Vec<usize> {
        self.content.iter().map(|c| Arc::strong_count(c)).collect()
    }

    pub fn content_by_popularity(&self) -> std::vec::IntoIter<&Arc<Content>> {
        self.content.iter().sorted_by(|a, b| Arc::strong_count(b).cmp(&Arc::strong_count(a)))
    }

    pub fn apply_policy(&mut self, policy: &Policy, conf: &mut SimulationConfig, rng: &mut StdRng) {
        match policy {
            Policy::Recession(n) => {
                conf.economy = f32::max(0., conf.economy - n);
                conf.unsubscribe_lag = max(0, conf.unsubscribe_lag as isize - (*n*10.) as isize) as usize;
                conf.unsubscribe_trust = f32::min(1., conf.unsubscribe_trust + n/5.);
                conf.subscribe_trust = f32::min(1., conf.subscribe_trust + n/5.);
                conf.base_conversion_rate = f32::max(0., conf.base_conversion_rate - n/100.);
                conf.revenue_per_ad = f32::max(0., conf.revenue_per_ad - n/100.);
            },

            Policy::MediaLiteracy(n) => {
                for a in &mut self.agents {
                    a.media_literacy = f32::min(1., a.media_literacy + n);
                }
            },

            Policy::FoundPlatforms(n) => {
                for _ in 0..*n {
                    let platform = Platform::new(self.platforms.len());
                    self.platforms.push(platform);
                }
            },

            Policy::TaxAdvertising(tax) => {
                self.advertising_tax = *tax;
            },
            Policy::SubsidizeProduction(amount) => {
                self.subsidy = *amount;
            },

            Policy::PopulationChange(n) => {
                if *n > 0 {
                    let mut new_agents: Vec<Agent> = (0..(*n as usize))
                        .map(|i| Agent::new(i, &conf.agent, rng))
                        .collect();

                    distribute_agents(&mut new_agents, &mut self.grid, rng);
                    set_agent_relevancies(&self.distances, &mut new_agents);

                    for a in new_agents {
                        self.agent_platforms.push(FnvHashSet::default());
                        self.share_queues.push(Vec::new());
                        self.agents.push(a);
                    }

                    self.network.preferential_attachment(&self.agents, MAX_FRIENDS, rng);
                }
            }
        }
    }
}

pub fn compute_distances(grid: &HexGrid, spots: &Vec<(Position, usize)>) -> FnvHashMap<Position, Vec<usize>> {
    let mut distances = FnvHashMap::default();
    for pos in grid.positions().iter() {
        let mut dists = Vec::new();
        for (loc, rad) in spots {
            let start = hexagon_dist(pos, loc);
            let dist = grid.radius(loc, *rad).iter().fold(start, |acc, pos_| {
                let dist = hexagon_dist(pos, pos_);
                dist.min(acc)
            });
            dists.push(dist);
        }
        distances.insert(*pos, dists);
    }
    distances
}

fn relevance_from_dist(dist: usize) -> f32 {
    let x = 2*(dist as isize)-4;
    1. - sigmoid(x as f32)
}

pub fn set_agent_relevancies(distances: &FnvHashMap<Position, Vec<usize>>, agents: &mut Vec<Agent>) {
    for agent in agents {
        for dist in &distances[&agent.location] {
            let relevance = relevance_from_dist(*dist);
            agent.relevancies.push(relevance);
        }
    }
}

fn distribute_agents(agents: &mut Vec<Agent>, grid: &mut FnvHashMap<Position, Vec<AgentId>>, rng: &mut StdRng) {
    // Randomly assign agents by density
    for agent in agents {
        let weights: Vec<(Position, usize)> = grid.iter()
            .map(|(pos, agents)| (*pos, agents.len() + 1))
            .collect();
        let pos = weights.choose_weighted(rng, |item| item.1).unwrap().0;
        grid.get_mut(&pos).unwrap().push(agent.id);
        agent.location = pos;
    }
}

pub fn ad_market(content: &mut FnvHashMap<(SharerType, usize), Vec<Content>>, agents: &Vec<Agent>, publishers: &Vec<Publisher>, platforms: &Vec<Platform>, conf: &SimulationConfig, rng: &mut StdRng) {
    let econ = f32::min(conf.economy, 1.);
    let z = platforms.iter().fold(0., |acc, platform| acc + platform.conversion_rate);
    let max_p = 0.95; // Required to avoid beta of 0.0
    let min_p = 0.05; // Required to avoid alpha of 0.0
    for ((typ, id), ref mut contents) in &mut content.iter_mut() {
        let (mut p, ad_slots) = match typ {
            SharerType::Publisher => {
                // TODO take reach into account
                let p = f32::max(min_p,
                                 f32::min(max_p,
                                          conf.base_conversion_rate/(conf.base_conversion_rate + z)));
                let ad_slots = publishers[*id].ads;
                (p, ad_slots)
            },
            SharerType::Agent => {
                // TODO what should this be for agents?
                let p = f32::max(min_p,
                                 f32::min(max_p,
                                          conf.base_conversion_rate/(conf.base_conversion_rate + z)));
                let ad_slots = agents[*id].ads;
                (p, ad_slots)
            }
        };
        if ad_slots > 0. {
            p *= econ;
            let alpha = p * ad_slots;
            let beta = (1.-p) * ad_slots;
            // println!("alpha {:?}, beta {:?}, ad slots {:?}", alpha, beta, ad_slots);
            let dist = Beta::new(alpha, beta).unwrap();
            for c in &mut contents.iter_mut() {
                c.ads = dist.sample(rng);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        let grid_size = 3;
        let grid = HexGrid::new(grid_size, grid_size);
        let spots = vec![
            ((0, 0), 0),
            ((0, 0), 2),
        ];
        let distances = compute_distances(&grid, &spots);

        assert_eq!(distances[&(0, 0)], vec![0, 0]);
        assert_eq!(distances[&(0, 1)], vec![1, 0]);
        assert_eq!(distances[&(1, 0)], vec![1, 0]);
        assert_eq!(distances[&(2, 0)], vec![2, 0]);
        assert_eq!(distances[&(0, 2)], vec![2, 0]);
        assert_eq!(distances[&(2, 1)], vec![2, 0]);
        assert_eq!(distances[&(1, 2)], vec![2, 1]);
    }

    #[test]
    fn relevances() {
        let mut last = 1.;
        let expected = [0.95, 0.85, 0.5, 0.1, 0.01];
        for i in 0..5 {
            let rel = relevance_from_dist(i);
            assert!(rel >= expected[i]);

            // Relevance should decrease with distance
            assert!(rel < last);
            last = rel;
        }
    }
}
