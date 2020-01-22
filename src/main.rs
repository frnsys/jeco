mod agent;
mod content;
mod network;

use self::agent::Agent;
use self::network::Network;

fn main() {
    let n_agents = 10;
    let n_steps = 5;

    let agents: Vec<Agent> = (0..n_agents)
        .map(|i| Agent::new(i))
        .collect();

    let network = Network::new(&agents);

    for step in 0..n_steps {
        println!("Step {:?}", step);

        // Produce content
        for h in &agents {
            match h.produce() {
                Some(content) => {
                    println!("{:?}", content);
                }
                None => {
                }
            }
        }
        // Consume content
        // TODO each human looks at neighbors in network
        // and decides what to consume
        // for h in &agents {
            // h.consume();
        // }
    }
}

