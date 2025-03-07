use std::collections::{HashMap, HashSet};

use crate::aggregator::models::{
    MarketDataType, NormalizedData, NormalizedValue, Opportunity, OpportunityType,
};
use crate::error::OrchestratorError;

#[derive(Debug)]
pub struct OpportunityDetector {
    // Latest market data for each exchange/symbol
    latest_data: HashMap<String, HashMap<String, NormalizedData>>, // exchange -> symbol -> data
    // Minimum profit threshold for opportunities (percentage)
    min_profit_threshold: f64,
    initialized: bool,
}

impl Default for OpportunityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OpportunityDetector {
    pub fn new() -> Self {
        Self {
            latest_data: HashMap::new(),
            min_profit_threshold: 0.5, // 0.5% by default
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

    pub fn detect(
        &mut self,
        normalized_data: NormalizedData,
    ) -> Result<Vec<Opportunity>, OrchestratorError> {
        if !self.initialized {
            return Err(OrchestratorError::NotInitialized("OpportunityDetector"));
        }

        // Update the latest data
        let exchange_map = self
            .latest_data
            .entry(normalized_data.exchange.clone())
            .or_default();

        exchange_map.insert(normalized_data.symbol.clone(), normalized_data.clone());

        // Detect opportunities based on data type
        match normalized_data.data_type {
            MarketDataType::OrderBook => self.detect_arbitrage_opportunities(&normalized_data),
            MarketDataType::Ticker => self.detect_spread_opportunities(&normalized_data),
            MarketDataType::Trade | MarketDataType::Candle => Ok(Vec::new()), // Not implemented yet
        }
    }

    fn detect_arbitrage_opportunities(
        &self,
        data: &NormalizedData,
    ) -> Result<Vec<Opportunity>, OrchestratorError> {
        let mut opportunities = Vec::new();

        // Extract relevant information from the normalized data
        let _exchange = &data.exchange;
        let symbol = &data.symbol;

        // Early return if we only have one exchange with data for this symbol
        let exchanges_with_data: HashSet<_> = self
            .latest_data
            .keys()
            .filter(|&ex| {
                self.latest_data
                    .get(ex)
                    .and_then(|map| map.get(symbol))
                    .is_some()
            })
            .collect();

        if exchanges_with_data.len() <= 1 {
            return Ok(opportunities);
        }

        // Get the best bid and ask from each exchange
        let mut best_bids: HashMap<String, f64> = HashMap::new();
        let mut best_asks: HashMap<String, f64> = HashMap::new();

        for ex in &exchanges_with_data {
            if let Some(ex_data) = self.latest_data.get(*ex) {
                if let Some(sym_data) = ex_data.get(symbol) {
                    if let NormalizedValue::OrderBook(ref orderbook) = sym_data.normalized_value {
                        // Find the best bid (highest price a buyer is willing to pay)
                        if let Some(best_bid) = orderbook.bids.first() {
                            best_bids.insert(ex.to_string(), best_bid.price);
                        }

                        // Find the best ask (lowest price a seller is willing to accept)
                        if let Some(best_ask) = orderbook.asks.first() {
                            best_asks.insert(ex.to_string(), best_ask.price);
                        }
                    }
                }
            }
        }

        // Look for arbitrage opportunities (buy low on one exchange, sell high on another)
        for (ex1, ask_price) in &best_asks {
            for (ex2, bid_price) in &best_bids {
                if ex1 == ex2 {
                    continue; // Skip same exchange
                }

                // Calculate potential profit (percentage)
                let potential_profit = (bid_price - ask_price) / ask_price * 100.0;

                // If profit is above threshold, we found an opportunity
                if potential_profit > self.min_profit_threshold {
                    let details = format!(
                        "Buy on {} at {:.2}, sell on {} at {:.2}, profit: {:.2}%",
                        ex1, ask_price, ex2, bid_price, potential_profit
                    );

                    opportunities.push(Opportunity {
                        opportunity_type: OpportunityType::Arbitrage,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        symbols: vec![symbol.clone()],
                        exchanges: vec![ex1.clone(), ex2.clone()],
                        potential_profit,
                        details,
                    });
                }
            }
        }

        Ok(opportunities)
    }

    fn detect_spread_opportunities(
        &self,
        data: &NormalizedData,
    ) -> Result<Vec<Opportunity>, OrchestratorError> {
        let mut opportunities = Vec::new();

        // For spread opportunities, we look at a single exchange and see if there's a significant spread
        if let NormalizedValue::Ticker(ref ticker) = data.normalized_value {
            // Calculate the spread percentage
            let spread_percentage = (ticker.ask - ticker.bid) / ticker.bid * 100.0;

            // If spread is above threshold, we found an opportunity
            if spread_percentage > self.min_profit_threshold * 2.0 {
                // Higher threshold for spreads
                let details = format!(
                    "Significant spread on {} for {}: bid={:.2}, ask={:.2}, spread={:.2}%",
                    data.exchange, data.symbol, ticker.bid, ticker.ask, spread_percentage
                );

                opportunities.push(Opportunity {
                    opportunity_type: OpportunityType::Spread,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    symbols: vec![data.symbol.clone()],
                    exchanges: vec![data.exchange.clone()],
                    potential_profit: spread_percentage,
                    details,
                });
            }
        }

        Ok(opportunities)
    }
}
