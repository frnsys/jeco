use rand::Rng;
use fnv::FnvHashMap;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use petgraph::{Directed, Incoming};
use petgraph::stable_graph::{StableGraph, NodeIndex};
use super::agent::Agent;

pub struct Network<'a> {
    agents: &'a Vec<Agent>,
    graph: StableGraph<usize, f32, Directed>,
    agent_to_node: FnvHashMap<usize, NodeIndex>,
}

impl<'a> Network<'a> {
    pub fn new(agents: &Vec<Agent>) -> Network {
        let sample_size = 10;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        // Network of agents, with trust as weight
        let mut graph = StableGraph::<usize, f32, Directed, u32>::default();
        let mut lookup = FnvHashMap::default();
        for agent in agents.iter() {
            let idx = graph.add_node(agent.id);
            lookup.insert(agent.id, idx);
        }

        // Social network formation (preferential attachment)
        let mut total_edges = 0.;
        for agent in agents.iter() {
            let idx = lookup[&agent.id];
            let candidates = agents.choose_multiple(&mut rng, sample_size);
            for candidate in candidates {
                let roll: f32 = rng.gen();
                let c_idx = lookup[&candidate.id];

                let sim = agent.similarity(&candidate);
                let pref = (graph.neighbors_directed(c_idx, Incoming).count() as f32)/total_edges;
                if roll < (sim + pref)/2. {
                    graph.add_edge(idx, c_idx, sim);
                    total_edges += 1.;
                }
            }
        }

        Network {
            graph: graph,
            agents: agents,
            agent_to_node: lookup
        }
    }
}
