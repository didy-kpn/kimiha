use std::collections::HashMap;

use tokio::sync::broadcast;

use crate::{channel_config::ChannelConfig, error::OrchestratorError, types::EventType};

#[derive(Clone)]
pub struct EventBus<E> {
    channels: HashMap<E, broadcast::Sender<String>>,
    configs: Vec<(E, ChannelConfig)>,
}

impl<E: EventType + 'static + ToString> EventBus<E> {
    pub fn new(configs: Vec<(E, ChannelConfig)>) -> Self {
        Self {
            channels: configs
                .iter()
                .map(|c| (c.0.clone(), broadcast::channel(c.1.capacity()).0))
                .collect(),
            configs: configs
                .iter()
                .map(|(e, c)| (e.clone(), c.clone()))
                .collect(),
        }
    }

    pub fn add_channel(mut self, event: E, config: ChannelConfig) -> Self {
        self.channels
            .insert(event.clone(), broadcast::channel(config.capacity()).0);
        self.configs.push((event, config));
        self
    }

    pub fn subscribe(
        &self,
        channel_event: &E,
    ) -> Result<broadcast::Receiver<String>, OrchestratorError> {
        self.channels
            .get(channel_event)
            .ok_or(OrchestratorError::InvalidChannel(channel_event.to_string()))
            .map(|sender| sender.subscribe())
    }

    pub fn clone_sender(
        &self,
        channel_event: &E,
    ) -> Result<broadcast::Sender<String>, OrchestratorError> {
        self.channels
            .get(channel_event)
            .ok_or(OrchestratorError::InvalidChannel(channel_event.to_string()))
            .cloned()
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn channels(&self) -> &Vec<(E, ChannelConfig)> {
        &self.configs
    }
}
