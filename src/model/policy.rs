use serde::Deserialize;
use strum_macros::{Display};

#[derive(Display, PartialEq, Debug, Deserialize)]
pub enum Policy {
    Foo,
}
