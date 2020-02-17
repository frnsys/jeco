// TODO
// - how users sign up
// - how users follow/unfollow new users
// - consuming on a platform generates data

use fnv::FnvHashMap;
use super::agent::{Agent, AgentId};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{Directed, Incoming, Outgoing};

pub type PlatformId = usize;

// A Platform provides a way for
// content to circulate more rapidly.
// It doesn't publish any content itself
#[derive(Debug)]
pub struct Platform {
    pub id: PlatformId,
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
            agent_to_node: lookup
        }
    }

    pub fn sign_up(&mut self, agent: &Agent) {
        let idx = self.graph.add_node(agent.id);
        self.agent_to_node.insert(agent.id, idx);
    }

    pub fn follow(&mut self, follower: &Agent, following: &Agent) {
        // TODO
    }
}
