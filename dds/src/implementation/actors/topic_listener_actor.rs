use dust_dds_derive::actor_interface;

use crate::{
    dds_async::{topic::TopicAsync, topic_listener::TopicListenerAsync},
    infrastructure::status::InconsistentTopicStatus,
};

pub struct TopicListenerActor {
    listener: Option<Box<dyn TopicListenerAsync + Send>>,
}

impl TopicListenerActor {
    pub fn new(listener: Option<Box<dyn TopicListenerAsync + Send>>) -> Self {
        Self { listener }
    }
}

#[actor_interface]
impl TopicListenerActor {
    async fn on_inconsistent_topic(
        &mut self,
        the_topic: TopicAsync,
        status: InconsistentTopicStatus,
    ) {
        if let Some(l) = &mut self.listener {
            l.on_inconsistent_topic(the_topic, status).await
        }
    }
}