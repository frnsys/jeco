use std::rc::Rc;
use std::cell::RefCell;
use super::agent::{Agent, Topics, Values};

#[derive(Debug)]
pub struct Content {
    pub author: Rc<RefCell<Agent>>,
    pub body: ContentBody,
}

#[derive(Debug)]
pub struct ContentBody {
    pub cost: f32,
    pub topics: Topics,
    pub values: Values,
}


#[derive(Debug)]
pub struct SharedContent {
    pub content: Rc<Content>,
    pub sharer: Rc<RefCell<Agent>>
}
