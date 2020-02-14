use std::rc::Rc;
use super::publisher::Publisher;
use super::agent::{Agent, Topics, Values};

#[derive(Debug)]
pub struct Content {
    pub publisher: Option<Rc<Publisher>>,
    pub author: Rc<Agent>,
    pub body: ContentBody,
}

#[derive(Debug, Copy, Clone)]
pub struct ContentBody {
    pub cost: f32,
    pub topics: Topics,
    pub values: Values,
}

#[derive(Debug)]
pub struct SharedContent {
    pub content: Rc<Content>,
    pub sharer: Rc<Agent>
}
