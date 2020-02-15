use std::rc::Rc;
use fnv::FnvHashMap;
use super::agent::{Agent, AgentId};
use super::policy::Policy;
use super::network::Network;
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
    publishers: Vec<Publisher>,
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

        let network = Network::new(&agents, &mut rng);

        Simulation {
            network: network,
            content: Vec::new(),
            agents: agents,
            share_queues: share_queues,
            publishers: publishers,
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
                                body: body
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

        for a in &self.agents {
            let mut shared: Vec<&SharedContent> = self.network.follower_ids(&a).iter()
                .flat_map(|n_id| self.share_queues[n_id].iter()).collect();
            shared.extend(a.subscriptions.borrow().iter().flat_map(|p_id| self.publishers[*p_id].outbox.iter()));
            shared.shuffle(&mut rng);

            let (will_share, (new_subs, unsubs)) = a.consume(shared, &self.network, &conf, &mut rng);
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

        for p in &mut self.publishers {
            p.audience_survey(conf.content_sample_size);
            p.update_reach();

            // Update subscribers
            p.subscribers = std::cmp::max(0, p.subscribers as isize + sub_changes[p.id]) as usize;

            // ENH: Publisher pushes content
            // for multiple steps?
            p.outbox.clear();
            p.budget = conf.publisher.base_budget + p.operating_budget();
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
