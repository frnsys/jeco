use rand::Rng;
use fnv::{FnvHashMap, FnvHashSet};
use super::agent::{Agent, AgentId};
use super::policy::Policy;
use super::network::Network;
use super::platform::{Platform, PlatformId};
use super::publisher::{Publisher, PublisherId};
use super::grid::{HexGrid, Position, hexagon_dist};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use super::content::{Content, ContentId, SharedContent, SharerType};
use super::util::{ewma, sigmoid};
use super::config::SimulationConfig;
use itertools::Itertools;
use rand_distr::{Distribution, Beta};
use std::sync::{Mutex, Arc};
use std::thread;
use rayon::prelude::*;

static MAX_FRIENDS: usize = 120;

pub struct Simulation {
    pub network: Arc<Mutex<Network>>,
    pub agents: Vec<Agent>,
    content: Vec<Arc<Content>>,
    pub publishers: Vec<Publisher>,
    pub platforms: Arc<Mutex<Vec<Platform>>>,
    pub ref_grid: HexGrid,
    pub grid: FnvHashMap<Position, Vec<AgentId>>,
    pub distances: FnvHashMap<Position, Vec<usize>>,

    // Stats
    pub n_produced: usize,
    pub n_pitched: usize,
    pub n_published: usize,

    // Content Agents will share in the next step.
    // Emptied each step.
    share_queues: Arc<Mutex<FnvHashMap<AgentId, Vec<SharedContent>>>>,

    // Store content the Publisher will
    // publish in the next step. Emptied each step.
    outboxes: Arc<Mutex<FnvHashMap<PublisherId, Vec<SharedContent>>>>,
}


impl Simulation {
    pub fn new(conf: &SimulationConfig, mut rng: &mut StdRng) -> Simulation {
        let mut agents: Vec<Agent> = (0..conf.population)
            .map(|i| Agent::new(i, &conf.agent, &mut rng))
            .collect();

        let mut share_queues = FnvHashMap::default();
        for agent in agents.iter() {
            share_queues.insert(agent.id, Vec::new());
        }

        let mut publishers: Vec<Publisher> = (0..conf.n_publishers)
            .map(|i| Publisher::new(i, &conf.publisher, &mut rng))
            .collect();

        let mut outboxes = FnvHashMap::default();
        for publisher in publishers.iter() {
            outboxes.insert(publisher.id, Vec::new());
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

        // Randomly assign agents by density
        for agent in &mut agents {
            let weights: Vec<(Position, usize)> = grid.iter()
                .map(|(pos, agents)| (*pos, agents.len() + 1))
                .collect();
            let pos = weights.choose_weighted(&mut rng, |item| item.1).unwrap().0;
            grid.get_mut(&pos).unwrap().push(agent.id);
            agent.location = pos;
        }

        // Randomly assign publishers by density
        let mut already_occupied: Vec<Position> = Vec::new();
        for publisher in &mut publishers {
            // If all locations have a Publisher,
            // reset to allow for multiple Publishers per location.
            if already_occupied.len() == grid.keys().len() {
                already_occupied.clear();
            }
            let weights: Vec<(Position, usize)> = grid.iter()
                .filter(|(pos, _)| !already_occupied.contains(pos))
                .map(|(pos, agents)| (*pos, agents.len() + 1))
                .collect();
            let pos = weights.choose_weighted(&mut rng, |item| item.1).unwrap().0;
            publisher.location = pos;
            already_occupied.push(pos);

            let radius = (rng.gen::<f32>() * (ref_grid.rows.max(ref_grid.cols) + 1) as f32).floor() as usize;
            publisher.radius = radius;
        }

        let distances = set_agent_relevancies(&ref_grid, &mut agents, &publishers);

        Simulation {
            grid: grid,
            ref_grid: ref_grid,
            distances: distances,
            network: Arc::new(Mutex::new(network)),
            content: Vec::new(),
            agents: agents,
            share_queues: Arc::new(Mutex::new(share_queues)),
            outboxes: Arc::new(Mutex::new(outboxes)),
            publishers: publishers,
            platforms: Arc::new(Mutex::new(platforms)),
            n_produced: 0,
            n_pitched: 0,
            n_published: 0,
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
        for mut a in &mut self.agents {
            match a.try_produce(&conf, &mut rng) {
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
        ad_market(&mut new_content, &self.agents, &self.publishers, &self.platforms.lock().unwrap(), &conf, &mut rng);
        for ((typ, id), contents) in new_content.into_iter() {
            for c in contents {
                let content = Arc::new(c);

                self.content.push(content.clone());

                // TODO
                match self.share_queues.lock().unwrap().get_mut(&content.author) {
                    Some(to_share) => {
                        to_share.push(SharedContent {
                            content: content.clone(),
                            sharer: (SharerType::Agent, content.author)
                        });
                    },
                    None => {}
                }
                self.agents[content.author].content.push(content.clone());
                match typ {
                    SharerType::Publisher => {
                        self.publishers[id].n_ads_sold += content.ads;
                        self.publishers[id].content.push(content.clone());
                        match self.outboxes.lock().unwrap().get_mut(&id) {
                            Some(to_share) => {
                                to_share.push(SharedContent {
                                    content: content.clone(),
                                    sharer: (SharerType::Publisher, id)
                                });
                            },
                            None => {}
                        }
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
        let mut new_to_share: FnvHashMap<AgentId, Vec<SharedContent>> = FnvHashMap::default();
        let mut sub_changes: Vec<isize> = vec![0; self.publishers.len()];

        let mut follow_changes: FnvHashMap<AgentId, (FnvHashSet<AgentId>, FnvHashSet<AgentId>)> = FnvHashMap::default();

        let mut signups: FnvHashMap<AgentId, PlatformId> = FnvHashMap::default();
        let mut all_data: FnvHashMap<PlatformId, f32> = FnvHashMap::default();
        let mut all_revenue: FnvHashMap<(SharerType, usize), f32> = FnvHashMap::default();

        // TODO TEMP
        let agent_platforms: Arc<Mutex<FnvHashMap<AgentId, FnvHashSet<PlatformId>>>> = Arc::new(Mutex::new(FnvHashMap::default()));
        {
            let mut agent_platforms = agent_platforms.lock().unwrap();
            for a in &self.agents {
                agent_platforms.insert(a.id, FnvHashSet::default());
            }
        }

        let agents: Vec<Agent> = self.agents.drain(..).collect();
        let handles: Vec<(Agent, (Vec<Arc<Content>>, (Vec<PublisherId>, Vec<PublisherId>), (FnvHashSet<AgentId>, FnvHashSet<AgentId>), FnvHashMap<PlatformId, f32>, FnvHashMap<(SharerType, usize), f32>))> = agents.into_par_iter().map(|mut a| {
            let conf = conf.clone();
            let network = self.network.clone();
            let share_queues = self.share_queues.clone();
            let outboxes = self.outboxes.clone();
            let platforms = self.platforms.clone();
            let agent_platforms = agent_platforms.clone();

            let mut rng = rand::thread_rng(); // TODO how to use seedable rng?
            let mut to_read: Vec<(Option<&PlatformId>, &SharedContent)> = Vec::new();

            // Agent encounters shared content
            let following = network.lock().unwrap().following_ids(&a.id).clone();

            // "Offline" encounters
            let share_queues = share_queues.lock().unwrap();
            to_read.extend(following.iter()
                .filter(|_| rng.gen::<f32>() < conf.contact_rate)
                .flat_map(|a_id| share_queues[a_id].iter().map(|sc| (None, sc))));

            // Subscribed publishers
            // ENH: Publishers on all platforms.
            // e.g. outbox.iter().flat_map(|sc| a.platforms.iter().map(|p_id| (p_id, sc.clone())))
            // Although maybe it's not worth the additional overhead?
            let outboxes = outboxes.lock().unwrap();
            to_read.extend(a.subscriptions.iter()
                          .flat_map(|p_id| outboxes[p_id].iter().map(|sc| (None, sc))));

            // Platforms
            // We basically assume that if someone shares something,
            // they share it across all platforms and increases the likelihood
            // that the Agent encounters that shared content.
            // Unlike offline encounters, we roll per shared content
            // rather than per agent.
            // ENH: Agents may develop a preference for a platform?
            let platforms = platforms.lock().unwrap();
            let agent_platforms = agent_platforms.lock().unwrap();
            to_read.extend(agent_platforms[&a.id].iter()
                .flat_map(|p_id| platforms[*p_id].following_ids(&a.id).into_iter()
                          .map(move |a_id| (p_id, a_id)))
                .flat_map(|(p_id, a_id)| share_queues[a_id].iter().map(move |sc| (Some(p_id), sc)))
                .filter(|(_, sc)| {
                    // "Algorithmic" rating based on Agent's trust of Agent B.
                    // ENH: Trust values should be platform-specific,
                    // to capture that platforms have incomplete/noisy information about
                    // "trust" between users.
                    rng.gen::<f32>() < conf.contact_rate + match a.trust.get(&sc.sharer.1) {
                        Some(v) => *v,
                        None => 0.
                    }
                }));

            // Avoid ordering bias
            to_read.shuffle(&mut rng);
            to_read.truncate(conf.max_shared_content);

            // Only consider signing up to new platforms
            // if Agent is not platform-saturated
            // if agent_platforms[&a.id].len() < conf.max_platforms {
            //     let ps = platforms.lock().unwrap();
            //     let mut platforms: FnvHashMap<PlatformId, usize> = FnvHashMap::default();
            //     for p in &ps {
            //         platforms.insert(p.id, 0);
            //     }

            //     // See what platforms friends are on
            //     following.iter()
            //         .flat_map(|a_id| &agent_platforms[a_id])
            //         .fold(&mut platforms, |acc, p_id| {
            //             // Only consider platforms the agent
            //             // isn't already signed up to
            //             if !agent_platforms[&a.id].contains(p_id) {
            //                 *(acc.entry(*p_id).or_insert(0)) += 1;
            //             }
            //             acc
            //         });

            //     // Get platform with most friends
            //     // If no friends, choose a random one
            //     if platforms.values().all(|v| *v == 0) {
            //         let p_ids: Vec<&PlatformId> = platforms.keys().collect();
            //         let p_id = p_ids.choose(&mut rng);
            //         match p_id {
            //             Some(p_id) => {
            //                 let roll: f32 = rng.gen();
            //                 if roll < conf.base_signup_rate {
            //                     signups.insert(a.id, **p_id);
            //                 }
            //             },
            //             None => {}
            //         }
            //     } else {
            //         match platforms.iter().max_by_key(|&(_, v)| v) {
            //             Some((p_id, count)) => {
            //                 let roll: f32 = rng.gen();
            //                 if roll < (conf.base_signup_rate + (*count as f32)/(following.len() as f32)) {
            //                     signups.insert(a.id, *p_id);
            //                 }
            //             },
            //             None => {}
            //         }
            //     }
            // }

            let results = a.consume(&to_read, &conf);
            (a, results)
        }).collect();

        for (agent, results) in handles {
            let (will_share, (new_subs, unsubs), (follows, unfollows), data, revenue) = results;
            let shareable = will_share.iter().map(|content| {
                SharedContent {
                    sharer: (SharerType::Agent, agent.id),
                    content: content.clone(),
                }
            }).collect();
            for pub_id in new_subs {
                sub_changes[pub_id] += 1;
            }
            for pub_id in unsubs {
                sub_changes[pub_id] -= 1;
            }

            follow_changes.insert(agent.id, (follows, unfollows));

            // Aggregate generated data
            for (p_id, d) in data {
                let d_ = all_data.entry(p_id).or_insert(0.);
                *d_ += d;
            }

            // Aggregate ad revenue
            for (tid, r) in revenue {
                let r_ = all_revenue.entry(tid).or_insert(0.);
                *r_ += r;
            }

            new_to_share.insert(agent.id, shareable);

            // Add agent back
            self.agents.push(agent);
        }

        // Update share lists
        let mut share_queues = self.share_queues.lock().unwrap();
        for (a_id, mut to_share_) in new_to_share {
            match share_queues.get_mut(&a_id) {
                Some(to_share) => {
                    to_share.clear();
                    to_share.append(&mut to_share_);
                },
                None => {
                    share_queues.insert(a_id, to_share_);
                }
            }
        }

        // Update follows
        // TODO this feels very messy
        let mut platforms = self.platforms.lock().unwrap();
        let mut agent_platforms = agent_platforms.lock().unwrap();
        for (a_id, (follows, unfollows)) in follow_changes {
            if follows.len() > 0 || unfollows.len() > 0 {
                let p_ids: Vec<&PlatformId> = agent_platforms[&a_id].iter().collect();
                for p_id in p_ids {
                    let pfrm = &mut platforms[*p_id];
                    for b_id in &follows {
                        if pfrm.is_signed_up(b_id) {
                            pfrm.follow(&a_id, &b_id);
                        }
                    }
                    for b_id in &unfollows {
                        if pfrm.is_signed_up(b_id) {
                            pfrm.unfollow(&a_id, &b_id);
                        }
                    }
                }
            }
        }

        let mut outboxes = self.outboxes.lock().unwrap();
        for p in &mut self.publishers {
            p.audience_survey(conf.content_sample_size);
            p.update_reach();

            // Update subscribers
            p.subscribers = std::cmp::max(0, p.subscribers as isize + sub_changes[p.id]) as usize;

            p.n_last_published = outboxes[&p.id].len();
            p.budget += p.regular_revenue();

            // ENH: Publisher pushes content
            // for multiple steps?
            match outboxes.get_mut(&p.id) {
                Some(outbox) => outbox.clear(),
                None => {}
            }
        }

        // Distribute ad revenue
        for ((typ, id), r) in all_revenue {
            let update = rng.gen::<f32>() < 0.1;
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
        for p in platforms.iter_mut() { // TODO?
            p.data += *all_data.entry(p.id).or_insert(0.);
            p.update_conversion_rate(conf.max_conversion_rate);
        }

        // Sign up agents and follow friends
        // ENH: Maybe not all friends should be followed
        let network = self.network.lock().unwrap();
        for (a_id, p_id) in signups {
            if !platforms[p_id].is_signed_up(&a_id) {
                platforms[p_id].signup(self.agents[a_id].id);
                agent_platforms.get_mut(&a_id).unwrap().insert(p_id);
                for b_id in network.following_ids(&self.agents[a_id].id) {
                    let platform = &mut platforms[p_id];
                    if platform.is_signed_up(b_id) {
                        platform.follow(&a_id, b_id);
                        platform.follow(b_id, &a_id);
                    }
                }
            }
        }
    }

    pub fn n_will_share(&self) -> usize {
        self.share_queues.lock().unwrap().values().fold(0, |acc, v| acc + v.len())
    }

    pub fn n_shares(&self) -> Vec<usize> {
        self.content.iter().map(|c| Arc::strong_count(c)).collect()
    }

    pub fn content_by_popularity(&self) -> std::vec::IntoIter<&Arc<Content>> {
        self.content.iter().sorted_by(|a, b| Arc::strong_count(b).cmp(&Arc::strong_count(a)))
    }

    pub fn apply_policy(&mut self, policy: &Policy) {
        let mut platforms = self.platforms.lock().unwrap();
        match policy {
            Policy::FoundPlatforms(n) => {
                for _ in 0..*n {
                    let platform = Platform::new(platforms.len());
                    platforms.push(platform);
                }
            },

            // TODO
            _ => {}
        }
    }
}

fn compute_distances(grid: &HexGrid, spots: &Vec<(Position, usize)>) -> FnvHashMap<Position, Vec<usize>> {
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

pub fn set_agent_relevancies(grid: &HexGrid, agents: &mut Vec<Agent>, publishers: &Vec<Publisher>) -> FnvHashMap<Position, Vec<usize>> {
    // Distance to a Publisher is
    // measured against the closest position
    // within its radius.
    let distances = compute_distances(
        grid,
        &publishers.iter()
            .map(|p| (p.location.clone(), p.radius))
            .collect());

    // Precompute relevancies for each Publisher
    for agent in agents {
        for dist in &distances[&agent.location] {
            let relevance = relevance_from_dist(*dist);
            agent.relevancies.push(relevance);
        }
    }

    distances
}

pub fn ad_market(content: &mut FnvHashMap<(SharerType, usize), Vec<Content>>, agents: &Vec<Agent>, publishers: &Vec<Publisher>, platforms: &Vec<Platform>, conf: &SimulationConfig, rng: &mut StdRng) {
    let z = platforms.iter().fold(0., |acc, platform| acc + platform.conversion_rate);
    let max_p = 0.95; // Required to avoid beta of 0.0
    let min_p = 0.05; // Required to avoid alpha of 0.0
    for ((typ, id), ref mut contents) in &mut content.iter_mut() {
        let (p, ad_slots) = match typ {
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
