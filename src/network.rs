use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;
use fnv::FnvHashMap;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use petgraph::{Directed, Incoming, Outgoing};
use petgraph::stable_graph::{StableGraph, NodeIndex};
use super::agent::Agent;
use super::content::{Content, SharedContent};

pub struct Network {
    pub agents: Vec<Rc<RefCell<Agent>>>,
    graph: StableGraph<usize, f32, Directed>,
    agent_to_node: FnvHashMap<usize, NodeIndex>,
    agent_to_share: FnvHashMap<usize, Vec<SharedContent>>,
}

impl Network {
    pub fn new(agents: Vec<Agent>) -> Network {
        let sample_size = 10;
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        // Network of agents, with trust as weight
        let mut graph = StableGraph::<usize, f32, Directed, u32>::default();
        let mut lookup = FnvHashMap::default();
        let mut agent_to_share = FnvHashMap::default();
        for agent in agents.iter() {
            let idx = graph.add_node(agent.id);
            lookup.insert(agent.id, idx);
            agent_to_share.insert(agent.id, Vec::new());
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
            agents: agents.into_iter().map(|a| Rc::new(RefCell::new(a))).collect(),
            agent_to_node: lookup,
            agent_to_share: agent_to_share,
        }
    }

    pub fn trust(&self, a: &Agent, b: &Agent) -> f32 {
        // Edge from A->B
        let idx_a  = self.agent_to_node[&a.id];
        let idx_b  = self.agent_to_node[&b.id];

        match self.graph.find_edge(idx_a, idx_b) {
            Some(edge) => match self.graph.edge_weight(edge) {
                Some(weight) => *weight,
                None => 0.
            },
            None => 0.
        }
    }

    pub fn produce(&mut self) {
        for ag in &self.agents {
            let a = ag.borrow();
            if let Some(to_share) = self.agent_to_share.get_mut(&a.id) {
                match a.produce() {
                    Some(body) => {
                        let content = Rc::new(Content {
                            author: ag.clone(),
                            body: body
                        });
                        to_share.push(SharedContent {
                            content: content.clone(),
                            sharer: ag.clone()
                        });
                    },
                    None => {}
                }
            }
        }
    }

    pub fn consume(&mut self) {
        // TODO
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        let mut new_to_share: FnvHashMap<usize, Vec<SharedContent>> = FnvHashMap::default();

        for ag in &self.agents {
            let mut a = ag.borrow_mut();
            let idx = self.agent_to_node[&a.id];
            let neighbs = self.graph.neighbors_directed(idx, Outgoing).filter_map(|idx| self.graph.node_weight(idx));
            let mut shared: Vec<&SharedContent> = neighbs.flat_map(|n_id| self.agent_to_share[n_id].iter()).collect();

            shared.shuffle(&mut rng);
            let will_share = a.consume(shared, &self);
            let shareable = will_share.iter().map(|content| {
                SharedContent {
                    sharer: ag.clone(),
                    content: content.clone(),
                }
            }).collect();
            new_to_share.insert(a.id, shareable);
        }

        // Update share lists
        for (a_id, mut to_share_) in new_to_share {
            match self.agent_to_share.get_mut(&a_id) {
                Some(to_share) => {
                    to_share.clear();
                    to_share.append(&mut to_share_);
                },
                None => {
                    self.agent_to_share.insert(a_id, to_share_);
                }
            }
        }
    }
}
