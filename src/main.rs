mod agent;
mod config;
mod content;
mod network;
mod output;

use self::agent::Agent;
use self::network::Network;
use rand::rngs::StdRng;
use rand::SeedableRng;
use pbr::ProgressBar;
use self::output::Recorder;

fn main() {
    let conf = config::load_config();
    let debug = conf.debug;
    let steps = conf.steps;
    let mut rng: StdRng = SeedableRng::seed_from_u64(conf.seed);

    let agents: Vec<Agent> = (0..conf.population)
        .map(|i| Agent::new(i))
        .collect();

    let mut network = Network::new(agents);

    if debug {
        let mut recorder = Recorder::new(&network, &mut rng);
        recorder.record(&network, 0);

        let mut pb = ProgressBar::new(steps as u64);
        for _ in 0..steps {
            let n_produced = network.produce();
            network.consume();

            recorder.record(&network, n_produced);
            pb.inc();
        }
        recorder.save(&conf);
    } else {
        for _ in 0..steps {
            network.produce();
            network.consume();
        }
    }
}

