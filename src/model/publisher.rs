use rand::Rng;
use std::rc::Rc;
use rand::rngs::StdRng;
use itertools::Itertools;
use super::agent::Agent;
use super::content::{Content, ContentId, ContentBody};
use super::util::{Vector, Learner, Sample, SampleRow, ewma, bayes_update, z_score, sigmoid, LimitedQueue, normal_range};
use super::config::PublisherConfig;
use super::grid::Position;

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
    pub id: PublisherId,

    pub location: Position,
    pub radius: usize,

    // Budget determines how much content
    // can be published per step
    // and at what quality.
    pub budget: f32,
    revenue_per_subscriber: f32,

    // The content quality the Publisher
    // aims for. Could be replaced with something
    // more sophisticated.
    pub quality: f32,

    // How many ads the Publisher uses
    pub ads: f32,

    // Params for estimating quality/ads mix
    learner: Learner,

    // A Publisher's "reach" is the mean shared
    // count of its content per step
    pub reach: f32,

    // Number of agents subscribed to the publication.
    // These count towards the Publisher's overall budget
    // and directly received the Publisher's content
    pub subscribers: usize,

    // How much was published in the last step
    pub n_last_published: usize,

    // How many ads were sold in the last step
    pub n_ads_sold: f32,

    // Archive of content the Publisher
    // has published
    pub content: LimitedQueue<Rc<Content>>,

    // Publisher tries to guess
    // the distribution of their
    // audiences' values and interests
    pub audience: Audience,
}

impl Publisher {
    pub fn new(id: PublisherId, conf: &PublisherConfig, mut rng: &mut StdRng) -> Publisher {
        Publisher {
            id: id,

            radius: 0,
            location: (0, 0),

            budget: conf.base_budget,
            revenue_per_subscriber: conf.revenue_per_subscriber,
            reach: 0.,

            ads: rng.gen::<f32>() * 10.,
            quality: rng.gen::<f32>(),
            learner: Learner::new(50, &mut rng),
            n_ads_sold: 0.,

            content: LimitedQueue::new(50),
            subscribers: 0,
            n_last_published: 0,

            // Priors
            audience: Audience::new(&mut rng),
        }
    }

    // An Agent pitches a piece
    // of content to the publisher
    pub fn pitch(&mut self, body: &ContentBody, author: &Agent, rng: &mut StdRng) -> Option<Content> {
        if self.budget < self.quality { return None }

        let z_ints = z_score(&body.topics, &self.audience.interests);
        let z_vals = z_score(&body.values, &self.audience.values);
        let sim_to_perceived_reader = f32::max(1. - (z_ints.mean() + z_vals.mean())/6., 0.);
        // 2*3=6; 2 for the mean, max z-score of 3

        // TODO this doesn't necessarily need to be random?
        // Could just be based on a threshold
        let p_accept = sigmoid(sim_to_perceived_reader-0.5);
        let accepted = rng.gen::<f32>() < p_accept;
        if accepted {
            // Publisher improves the quality
            let mut body_ = body.clone();
            body_.quality += self.quality;
            let content = Content {
                id: ContentId::new_v4(),
                publisher: Some(self.id),
                body: body_,
                author: author.id,
                ads: self.ads
            };

            // Deduct from budget
            self.budget -= self.quality;
            Some(content)
        } else {
            None
        }
    }

    pub fn operating_budget(&self) -> f32 {
        self.subscribers as f32 * self.revenue_per_subscriber
    }

    // Update understanding of audience values/interests
    // ENH: Can be smarter about how we pick the content
    // to look at. Ideally also weight content by shares.
    pub fn audience_survey(&mut self, sample_size: usize) {
        if self.content.len() == 0 { return }
        let sample: Vec<Rc<Content>> = self.content_by_popularity().take(sample_size).cloned().collect();
        self.audience.update(sample);
    }

    pub fn content_by_popularity(&self) -> std::vec::IntoIter<&Rc<Content>> {
        self.content.iter().sorted_by(|a, b| Rc::strong_count(b).cmp(&Rc::strong_count(a)))
    }

    pub fn n_shares(&self) -> Vec<usize> {
        // -1 to account for reference in self.content
        // -1 to account for reference in Sim's self.content
        self.content.iter().map(|c| Rc::strong_count(c) - 2).collect()
    }

    pub fn update_reach(&mut self) {
        let shares = self.n_shares();
        if shares.len() == 0 {
            self.reach = 0.;
        } else {
            let mean_shares = shares.iter().fold(0, |acc, v| acc + v) as f32 / shares.len() as f32;
            self.reach = ewma(mean_shares, self.reach);
        }
    }

    pub fn learn(&mut self, revenue: f32, change_rate: f32) {
        // Assume reach has been updated
        // TODO more balanced mixture of the two?
        let outcome = revenue * self.reach;
        self.learner.learn(vec![self.quality, self.ads], outcome, change_rate);
        self.quality = self.learner.params.x;
        self.ads = self.learner.params.y;
    }
}


#[derive(Debug)]
pub struct Audience {
    pub values: (Vector, Vector),
    pub interests: (Vector, Vector),

    val_sample: Vec<SampleRow>,
    int_sample: Vec<SampleRow>,
}

impl Audience {
    pub fn new(mut rng: &mut StdRng) -> Audience {
        let mu_values = Vector::from_vec(vec![
            normal_range(&mut rng),
            normal_range(&mut rng),
        ]);
        let mu_interests = Vector::from_vec(vec![
            normal_range(&mut rng),
            normal_range(&mut rng),
        ]);
        let var = Vector::from_vec(vec![0.5, 0.5]);

        Audience {
            values: (mu_values, var.clone()),
            interests: (mu_interests, var.clone()),

            val_sample: Vec::new(),
            int_sample: Vec::new(),
        }
    }

    pub fn update<'a >(&mut self, sample: Vec<Rc<Content>>) {
        self.val_sample.clear();
        self.int_sample.clear();

        for c in sample {
            self.val_sample.push(c.body.values.transpose());
            self.int_sample.push(c.body.topics.transpose());
        }

        let mut sample = Sample::from_rows(self.val_sample.as_slice());
        self.values = bayes_update(self.values, sample);

        sample = Sample::from_rows(self.int_sample.as_slice());
        self.interests = bayes_update(self.interests, sample);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agent::{Topics, Values};

    #[test]
    fn test_audience_learning() {
        let mu = Vector::from_vec(vec![0., 0.]);
        let var = Vector::from_vec(vec![0.5, 0.5]);
        let mut audience = Audience {
            values: (mu.clone(), var.clone()),
            interests: (mu.clone(), var.clone()),

            val_sample: Vec::new(),
            int_sample: Vec::new(),
        };

        for _ in 0..10 {
            let sample: Vec<Rc<Content>> = (0..2).map(|_| {
                Rc::new(Content {
                    id: ContentId::new_v4(),
                    publisher: Some(0),
                    body: ContentBody {
                        cost: 0.,
                        quality: 0.,
                        topics: Topics::from_vec(vec![ 0., 1.]),
                        values: Values::from_vec(vec![-1., 1.]),
                    },
                    author: 0,
                    ads: 0.
                })
            }).collect();
            audience.update(sample);
        }
        assert_eq!(audience.values.0, Values::from_vec(vec![-1., 1.]));
        assert_eq!(audience.interests.0, Topics::from_vec(vec![0., 1.]));
    }
}
