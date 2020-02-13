use std::rc::Rc;
use fnv::FnvHashMap;
use super::agent::Agent;
use super::policy::Policy;
use super::network::Network;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use super::content::{Content, SharedContent};

pub struct Simulation {
    pub network: Network,
    pub agents: Vec<Rc<Agent>>,
    content: Vec<Rc<Content>>,
    share_queues: FnvHashMap<usize, Vec<SharedContent>>,
}

impl Simulation {
    pub fn new(population: usize, mut rng: &mut StdRng) -> Simulation {
        let agents: Vec<Agent> = (0..population)
            .map(|i| Agent::new(i, &mut rng))
            .collect();

        let mut share_queues = FnvHashMap::default();
        for agent in agents.iter() {
            share_queues.insert(agent.id, Vec::new());
        }

        let network = Network::new(&agents, &mut rng);

        Simulation {
            network: network,
            content: Vec::new(),
            share_queues: share_queues,
            agents: agents.into_iter().map(|a| Rc::new(a)).collect(),
        }
    }

    pub fn produce(&mut self, mut rng: &mut StdRng) -> usize {
        let mut n_produced = 0;
        for a in &self.agents {
            if let Some(to_share) = self.share_queues.get_mut(&a.id) {
                match a.produce(&mut rng) {
                    Some(body) => {
                        let content = Rc::new(Content {
                            author: a.clone(),
                            body: body
                        });
                        to_share.push(SharedContent {
                            content: content.clone(),
                            sharer: a.clone()
                        });
                        self.content.push(content.clone());
                        n_produced += 1;
                    },
                    None => {}
                }
            }
        }
        n_produced
    }

    pub fn consume(&mut self, mut rng: &mut StdRng) {
        let mut new_to_share: FnvHashMap<usize, Vec<SharedContent>> = FnvHashMap::default();

        for a in &self.agents {
            let mut shared: Vec<&SharedContent> = self.network.follower_ids(&a).iter()
                .flat_map(|n_id| self.share_queues[n_id].iter()).collect();

            shared.shuffle(&mut rng);
            let will_share = a.consume(shared, &self.network, &mut rng);
            let shareable = will_share.iter().map(|content| {
                SharedContent {
                    sharer: a.clone(),
                    content: content.clone(),
                }
            }).collect();
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
    }

    pub fn n_will_share(&self) -> usize {
        self.share_queues.values().fold(0, |acc, v| acc + v.len())
    }

    pub fn n_shares(&self) -> Vec<usize> {
        // -1 to account for reference in self.content
        self.content.iter().map(|c| Rc::strong_count(c) - 1).collect()
    }

    pub fn apply_policy(&mut self, policy: &Policy) {
        // TODO
    }
}
