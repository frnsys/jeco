use super::agent::{Agent, AgentId};
use fnv::FnvHashMap;
use petgraph::visit::EdgeRef;
use petgraph::stable_graph::{NodeIndex, EdgeIndex, StableGraph};
use petgraph::{Directed, Incoming, Outgoing};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug)]
pub struct Network {
    graph: StableGraph<usize, f32, Directed>,
    pub lookup: FnvHashMap<AgentId, NodeIndex>,
}

impl Network {
    pub fn new() -> Network {
        Network {
            graph: StableGraph::<usize, f32, Directed, u32>::default(),
            lookup: FnvHashMap::default(),
        }
    }

    pub fn preferential_attachment(&mut self, agents: &Vec<Agent>, sample_p: f32, mut rng: &mut StdRng) {
        // Network of agents, with trust as weight
        let sample_size = (agents.len() as f32 * sample_p).floor() as usize;
        for agent in agents.iter() {
            let idx = self.graph.add_node(agent.id);
            self.lookup.insert(agent.id, idx);
        }

        // Social network formation (preferential attachment)
        let mut total_edges = 1.;
        for agent in agents.iter() {
            let idx = self.lookup[&agent.id];
            let candidates = agents.choose_multiple(&mut rng, sample_size);
            for candidate in candidates {
                // Probability that two Agents know each other
                // based on:
                // 1. their similarity
                // 2. in-degree
                // 3. if they are at the same location
                let c_idx = self.lookup[&candidate.id];
                let sim = agent.similarity(&candidate);
                let pref = (self.graph.neighbors_directed(c_idx, Incoming).count() as f32) / total_edges;
                let same_location = if agent.location == candidate.location { 1. } else { 0. };
                let p = (sim + pref + same_location) / 3.;
                if rng.gen::<f32>() < p {
                    self.graph.add_edge(idx, c_idx, sim);
                    total_edges += 1.;
                }
            }
        }

    }

    pub fn add(&mut self, a: AgentId) {
        let idx = self.graph.add_node(a);
        self.lookup.insert(a, idx);
    }

    pub fn exists(&self, a: &AgentId) -> bool {
        self.lookup.contains_key(a)
    }

    pub fn trust(&self, a: &AgentId, b: &AgentId) -> f32 {
        // Edge from A->B
        let idx_a = self.lookup[a];
        let idx_b = self.lookup[b];

        match self.graph.find_edge(idx_a, idx_b) {
            Some(edge) => match self.graph.edge_weight(edge) {
                Some(weight) => *weight,
                None => 0.,
            },
            None => 0.,
        }
    }

    pub fn n_nodes(&self) -> usize {
        self.graph.node_count()
    }

    pub fn n_followers(&self) -> Vec<usize> {
        self.graph
            .node_indices()
            .map(|idx| self.graph.neighbors_directed(idx, Incoming).count())
            .collect()
    }

    pub fn following_ids(&self, a: &AgentId) -> Vec<&usize> { //impl Iterator<Item=&usize> {
        let idx = self.lookup[&a];
        self.graph
            .neighbors_directed(idx, Outgoing)
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    pub fn add_edge(&mut self, a: &AgentId, b: &AgentId, weight: f32) {
        let idx_a = self.lookup[&a];
        let idx_b = self.lookup[&b];
        self.graph.add_edge(idx_a, idx_b, weight);
    }

    pub fn remove_edges(&mut self, a: &AgentId, b: &AgentId) {
        let idx_a = self.lookup[&a];
        let idx_b = self.lookup[&b];

        // TODO is there a way to do this without
        // creating a new vec?
        let to_remove: Vec<EdgeIndex> = self.graph.edges_directed(idx_a, Outgoing)
            .filter(|edge| edge.target() == idx_b)
            .map(|edge| edge.id())
            .collect();
        for edge in to_remove {
            self.graph.remove_edge(edge);
        }
    }
}
