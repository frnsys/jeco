use serde::{Serialize, Deserialize};
use strum_macros::{Display, EnumIter};

#[derive(Display, EnumIter, PartialEq, Debug, Serialize, Deserialize)]
pub enum Policy {
    PopulationChange(isize),
    SubsidizeProduction(f32),
    TaxAdvertising(f32),
    FoundPlatform,
}
