pub mod components;
pub mod models;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};

use crate::error::OrchestratorError;
use crate::orchestrator::TradingMessage;
use crate::types::{Aggregator as AggregatorTrait, BackgroundTask, Executable};

use self::components::{
    opportunity_detector::OpportunityDetector, orderbook_manager::OrderBookManager,
    price_normalizer::PriceNormalizer,
};
use self::models::MarketData;

pub struct Aggregator {
    name: String,
    // Receiver for data from Connectors
    connector_rx: mpsc::Receiver<MarketData>,
    // Sender for Connectors to send data to the Aggregator
    connector_tx: mpsc::Sender<MarketData>,

    // Internal components
    orderbook_manager: Arc<Mutex<OrderBookManager>>,
    price_normalizer: Arc<Mutex<PriceNormalizer>>,
    opportunity_detector: Arc<Mutex<OpportunityDetector>>,

    // event sender using TradingMessage
    event_sender: broadcast::Sender<TradingMessage>,

    // Is the aggregator running
    running: bool,
}

impl Aggregator {
    pub fn new(
        name: String,
        event_sender: broadcast::Sender<crate::orchestrator::TradingMessage>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100); // Buffer size of 100, might need adjustment

        Self {
            name,
            connector_rx: rx,
            connector_tx: tx,
            orderbook_manager: Arc::new(Mutex::new(OrderBookManager::new())),
            price_normalizer: Arc::new(Mutex::new(PriceNormalizer::new())),
            opportunity_detector: Arc::new(Mutex::new(OpportunityDetector::new())),
            event_sender,
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
            for _opportunity in opportunities {
                let _ = self.event_sender.send(TradingMessage::None);
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
impl Executable for Aggregator {
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
impl BackgroundTask for Aggregator {
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
                let _snapshot = self.create_snapshot().await?;
                // Instead of converting the snapshot string, send a unit event
                let _ = self.event_sender.send(TradingMessage::None);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl AggregatorTrait for Aggregator {
    fn market_data_sender(&self) -> mpsc::Sender<MarketData> {
        self.connector_tx.clone()
    }
}
