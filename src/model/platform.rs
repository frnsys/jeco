use fnv::FnvHashMap;
use super::agent::{Agent, AgentId};
use petgraph::stable_graph::{NodeIndex, EdgeIndex, StableGraph};
use petgraph::{Directed, Outgoing};
use petgraph::visit::EdgeRef;
use super::util;

pub type PlatformId = usize;

// A Platform provides a way for
// content to circulate more rapidly.
// It doesn't publish any content itself
#[derive(Debug)]
pub struct Platform {
    pub id: PlatformId,
    pub data: f32,
    pub conversion_rate: f32,
    graph: StableGraph<usize, f32, Directed>,
    agent_to_node: FnvHashMap<AgentId, NodeIndex>,
}

impl Platform {
    pub fn new(id: PlatformId) -> Platform {
        let graph = StableGraph::<usize, f32, Directed, u32>::default();
        let lookup = FnvHashMap::default();
        Platform {
            id: id,
            graph: graph,
            agent_to_node: lookup,
            conversion_rate: 0.,
            data: 0.,
        }
    }

    pub fn signup(&mut self, agent: &Agent) {
        let idx = self.graph.add_node(agent.id);
        self.agent_to_node.insert(agent.id, idx);
    }

    pub fn is_signed_up(&self, a_id: &usize) -> bool {
        self.agent_to_node.contains_key(a_id)
    }

    pub fn follow(&mut self, a_id: &usize, b_id: &usize, weight: f32) {
        let idx_a = self.agent_to_node[a_id];
        let idx_b = self.agent_to_node[b_id];
        self.graph.add_edge(idx_a, idx_b, weight);
    }

    pub fn unfollow(&mut self, a_id: &usize, b_id: &usize) {
        let idx_a = self.agent_to_node[a_id];
        let idx_b = self.agent_to_node[b_id];

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

    pub fn following_ids(&self, agent: &Agent) -> Vec<&usize> { //impl Iterator<Item=&usize> {
        let idx = self.agent_to_node[&agent.id];
        self.graph
            .neighbors_directed(idx, Outgoing)
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    pub fn n_users(&self) -> usize {
        self.graph.node_count()
    }

    pub fn update_conversion_rate(&mut self, max_conversion_rate: f32) {
        self.conversion_rate = util::sigmoid(self.data-0.5) * max_conversion_rate;
    }
}
