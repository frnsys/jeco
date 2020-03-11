use super::agent::{Agent, AgentId};
use fnv::{FnvHashMap, FnvHashSet};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug)]
pub struct Network {
    incoming: FnvHashMap<AgentId, FnvHashSet<AgentId>>,
    outgoing: FnvHashMap<AgentId, FnvHashSet<AgentId>>,
    total_edges: f32,
}

impl Network {
    pub fn new() -> Network {
        Network {
            incoming: FnvHashMap::default(),
            outgoing: FnvHashMap::default(),
            total_edges: 0.,
        }
    }

    pub fn add_node(&mut self, id: usize) {
        self.incoming.insert(id, FnvHashSet::default());
        self.outgoing.insert(id, FnvHashSet::default());
    }

    pub fn add_edge(&mut self, a: &usize, b: &usize) {
        let outgoing = self.outgoing.get_mut(a).unwrap();
        outgoing.insert(*b);

        let incoming = self.incoming.get_mut(b).unwrap();
        incoming.insert(*a);

        self.total_edges += 1.;
    }

    pub fn preferential_attachment(&mut self, agents: &Vec<Agent>, max_friends: usize, mut rng: &mut StdRng) {
        let mut new = Vec::new();

        // Network of agents, with trust as weight
        for agent in agents {
            if !self.incoming.contains_key(&agent.id) {
                self.add_node(agent.id);
                new.push(agent);
            }
        }

        // Social network formation (preferential attachment)
        for agent in new {
            let idx = &agent.id;
            let sample_size = (rng.gen::<f32>() * max_friends as f32).floor() as usize;
            let candidates = agents.choose_multiple(&mut rng, sample_size);
            for candidate in candidates {
                // Probability that two Agents know each other
                // based on:
                // 1. their similarity
                // 2. in-degree
                // 3. if they are at the same location
                let c_idx = &candidate.id;
                let sim = agent.similarity(&candidate);
                let pref = (self.incoming[c_idx].len() as f32) / self.total_edges;
                let same_location = if agent.location == candidate.location { 1. } else { 0. };
                let p = (sim + pref + same_location) / 3.;
                if rng.gen::<f32>() < p {
                    self.add_edge(idx, c_idx);
                }
            }
        }

    }

    pub fn exists(&self, a: &AgentId) -> bool {
        // Added to both outgoing/incoming,
        // only need to check one
        self.incoming.contains_key(a)
    }

    pub fn n_nodes(&self) -> usize {
        // Added to both outgoing/incoming,
        // only need to check one
        self.incoming.len()
    }

    pub fn n_followers(&self) -> Vec<usize> {
        self.incoming.values().map(|v| v.len()).collect()
    }

    pub fn following_ids(&self, a: &AgentId) -> &FnvHashSet<usize> { //impl Iterator<Item=&usize> {
        &self.outgoing[a]
    }

    pub fn remove_edges(&mut self, a: &AgentId, b: &AgentId) {
        let outgoing = self.outgoing.get_mut(a).unwrap();
        outgoing.remove(b);

        let incoming = self.incoming.get_mut(b).unwrap();
        incoming.remove(a);
    }
}
