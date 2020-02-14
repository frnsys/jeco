// TODO
// - implement pitching
//  - [X] publisher deciding when to accept a pitch
//  - [X] publisher deciding how much to invest in a pitch
//  - agent deciding when and where to pitch
//      - based on pitch topics/values
//      - EV calculation of expected reach and prob of accepting
// - implement publishing
//  - add content to all subscriber inboxes
//  - add content through agent networks/non-subscriber readers
// - implement audience interest/value tracking (EWMAs et al)
// - implement subscribing
//  - agents deciding when to subscribe
//
// After ads are implemented
// - deciding on how many ads to include
//  - civic vs profit-driven utility mixture

use rand::Rng;
use std::rc::Rc;
use rand::rngs::StdRng;
use std::f32::consts::E;
use super::agent::Agent;
use super::content::{Content, ContentBody};

static REVENUE_PER_SUBSCRIBER: f32 = 0.01;

pub type PublisherId = usize;

// A Publisher is a platform which
// exercises discretion of what
// content circulates through it.
// When content is circulated from a Publisher,
// it carries the reputation of the Publisher.
// (In contrast, on a Platform the content
// that circulates through it carries
// the reputation of the sender)
#[derive(Debug)]
pub struct Publisher {
    id: PublisherId,

    // Budget determines how much content
    // can be published per step
    // and at what quality.
    budget: f32,

    // The content quality the Publisher
    // aims for. Could be replaced with something
    // more sophisticated.
    quality: f32,

    // Agents subscribed to the publication.
    // These count towards the Publisher's overall budget
    // and directly received the Publisher's content
    subscribers: Vec<Rc<Agent>>,

    // Store content the Publisher will
    // publish in the next step. Emptied each step.
    pub outbox: Vec<Rc<Content>>,
}

impl Publisher {
    pub fn new(id: PublisherId, rng: &mut StdRng) -> Publisher {
        Publisher {
            id: id,
            budget: 0.,
            quality: rng.gen(),
            outbox: Vec::new(),
            subscribers: Vec::new()
        }
    }

    // An Agent pitches a piece
    // of content to the publisher
    pub fn pitch(&mut self, body: &ContentBody, author: Rc<Agent>, rng: &mut StdRng) -> bool {
        if self.budget < self.quality { return false }

        // TODO
        let sim_to_perceived_reader = 0.5;

        // Sigmoid
        let p_accept = 1./(1.+E.powf(-sim_to_perceived_reader-0.5));
        let roll: f32 = rng.gen();
        let accepted = roll < p_accept;
        if accepted {
            let content = Content {
                publisher: Some(self.id),
                body: *body,
                author: author.id,
            };
            self.outbox.push(Rc::new(content));

            // Deduct from budget
            self.budget -= self.quality;
        }
        accepted
    }

    // An Agent subscribes to the publisher
    pub fn subscribe(&mut self, agent: Rc<Agent>) {
        self.subscribers.push(agent.clone());
    }

    fn operating_budget(&self) -> f32 {
        self.subscribers.len() as f32 * REVENUE_PER_SUBSCRIBER
    }
}
