use super::agent::AgentId;
use super::network::Network;
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
    network: Network,
}

impl Platform {
    pub fn new(id: PlatformId) -> Platform {
        let network = Network::new();
        Platform {
            id: id,
            network: network,
            data: 0.,
            conversion_rate: 0.,
        }
    }

    pub fn signup(&mut self, a: AgentId) {
        self.network.add(a);
    }

    pub fn is_signed_up(&self, a: &AgentId) -> bool {
        self.network.exists(a)
    }

    pub fn unfollow(&mut self, a: &AgentId, b: &AgentId) {
        self.network.remove_edges(a, b);
    }

    pub fn follow(&mut self, a: &AgentId, b: &AgentId, weight: f32) {
        self.network.add_edge(a, b, weight);
    }

    pub fn following_ids(&self, a: &AgentId) -> Vec<&usize> {
        self.network.following_ids(a)
    }

    pub fn n_users(&self) -> usize {
        self.network.n_nodes()
    }

    pub fn n_followers(&self) -> Vec<usize> {
        self.network.n_followers()
    }

    pub fn update_conversion_rate(&mut self, max_conversion_rate: f32) {
        self.conversion_rate = util::sigmoid(self.data-0.5) * max_conversion_rate;
    }
}
