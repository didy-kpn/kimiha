use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Market data structure received from connectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub exchange: String,
    pub symbol: String,
    pub timestamp: u64,
    pub data_type: MarketDataType,
    pub data: MarketDataValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketDataType {
    OrderBook,
    Trade,
    Ticker,
    Candle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataValue {
    OrderBook(OrderBookData),
    Trade(TradeData),
    Ticker(TickerData),
    Candle(CandleData),
}

// Order book data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub amount: f64,
}

// Trade data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub id: String,
    pub price: f64,
    pub amount: f64,
    pub side: TradeSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

// Ticker data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerData {
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: f64,
}

// Candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub interval: String,
}

// Normalized data after processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedData {
    pub exchange: String,
    pub symbol: String,
    pub timestamp: u64,
    pub data_type: MarketDataType,
    pub normalized_value: NormalizedValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizedValue {
    OrderBook(OrderBookData),
    Trade(TradeData),
    Ticker(TickerData),
    Candle(CandleData),
}

// Trading opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub opportunity_type: OpportunityType,
    pub timestamp: u64,
    pub symbols: Vec<String>,
    pub exchanges: Vec<String>,
    pub potential_profit: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpportunityType {
    Arbitrage,
    Spread,
    Trend,
    Other(String),
}

// Market snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub timestamp: u64,
    pub orderbooks: HashMap<String, HashMap<String, OrderBookData>>, // exchange -> symbol -> orderbook
    pub tickers: HashMap<String, HashMap<String, TickerData>>,       // exchange -> symbol -> ticker
}
