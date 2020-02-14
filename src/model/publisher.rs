// TODO
// - implement pitching
//  - publisher deciding when to accept a pitch
//  - publisher deciding how much to invest in a pitch
//  - agent deciding when and where to pitch
//      - based on pitch topics/values
//      - EV calculation of expected reach and prob of accepting
// - implement publishing
//  - add content to all subscriber inboxes
//  - add content through agent networks/non-subscriber readers
// - implement audience interest/value tracking (EWMAs et al)
// - implement subscribing
//  - when agents decide to subscribe
//
// After ads are implemented
// - deciding on how many ads to include
//  - civic vs profit-driven utility mixture

use std::rc::Rc;
use super::agent::Agent;

// A Publisher is a platform which
// exercises discretion of what
// content circulates through it.
// When content is circulated from a Publisher,
// it carries the reputation of the Publisher.
// (In contrast, on a Platform the content
// that circulates through it carries
// the reputation of the sender)
struct Publisher {
    budget: f32,
    subscribers: Vec<Rc<Agent>>
}

impl Publisher {
    pub fn new() -> Publisher {
        Publisher {
            budget: 0.,
            subscribers: Vec::new()
        }
    }

    // An Agent pitches a piece
    // of content to the publisher
    pub fn pitch(&mut self) -> bool {
        // TODO
        false
    }

    // An Agent subscribes to the publisher
    pub fn subscribe(&mut self, agent: Rc<Agent>) {
        self.subscribers.push(agent.clone());
    }
}
