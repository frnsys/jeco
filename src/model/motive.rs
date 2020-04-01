use serde::{Serialize, Deserialize};
use strum_macros::{Display, EnumIter};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

#[derive(Display, EnumIter, PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Motive {
    Profit,
    Civic,
    Influence,
}

impl Distribution<Motive> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Motive {
        match rng.gen_range(0, 3) {
            0 => Motive::Profit,
            1 => Motive::Civic,
            _ => Motive::Influence,
        }
    }
}
