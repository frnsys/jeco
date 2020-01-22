mod agent;
mod config;
mod content;
mod network;

use self::agent::Agent;
use self::network::Network;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() {
    let conf = config::load_config();
    let debug = conf.debug;
    let steps = conf.steps;
    let mut rng: StdRng = SeedableRng::seed_from_u64(conf.seed);

    let agents: Vec<Agent> = (0..conf.population)
        .map(|i| Agent::new(i))
        .collect();

    let mut network = Network::new(agents);

    for step in 0..steps {
        println!("Step {:?}", step);
        network.produce();
        network.consume();
    }
}

