use rand::Rng;
use std::rc::Rc;
use fnv::{FnvHashMap, FnvHashSet};
use super::agent::{Agent, AgentId};
use super::policy::Policy;
use super::network::Network;
use super::platform::{Platform, PlatformId};
use super::publisher::Publisher;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use super::content::{Content, SharedContent, SharerType};
use super::util::ewma;
use super::config::SimulationConfig;
use itertools::Itertools;

pub struct Simulation {
    pub network: Network,
    pub agents: Vec<Agent>,
    content: Vec<Rc<Content>>,
    pub publishers: Vec<Publisher>,
    pub platforms: Vec<Platform>,
    share_queues: FnvHashMap<AgentId, Vec<SharedContent>>,
}


impl Simulation {
    pub fn new(conf: &SimulationConfig, mut rng: &mut StdRng) -> Simulation {
        let agents: Vec<Agent> = (0..conf.population)
            .map(|i| Agent::new(i, &mut rng))
            .collect();

        let mut share_queues = FnvHashMap::default();
        for agent in agents.iter() {
            share_queues.insert(agent.id, Vec::new());
        }

        let publishers: Vec<Publisher> = (0..conf.n_publishers)
            .map(|i| Publisher::new(i, &conf.publisher, &mut rng))
            .collect();

        let platforms: Vec<Platform> = (0..conf.n_platforms)
            .map(|i| Platform::new(i))
            .collect();

        let network = Network::new(&agents, &mut rng);

        Simulation {
            network: network,
            content: Vec::new(),
            agents: agents,
            share_queues: share_queues,
            publishers: publishers,
            platforms: platforms,
        }
    }

    pub fn produce(&mut self, mut rng: &mut StdRng) -> usize {
        let n_content_start = self.content.len();
        for a in &mut self.agents {
            if let Some(to_share) = self.share_queues.get_mut(&a.id) {
                match a.produce(&mut rng) {
                    Some(body) => {
                        // People give up after not getting anything
                        // published
                        let mut published = false;
                        if a.publishability > 0.1 {
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
                                match self.publishers[pub_id].pitch(&body, &a, &mut rng) {
                                    Some(content) => {
                                        published = true;
                                        a.publishabilities.insert(pub_id, ewma(1., p));
                                        a.publishability = ewma(1., a.publishability);

                                        // Share to own networks
                                        to_share.push(SharedContent {
                                            content: content.clone(),
                                            sharer: (SharerType::Agent, a.id)
                                        });
                                        self.content.push(content.clone());
                                        break;
                                    },
                                    None => {
                                        a.publishabilities.insert(pub_id, ewma(0., p));
                                    }
                                }
                            }
                        }

                        // Self-publish
                        if !published {
                            a.publishability = ewma(0., a.publishability);

                            let content = Rc::new(Content {
                                publisher: None,
                                author: a.id,
                                body: body,
                                ads: 100. // TODO
                            });
                            to_share.push(SharedContent {
                                content: content.clone(),
                                sharer: (SharerType::Agent, a.id)
                            });
                            self.content.push(content.clone());
                        }
                    },
                    None => {}
                }
            }
        }
        self.content.len() - n_content_start
    }

    pub fn consume(&mut self,
                   conf: &SimulationConfig,
                   mut rng: &mut StdRng) {
        let mut new_to_share: FnvHashMap<AgentId, Vec<SharedContent>> = FnvHashMap::default();
        let mut sub_changes: Vec<isize> = vec![0; self.publishers.len()];

        let mut follow_changes: FnvHashMap<AgentId, (FnvHashSet<AgentId>, FnvHashSet<AgentId>)> = FnvHashMap::default();

        let mut signups: FnvHashMap<AgentId, PlatformId> = FnvHashMap::default();
        let mut platforms: FnvHashMap<PlatformId, usize> = FnvHashMap::default();
        let mut all_data: FnvHashMap<PlatformId, f32> = FnvHashMap::default();
        let mut all_revenue: FnvHashMap<(SharerType, usize), f32> = FnvHashMap::default();
        for a in &self.agents {
            // Agent encounters shared content
            let following = self.network.following_ids(&a).clone();

            // "Offline" encounters
            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = following.iter()
                .filter(|_| rng.gen::<f32>() < conf.contact_rate)
                .flat_map(|a_id| self.share_queues[a_id].iter().map(|sc| (None, sc)))
                .collect();

            // Subscribed publishers
            // ENH: Publishers on all platforms.
            // e.g. outbox.iter().flat_map(|sc| a.platforms.iter().map(|p_id| (p_id, sc.clone())))
            // Although maybe it's not worth the additional overhead?
            shared.extend(a.subscriptions.borrow().iter()
                          .flat_map(|p_id| self.publishers[*p_id].outbox.iter().map(|sc| (None, sc))));

            // Platforms
            // We basically assume that if someone shares something,
            // they share it across all platforms and increases the likelihood
            // that the Agent encounters that shared content.
            // Unlike offline encounters, we roll per shared content
            // rather than per agent.
            // ENH: Agents may develop a preference for a platform?
            shared.extend(a.platforms.iter()
                .flat_map(|p_id| self.platforms[*p_id].following_ids(&a).into_iter()
                          .map(move |a_id| (p_id, a_id)))
                .flat_map(|(p_id, a_id)| self.share_queues[a_id].iter().map(move |sc| (Some(p_id), sc)))
                .filter(|(_, sc)| {
                    // "Algorithmic" rating based on Agent's trust of Agent B.
                    // ENH: Trust values should be platform-specific,
                    // to capture that platforms have incomplete/noisy information about
                    // "trust" between users.
                    rng.gen::<f32>() < conf.contact_rate + match a.trust.borrow().get(&sc.sharer.1) {
                        Some(v) => *v,
                        None => 0.
                    }
                }));

            // Avoid ordering bias
            shared.shuffle(&mut rng);

            // Only consider signing up to new platforms
            // if Agent is not platform-saturated
            if a.platforms.len() < conf.max_platforms {
                for p in &self.platforms {
                    platforms.insert(p.id, 0);
                }

                // See what platforms friends are on
                following.iter()
                    .flat_map(|a_id| &self.agents[**a_id].platforms)
                    .fold(&mut platforms, |acc, p_id| {
                        // Only consider platforms the agent
                        // isn't already signed up to
                        if !a.platforms.contains(p_id) {
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

            let (will_share, (new_subs, unsubs), (follows, unfollows), data, revenue) = a.consume(shared, &self.network, &conf, &mut rng);
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

            follow_changes.insert(a.id, (follows, unfollows));

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

            new_to_share.insert(a.id, shareable);
        }

        // Update share lists
        for (a_id, mut to_share_) in new_to_share {
            match self.share_queues.get_mut(&a_id) {
                Some(to_share) => {
                    to_share.clear();
                    to_share.append(&mut to_share_);
                },
                None => {
                    self.share_queues.insert(a_id, to_share_);
                }
            }
        }

        // Update follows
        // TODO this feels very messy
        for (a_id, (follows, unfollows)) in follow_changes {
            if follows.len() > 0 || unfollows.len() > 0 {
                let p_ids: Vec<&PlatformId> = self.agents[a_id].platforms.iter().collect();
                for p_id in p_ids {
                    let pfrm = &mut self.platforms[*p_id];
                    for b_id in &follows {
                        if pfrm.is_signed_up(b_id) {
                            pfrm.follow(&a_id, &b_id, 1.); // TODO diff weights?
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

        for p in &mut self.publishers {
            p.audience_survey(conf.content_sample_size);
            p.update_reach();

            // Update subscribers
            p.subscribers = std::cmp::max(0, p.subscribers as isize + sub_changes[p.id]) as usize;

            p.n_last_published = p.outbox.len();
            p.budget = conf.publisher.base_budget + p.operating_budget();

            // ENH: Publisher pushes content
            // for multiple steps?
            p.outbox.clear();
        }

        // Distribute ad revenue
        for ((typ, id), r) in all_revenue {
            match typ {
                SharerType::Publisher => {
                    self.publishers[id].budget += r;
                    self.publishers[id].learn(r);
                },
                SharerType::Agent => {
                    self.agents[id].resources += r;
                }
            }
        }

        // Add data to platforms
        for p in &mut self.platforms {
            p.data += *all_data.entry(p.id).or_insert(0.);
        }

        // Sign up agents and follow friends
        // ENH: Maybe not all friends should be followed
        for (a_id, p_id) in signups {
            if !self.platforms[p_id].is_signed_up(&a_id) {
                self.platforms[p_id].signup(&self.agents[a_id]);
                self.agents[a_id].platforms.insert(p_id);
                for b_id in self.network.following_ids(&self.agents[a_id]) {
                    let platform = &mut self.platforms[p_id];
                    if platform.is_signed_up(b_id) {
                        let trust_a = self.network.trust(&a_id, b_id);
                        let trust_b = self.network.trust(b_id, &a_id);
                        platform.follow(&a_id, b_id, trust_a); // TODO what should this weight be?
                        platform.follow(b_id, &a_id, trust_b); // TODO what should this weight be?
                    }
                }
            }
        }
    }

    pub fn n_will_share(&self) -> usize {
        self.share_queues.values().fold(0, |acc, v| acc + v.len())
    }

    pub fn n_shares(&self) -> Vec<usize> {
        // -1 to account for reference in self.content
        // Note that content from Publishers will have an extra +1
        // because of their publisher.content reference.
        // But that should be negligible
        self.content.iter().map(|c| Rc::strong_count(c) - 1).collect()
    }

    pub fn content_by_popularity(&self) -> std::vec::IntoIter<&Rc<Content>> {
        self.content.iter().sorted_by(|a, b| Rc::strong_count(b).cmp(&Rc::strong_count(a)))
    }

    pub fn apply_policy(&mut self, policy: &Policy) {
        // TODO
    }
}
