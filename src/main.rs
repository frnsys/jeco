mod agent;
mod content;
mod network;

use self::agent::Agent;
use self::network::Network;

fn main() {
    let n_agents = 100;
    let n_steps = 5;

    let agents: Vec<Agent> = (0..n_agents)
        .map(|i| Agent::new(i))
        .collect();

    let mut network = Network::new(agents);

    for step in 0..n_steps {
        println!("Step {:?}", step);
        network.produce();
        network.consume();
    }
}

