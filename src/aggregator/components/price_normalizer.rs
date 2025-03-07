use std::collections::HashMap;

use crate::aggregator::models::{
    CandleData, MarketData, MarketDataValue, NormalizedData, NormalizedValue, OrderBookData,
    TickerData, TradeData,
};
use crate::error::OrchestratorError;

#[derive(Debug)]
pub struct PriceNormalizer {
    // Exchange-specific configuration for normalization
    exchange_configs: HashMap<String, ExchangeConfig>,
    initialized: bool,
}

#[derive(Debug)]
struct ExchangeConfig {
    // Fee rate (percentage) for the exchange
    #[allow(dead_code)]
    fee_rate: f64,
    // Decimal precision (number of decimal places)
    precision: HashMap<String, usize>,
    // Minimum order size
    #[allow(dead_code)]
    min_order_size: HashMap<String, f64>,
}

impl Default for PriceNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceNormalizer {
    pub fn new() -> Self {
        Self {
            exchange_configs: HashMap::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), OrchestratorError> {
        // Add default configurations for common exchanges
        // These would typically come from a configuration file or database

        let mut binance_precision = HashMap::new();
        binance_precision.insert("BTC-USD".to_string(), 2);
        binance_precision.insert("ETH-USD".to_string(), 2);

        let mut binance_min_order = HashMap::new();
        binance_min_order.insert("BTC-USD".to_string(), 0.001);
        binance_min_order.insert("ETH-USD".to_string(), 0.01);

        self.exchange_configs.insert(
            "binance".to_string(),
            ExchangeConfig {
                fee_rate: 0.1,
                precision: binance_precision,
                min_order_size: binance_min_order,
            },
        );

        let mut coinbase_precision = HashMap::new();
        coinbase_precision.insert("BTC-USD".to_string(), 2);
        coinbase_precision.insert("ETH-USD".to_string(), 2);

        let mut coinbase_min_order = HashMap::new();
        coinbase_min_order.insert("BTC-USD".to_string(), 0.001);
        coinbase_min_order.insert("ETH-USD".to_string(), 0.01);

        self.exchange_configs.insert(
            "coinbase".to_string(),
            ExchangeConfig {
                fee_rate: 0.5,
                precision: coinbase_precision,
                min_order_size: coinbase_min_order,
            },
        );

        self.initialized = true;
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.initialized = false;
        Ok(())
    }

    pub fn normalize(&self, market_data: MarketData) -> Result<NormalizedData, OrchestratorError> {
        if !self.initialized {
            return Err(OrchestratorError::NotInitialized("PriceNormalizer"));
        }

        // Get the exchange configuration
        let exchange_config = self
            .exchange_configs
            .get(&market_data.exchange.to_lowercase())
            .ok_or_else(|| {
                OrchestratorError::InvalidConfiguration(format!(
                    "No configuration for exchange: {}",
                    market_data.exchange
                ))
            })?;

        // Get the precision for this symbol (use default if not found)
        let precision = exchange_config
            .precision
            .get(&market_data.symbol)
            .copied()
            .unwrap_or(2);

        // Normalize the market data based on its type
        let normalized_value = match market_data.data {
            MarketDataValue::OrderBook(ref orderbook) => {
                NormalizedValue::OrderBook(self.normalize_orderbook(orderbook, precision))
            }
            MarketDataValue::Trade(ref trade) => {
                NormalizedValue::Trade(self.normalize_trade(trade, precision))
            }
            MarketDataValue::Ticker(ref ticker) => {
                NormalizedValue::Ticker(self.normalize_ticker(ticker, precision))
            }
            MarketDataValue::Candle(ref candle) => {
                NormalizedValue::Candle(self.normalize_candle(candle, precision))
            }
        };

        Ok(NormalizedData {
            exchange: market_data.exchange,
            symbol: market_data.symbol,
            timestamp: market_data.timestamp,
            data_type: market_data.data_type,
            normalized_value,
        })
    }

    fn normalize_orderbook(&self, orderbook: &OrderBookData, precision: usize) -> OrderBookData {
        let mut normalized = orderbook.clone();

        // Normalize prices in bids
        for bid in &mut normalized.bids {
            bid.price = self.round_to_precision(bid.price, precision);
        }

        // Normalize prices in asks
        for ask in &mut normalized.asks {
            ask.price = self.round_to_precision(ask.price, precision);
        }

        normalized
    }

    fn normalize_trade(&self, trade: &TradeData, precision: usize) -> TradeData {
        let mut normalized = trade.clone();
        normalized.price = self.round_to_precision(normalized.price, precision);
        normalized
    }

    fn normalize_ticker(&self, ticker: &TickerData, precision: usize) -> TickerData {
        let mut normalized = ticker.clone();
        normalized.bid = self.round_to_precision(normalized.bid, precision);
        normalized.ask = self.round_to_precision(normalized.ask, precision);
        normalized.last = self.round_to_precision(normalized.last, precision);
        normalized
    }

    fn normalize_candle(&self, candle: &CandleData, precision: usize) -> CandleData {
        let mut normalized = candle.clone();
        normalized.open = self.round_to_precision(normalized.open, precision);
        normalized.high = self.round_to_precision(normalized.high, precision);
        normalized.low = self.round_to_precision(normalized.low, precision);
        normalized.close = self.round_to_precision(normalized.close, precision);
        normalized
    }

    fn round_to_precision(&self, value: f64, precision: usize) -> f64 {
        let factor = 10.0_f64.powi(precision as i32);
        (value * factor).round() / factor
    }
}
