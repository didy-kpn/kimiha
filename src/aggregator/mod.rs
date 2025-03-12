pub mod components;
pub mod models;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::error::OrchestratorError;
use crate::event_bus::EventBus;
use crate::types::{Aggregator as AggregatorTrait, BackgroundTask, EventType, Executable};

use self::components::{
    opportunity_detector::OpportunityDetector, orderbook_manager::OrderBookManager,
    price_normalizer::PriceNormalizer,
};
use self::models::MarketData;

pub struct Aggregator<E: EventType + 'static + ToString, M: Clone + Send + 'static = String> {
    name: String,
    // Receiver for data from Connectors
    connector_rx: mpsc::Receiver<MarketData>,
    // Sender for Connectors to send data to the Aggregator
    connector_tx: mpsc::Sender<MarketData>,

    // Internal components
    orderbook_manager: Arc<Mutex<OrderBookManager>>,
    price_normalizer: Arc<Mutex<PriceNormalizer>>,
    opportunity_detector: Arc<Mutex<OpportunityDetector>>,

    // Event bus for publishing events
    event_bus: EventBus<E, M>,

    // Market event type
    market_event: E,

    // Is the aggregator running
    running: bool,
}

impl<E: EventType + 'static + ToString, M: Clone + Send + 'static + From<String>> Aggregator<E, M> {
    pub fn new(name: String, event_bus: EventBus<E, M>, market_event: E) -> Self {
        let (tx, rx) = mpsc::channel(100); // Buffer size of 100, might need adjustment

        Self {
            name,
            connector_rx: rx,
            connector_tx: tx,
            orderbook_manager: Arc::new(Mutex::new(OrderBookManager::new())),
            price_normalizer: Arc::new(Mutex::new(PriceNormalizer::new())),
            opportunity_detector: Arc::new(Mutex::new(OpportunityDetector::new())),
            event_bus,
            market_event,
            running: false,
        }
    }

    // Process a market data update
    async fn process_market_data(
        &mut self,
        market_data: MarketData,
    ) -> Result<(), OrchestratorError> {
        // Update the orderbook
        let mut orderbook_manager = self.orderbook_manager.lock().await;
        orderbook_manager.update(market_data.clone())?;

        // Normalize prices
        let price_normalizer = self.price_normalizer.lock().await;
        let normalized_data = price_normalizer.normalize(market_data)?;

        // Detect opportunities
        let mut opportunity_detector = self.opportunity_detector.lock().await;
        let opportunities = opportunity_detector.detect(normalized_data)?;

        // If opportunities were found, publish them as events
        if !opportunities.is_empty() {
            let sender = self.event_bus.clone_sender(&self.market_event)?;

            for opportunity in opportunities {
                let serialized = serde_json::to_string(&opportunity)
                    .map_err(|e| OrchestratorError::SerializationError(e.to_string()))?;

                // Ignore errors here, as receivers might have dropped
                let _ = sender.send(M::from(serialized));
            }
        }

        Ok(())
    }

    // Create a snapshot of current market state
    async fn create_snapshot(&self) -> Result<String, OrchestratorError> {
        let orderbook_manager = self.orderbook_manager.lock().await;
        let snapshot = orderbook_manager.create_snapshot()?;

        serde_json::to_string(&snapshot)
            .map_err(|e| OrchestratorError::SerializationError(e.to_string()))
    }
}

#[async_trait]
impl<E: EventType + 'static + ToString, M: Clone + Send + 'static + From<String>> Executable for Aggregator<E, M> {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        // Initialize all components
        self.orderbook_manager.lock().await.initialize()?;
        self.price_normalizer.lock().await.initialize()?;
        self.opportunity_detector.lock().await.initialize()?;

        self.running = true;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.running = false;

        // Shutdown all components
        self.orderbook_manager.lock().await.shutdown()?;
        self.price_normalizer.lock().await.shutdown()?;
        self.opportunity_detector.lock().await.shutdown()?;

        Ok(())
    }
}

#[async_trait]
impl<E: EventType + 'static + ToString, M: Clone + Send + 'static + From<String>> BackgroundTask for Aggregator<E, M> {
    async fn execute(&mut self) -> Result<(), OrchestratorError> {
        if !self.running {
            return Ok(());
        }

        // Try to receive market data from connectors
        if let Ok(market_data) = self.connector_rx.try_recv() {
            self.process_market_data(market_data).await?;
        }

        // Periodically create a snapshot (for example, every 100 executions)
        // This is just a placeholder for actual implementation
        static mut COUNTER: usize = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER % 100 == 0 {
                let snapshot = self.create_snapshot().await?;
                // Do something with the snapshot, e.g., publish it as an event
                let sender = self.event_bus.clone_sender(&self.market_event)?;
                let _ = sender.send(M::from(snapshot));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<E: EventType + 'static + ToString, M: Clone + Send + 'static + From<String>> AggregatorTrait for Aggregator<E, M> {
    fn market_data_sender(&self) -> mpsc::Sender<MarketData> {
        self.connector_tx.clone()
    }
}
