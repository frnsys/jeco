mod model;
mod config;
mod control;
mod rec;

use self::rec::Recorder;
use self::control::{Commander, Command};
use self::model::Simulation;
use pbr::ProgressBar;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() {
    let conf = config::load_config();
    let mut rng: StdRng = SeedableRng::seed_from_u64(conf.seed);

    let steps = conf.steps;
    let debug = conf.debug;

    // Interactive mode
    if conf.command {
        let redis_host = "redis://127.0.0.1/1";
        let mut command = Commander::new(redis_host);

        loop {
            command.reset().unwrap();
            command.set_loading().unwrap();
            let mut sim = Simulation::new(conf.population, &mut rng);
            let mut recorder = Recorder::new(&sim, &mut rng);
            recorder.record(&sim, 0);
            command.set_ready().unwrap();

            // Blocks until a run command is received;
            // will process other commands while waiting
            match command.wait_for_command() {
                Command::Run(steps) => {
                    command.set_running().unwrap();
                    for _ in 0..steps {
                        let n_produced = sim.produce(&mut rng);
                        sim.consume(&mut rng);

                        recorder.record(&sim, n_produced);
                        recorder.sync(redis_host).unwrap();
                    }
                    command.set_ready().unwrap();
                },
                Command::Reset => {
                    break;
                }
            }
        }

    // Single run mode
    } else {
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
}
