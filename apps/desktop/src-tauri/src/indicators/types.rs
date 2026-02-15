use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCData {
    pub time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineData {
    pub time: String,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    pub time: String,
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MACDResult {
    pub macd: Vec<LineData>,
    pub signal: Vec<LineData>,
    pub histogram: Vec<HistogramData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerResult {
    pub upper: Vec<LineData>,
    pub middle: Vec<LineData>,
    pub lower: Vec<LineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StochasticResult {
    pub k: Vec<LineData>,
    pub d: Vec<LineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ADXResult {
    pub adx: Vec<LineData>,
    pub di_plus: Vec<LineData>,
    pub di_minus: Vec<LineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IchimokuResult {
    pub tenkan: Vec<LineData>,
    pub kijun: Vec<LineData>,
    pub senkou_a: Vec<LineData>,
    pub senkou_b: Vec<LineData>,
    pub chikou: Vec<LineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotPointsResult {
    pub pivot: Vec<LineData>,
    pub r1: Vec<LineData>,
    pub r2: Vec<LineData>,
    pub r3: Vec<LineData>,
    pub s1: Vec<LineData>,
    pub s2: Vec<LineData>,
    pub s3: Vec<LineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FibonacciLevel {
    pub level: f64,
    pub price: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FibonacciResult {
    pub levels: Vec<FibonacciLevel>,
    pub swing_high: SwingPoint,
    pub swing_low: SwingPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwingPoint {
    pub price: f64,
    pub time: String,
}

// Pattern types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CandlestickPattern {
    Doji,
    Hammer,
    InvertedHammer,
    HangingMan,
    ShootingStar,
    SpinningTop,
    MarubozuBullish,
    MarubozuBearish,
    EngulfingBullish,
    EngulfingBearish,
    HaramiBullish,
    HaramiBearish,
    PiercingLine,
    DarkCloudCover,
    TweezerTop,
    TweezerBottom,
    MorningStar,
    EveningStar,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,
    ThreeInsideUp,
    ThreeInsideDown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternDirection {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PatternReliability {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternMatch {
    pub pattern: CandlestickPattern,
    pub name: String,
    pub start_index: usize,
    pub end_index: usize,
    pub direction: PatternDirection,
    pub reliability: PatternReliability,
    pub description: String,
}

// Signal types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    RsiOversold,
    RsiOverbought,
    MacdBullishCross,
    MacdBearishCross,
    BollingerSqueeze,
    BollingerBreakoutUp,
    BollingerBreakoutDown,
    StochasticOversold,
    StochasticOverbought,
    StochasticBullishCross,
    StochasticBearishCross,
    AdxTrendStart,
    AdxTrendStrong,
    GoldenCross,
    DeathCross,
    DivergenceBullish,
    DivergenceBearish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalStrength {
    Strong,
    Moderate,
    Weak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalSignal {
    #[serde(rename = "type")]
    pub signal_type: SignalType,
    pub direction: SignalDirection,
    pub strength: SignalStrength,
    pub date: String,
    pub price: f64,
    pub indicator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivergenceSignal {
    #[serde(rename = "type")]
    pub div_type: String,
    pub indicator: String,
    pub start_date: String,
    pub end_date: String,
    pub price_start: f64,
    pub price_end: f64,
    pub indicator_start: f64,
    pub indicator_end: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalDetectionConfig {
    #[serde(default = "default_rsi_oversold")]
    pub rsi_oversold: f64,
    #[serde(default = "default_rsi_overbought")]
    pub rsi_overbought: f64,
    #[serde(default = "default_adx_trend")]
    pub adx_trend_threshold: f64,
    #[serde(default = "default_adx_strong")]
    pub adx_strong_threshold: f64,
    #[serde(default = "default_bollinger_squeeze")]
    pub bollinger_squeeze_period: usize,
    #[serde(default = "default_divergence_lookback")]
    pub divergence_lookback: usize,
}

fn default_rsi_oversold() -> f64 { 30.0 }
fn default_rsi_overbought() -> f64 { 70.0 }
fn default_adx_trend() -> f64 { 25.0 }
fn default_adx_strong() -> f64 { 40.0 }
fn default_bollinger_squeeze() -> usize { 20 }
fn default_divergence_lookback() -> usize { 20 }

impl Default for SignalDetectionConfig {
    fn default() -> Self {
        Self {
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            adx_trend_threshold: 25.0,
            adx_strong_threshold: 40.0,
            bollinger_squeeze_period: 20,
            divergence_lookback: 20,
        }
    }
}

// Screener types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerFilter {
    pub id: String,
    pub indicator: String,
    pub condition: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value2: Option<f64>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerSecurityData {
    pub security_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub ohlc_data: Vec<OHLCData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerResult {
    pub security_id: i64,
    pub security_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub matched_filters: Vec<String>,
    pub current_values: std::collections::HashMap<String, Option<f64>>,
    pub last_price: f64,
    pub change_1d: Option<f64>,
    pub change_5d: Option<f64>,
    pub change_20d: Option<f64>,
}

// Batch result for calculating all indicators at once
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllIndicatorsResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sma: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsi: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macd: Option<MACDResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bollinger: Option<BollingerResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vwap: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stochastic: Option<StochasticResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obv: Option<Vec<LineData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adx: Option<ADXResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ichimoku: Option<IchimokuResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot: Option<PivotPointsResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fibonacci: Option<FibonacciResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heikin_ashi: Option<Vec<OHLCData>>,
}

// ============================================================================
// Trading Analysis Types (Regime, Setup Scoring, Risk)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegimeType {
    StrongUptrend,
    Uptrend,
    Sideways,
    Downtrend,
    StrongDowntrend,
    Squeeze,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VolatilityLevel {
    Low,
    Normal,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeAnalysis {
    pub regime: RegimeType,
    pub confidence: f64,
    pub trend_strength: f64,
    pub volatility_level: VolatilityLevel,
    pub momentum: f64,
    pub supporting_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SetupType {
    TrendFollowing,
    Breakout,
    Reversal,
    MeanReversion,
    Squeeze,
    NoSetup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TradeDirection {
    Long,
    Short,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupFactorDetail {
    pub name: String,
    pub score: f64,
    pub weight: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupScore {
    pub total_score: f64,
    pub regime_score: f64,
    pub momentum_score: f64,
    pub pattern_score: f64,
    pub volume_score: f64,
    pub risk_score: f64,
    pub factors: Vec<SetupFactorDetail>,
    pub setup_type: SetupType,
    pub setup_label: String,
    pub direction: TradeDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskAnalysis {
    pub atr_value: f64,
    pub atr_stop_price: f64,
    pub atr_stop_percent: f64,
    pub suggested_position_size: f64,
    pub suggested_position_value: f64,
    pub risk_per_trade: f64,
    pub risk_reward_ratio: f64,
    pub portfolio_impact_percent: f64,
    pub max_loss_percent: f64,
    pub entry_price: f64,
    pub stop_price: f64,
    pub target_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingAnalysis {
    pub regime: RegimeAnalysis,
    pub setup: SetupScore,
    pub risk: Option<RiskAnalysis>,
}
