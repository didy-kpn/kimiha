use std::hash::Hash;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::aggregator::models::MarketData;
use crate::error::OrchestratorError;

pub trait EventType: Send + Sync + Clone + Hash + Eq {}

#[async_trait]
pub trait Executable: Send + Sync {
    fn name(&self) -> &str;

    async fn initialize(&mut self) -> Result<(), OrchestratorError>;
    async fn shutdown(&mut self) -> Result<(), OrchestratorError>;
}

#[async_trait]
pub trait BackgroundTask: Executable {
    async fn execute(&mut self) -> Result<(), OrchestratorError>;
}

#[async_trait]
pub trait EventTask<E: EventType + 'static, M: Clone + Send + 'static>: Executable {
    fn subscribed_event(&self) -> &E;
    async fn handle_event(&mut self, event: M) -> Result<(), OrchestratorError>;
}

#[async_trait]
pub trait Connector: BackgroundTask {}

#[async_trait]
pub trait Aggregator: BackgroundTask {
    fn market_data_sender(&self) -> mpsc::Sender<MarketData>;
}

#[async_trait]
pub trait Strategy<E: EventType + 'static, M: Clone + Send + 'static>: EventTask<E, M> {}

#[async_trait]
pub trait Executor<E: EventType + 'static, M: Clone + Send + 'static>: EventTask<E, M> {}
