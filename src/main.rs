mod model;
mod config;
mod output;

use self::output::Recorder;
use self::model::Simulation;
use pbr::ProgressBar;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() {
    let conf = config::load_config();
    let debug = conf.debug;
    let steps = conf.steps;
    let mut rng: StdRng = SeedableRng::seed_from_u64(conf.seed);
    let mut sim = Simulation::new(conf.population, &mut rng);

    if debug {
        let mut recorder = Recorder::new(&sim, &mut rng);
        recorder.record(&sim, 0);

        let mut pb = ProgressBar::new(steps as u64);
        for _ in 0..steps {
            let n_produced = sim.produce(&mut rng);
            sim.consume(&mut rng);

            recorder.record(&sim, n_produced);
            pb.inc();
        }
        recorder.save(&conf);
    } else {
        for _ in 0..steps {
            sim.produce(&mut rng);
            sim.consume(&mut rng);
        }
    }
}
