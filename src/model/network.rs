use super::agent::{Agent, AgentId};
use fnv::FnvHashMap;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{Directed, Incoming, Outgoing};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

static NETWORK_SAMPLE_P: f32 = 0.2;

pub struct Network {
    graph: StableGraph<usize, f32, Directed>, // TODO make this undirected
    pub agent_to_node: FnvHashMap<AgentId, NodeIndex>,
}

impl Network {
    pub fn new(agents: &Vec<Agent>, mut rng: &mut StdRng) -> Network {
        // Network of agents, with trust as weight
        let sample_size = (agents.len() as f32 * NETWORK_SAMPLE_P).floor() as usize;
        let mut graph = StableGraph::<usize, f32, Directed, u32>::default();
        let mut lookup = FnvHashMap::default();
        for agent in agents.iter() {
            let idx = graph.add_node(agent.id);
            lookup.insert(agent.id, idx);
        }

        // Social network formation (preferential attachment)
        let mut total_edges = 1.;
        for agent in agents.iter() {
            let idx = lookup[&agent.id];
            let candidates = agents.choose_multiple(&mut rng, sample_size);
            for candidate in candidates {
                let roll: f32 = rng.gen();
                let c_idx = lookup[&candidate.id];

                let sim = agent.similarity(&candidate);
                let pref = (graph.neighbors_directed(c_idx, Incoming).count() as f32) / total_edges;
                let same_location = if agent.location == candidate.location {
                    1.
                } else {
                    0.
                };
                let p = (sim + pref + same_location) / 3.;
                if roll < p {
                    graph.add_edge(idx, c_idx, sim);
                    total_edges += 1.;
                }
            }
        }

        Network {
            graph: graph,
            agent_to_node: lookup,
        }
    }

    pub fn trust(&self, a: &AgentId, b: &AgentId) -> f32 {
        // Edge from A->B
        let idx_a = self.agent_to_node[a];
        let idx_b = self.agent_to_node[b];

        match self.graph.find_edge(idx_a, idx_b) {
            Some(edge) => match self.graph.edge_weight(edge) {
                Some(weight) => *weight,
                None => 0.,
            },
            None => 0.,
        }
    }

    pub fn n_followers(&self) -> Vec<usize> {
        self.graph
            .node_indices()
            .map(|idx| self.graph.neighbors_directed(idx, Incoming).count())
            .collect()
    }

    pub fn following_ids(&self, agent: &Agent) -> Vec<&usize> { //impl Iterator<Item=&usize> {
        let idx = self.agent_to_node[&agent.id];
        self.graph
            .neighbors_directed(idx, Outgoing)
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    pub fn follow(&mut self, a: &Agent, b: &Agent, weight: f32) {
        let idx_a = self.agent_to_node[&a.id];
        let idx_b = self.agent_to_node[&b.id];
        self.graph.add_edge(idx_a, idx_b, weight);
    }
}
