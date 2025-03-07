use std::collections::HashMap;

use crate::aggregator::models::{
    MarketData, MarketDataType, MarketDataValue, MarketSnapshot, OrderBookData,
};
use crate::error::OrchestratorError;

#[derive(Debug)]
pub struct OrderBookManager {
    // Exchange -> Symbol -> OrderBookData
    orderbooks: HashMap<String, HashMap<String, OrderBookData>>,
    initialized: bool,
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            orderbooks: HashMap::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), OrchestratorError> {
        self.initialized = true;
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.initialized = false;
        Ok(())
    }

    pub fn update(&mut self, market_data: MarketData) -> Result<(), OrchestratorError> {
        if !self.initialized {
            return Err(OrchestratorError::NotInitialized("OrderBookManager"));
        }

        if market_data.data_type != MarketDataType::OrderBook {
            return Ok(()); // Ignore non-orderbook data
        }

        match market_data.data {
            MarketDataValue::OrderBook(orderbook_data) => {
                // Get or create the exchange map
                let exchange_map = self
                    .orderbooks
                    .entry(market_data.exchange.clone())
                    .or_default();

                // Update the orderbook
                exchange_map.insert(market_data.symbol.clone(), orderbook_data);

                Ok(())
            }
            _ => Ok(()), // Should not happen due to the check above
        }
    }

    pub fn get_orderbook(&self, exchange: &str, symbol: &str) -> Option<&OrderBookData> {
        self.orderbooks
            .get(exchange)
            .and_then(|exchange_map| exchange_map.get(symbol))
    }

    pub fn create_snapshot(&self) -> Result<MarketSnapshot, OrchestratorError> {
        if !self.initialized {
            return Err(OrchestratorError::NotInitialized("OrderBookManager"));
        }

        // Create a deep copy of the orderbooks
        let orderbooks = self.orderbooks.clone();

        // Create an empty ticker map (to be filled by other components)
        let tickers = HashMap::new();

        Ok(MarketSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            orderbooks,
            tickers,
        })
    }
}
