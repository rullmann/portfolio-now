use crate::indicators::types::*;
use crate::indicators::calculations;
use crate::indicators::patterns;
use crate::indicators::signals;
use crate::indicators::screener;

// ============================================================================
// Individual Indicator Commands
// ============================================================================

#[tauri::command]
pub fn calculate_sma(data: Vec<OHLCData>, period: usize) -> Vec<LineData> {
    calculations::calculate_sma(&data, period)
}

#[tauri::command]
pub fn calculate_ema(data: Vec<OHLCData>, period: usize) -> Vec<LineData> {
    calculations::calculate_ema(&data, period)
}

#[tauri::command]
pub fn calculate_rsi(data: Vec<OHLCData>, period: Option<usize>) -> Vec<LineData> {
    calculations::calculate_rsi(&data, period.unwrap_or(14))
}

#[tauri::command]
pub fn calculate_macd(data: Vec<OHLCData>, fast: Option<usize>, slow: Option<usize>, signal: Option<usize>) -> MACDResult {
    calculations::calculate_macd(&data, fast.unwrap_or(12), slow.unwrap_or(26), signal.unwrap_or(9))
}

#[tauri::command]
pub fn calculate_bollinger(data: Vec<OHLCData>, period: Option<usize>, std_dev: Option<f64>) -> BollingerResult {
    calculations::calculate_bollinger(&data, period.unwrap_or(20), std_dev.unwrap_or(2.0))
}

#[tauri::command]
pub fn calculate_atr(data: Vec<OHLCData>, period: Option<usize>) -> Vec<LineData> {
    calculations::calculate_atr(&data, period.unwrap_or(14))
}

#[tauri::command]
pub fn calculate_vwap(data: Vec<OHLCData>) -> Vec<LineData> {
    calculations::calculate_vwap(&data)
}

#[tauri::command]
pub fn calculate_stochastic(data: Vec<OHLCData>, k_period: Option<usize>, k_slow: Option<usize>, d_period: Option<usize>) -> StochasticResult {
    calculations::calculate_stochastic(&data, k_period.unwrap_or(14), k_slow.unwrap_or(3), d_period.unwrap_or(3))
}

#[tauri::command]
pub fn calculate_obv(data: Vec<OHLCData>) -> Vec<LineData> {
    calculations::calculate_obv(&data)
}

#[tauri::command]
pub fn calculate_adx(data: Vec<OHLCData>, period: Option<usize>) -> ADXResult {
    calculations::calculate_adx(&data, period.unwrap_or(14))
}

#[tauri::command]
pub fn calculate_ichimoku(data: Vec<OHLCData>, tenkan: Option<usize>, kijun: Option<usize>, senkou_b: Option<usize>) -> IchimokuResult {
    calculations::calculate_ichimoku(&data, tenkan.unwrap_or(9), kijun.unwrap_or(26), senkou_b.unwrap_or(52))
}

#[tauri::command]
pub fn calculate_pivot_points(data: Vec<OHLCData>, pivot_type: Option<String>) -> PivotPointsResult {
    calculations::calculate_pivot_points(&data, &pivot_type.unwrap_or_else(|| "standard".to_string()))
}

#[tauri::command]
pub fn calculate_fibonacci(data: Vec<OHLCData>, lookback: Option<usize>) -> FibonacciResult {
    calculations::calculate_fibonacci(&data, lookback.unwrap_or(50))
}

#[tauri::command]
pub fn convert_to_heikin_ashi(data: Vec<OHLCData>) -> Vec<OHLCData> {
    calculations::convert_to_heikin_ashi(&data)
}

// ============================================================================
// Batch: Calculate all requested indicators in one call
// ============================================================================

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorRequest {
    pub indicator_type: String,
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

#[tauri::command]
pub fn calculate_all_indicators(data: Vec<OHLCData>, requests: Vec<IndicatorRequest>) -> AllIndicatorsResult {
    let mut result = AllIndicatorsResult {
        sma: None, ema: None, rsi: None, macd: None, bollinger: None,
        atr: None, vwap: None, stochastic: None, obv: None, adx: None,
        ichimoku: None, pivot: None, fibonacci: None, heikin_ashi: None,
    };

    for req in &requests {
        match req.indicator_type.as_str() {
            "sma" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                result.sma = Some(calculations::calculate_sma(&data, period));
            }
            "ema" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                result.ema = Some(calculations::calculate_ema(&data, period));
            }
            "rsi" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                result.rsi = Some(calculations::calculate_rsi(&data, period));
            }
            "macd" => {
                let fast = req.params.get("fast").and_then(|v| v.as_u64()).unwrap_or(12) as usize;
                let slow = req.params.get("slow").and_then(|v| v.as_u64()).unwrap_or(26) as usize;
                let signal = req.params.get("signal").and_then(|v| v.as_u64()).unwrap_or(9) as usize;
                result.macd = Some(calculations::calculate_macd(&data, fast, slow, signal));
            }
            "bollinger" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                let std_dev = req.params.get("stdDev").and_then(|v| v.as_f64()).unwrap_or(2.0);
                result.bollinger = Some(calculations::calculate_bollinger(&data, period, std_dev));
            }
            "atr" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                result.atr = Some(calculations::calculate_atr(&data, period));
            }
            "vwap" => {
                result.vwap = Some(calculations::calculate_vwap(&data));
            }
            "stochastic" => {
                let k = req.params.get("kPeriod").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                let ks = req.params.get("kSlowPeriod").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
                let d = req.params.get("dPeriod").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
                result.stochastic = Some(calculations::calculate_stochastic(&data, k, ks, d));
            }
            "obv" => {
                result.obv = Some(calculations::calculate_obv(&data));
            }
            "adx" => {
                let period = req.params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                result.adx = Some(calculations::calculate_adx(&data, period));
            }
            "ichimoku" => {
                let tenkan = req.params.get("tenkan").and_then(|v| v.as_u64()).unwrap_or(9) as usize;
                let kijun = req.params.get("kijun").and_then(|v| v.as_u64()).unwrap_or(26) as usize;
                let senkou_b = req.params.get("senkouB").and_then(|v| v.as_u64()).unwrap_or(52) as usize;
                result.ichimoku = Some(calculations::calculate_ichimoku(&data, tenkan, kijun, senkou_b));
            }
            "pivot" => {
                let pt = req.params.get("pivotType").and_then(|v| v.as_str()).unwrap_or("standard");
                result.pivot = Some(calculations::calculate_pivot_points(&data, pt));
            }
            "fibonacci" => {
                let lb = req.params.get("lookback").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                result.fibonacci = Some(calculations::calculate_fibonacci(&data, lb));
            }
            "heikinAshi" => {
                result.heikin_ashi = Some(calculations::convert_to_heikin_ashi(&data));
            }
            _ => {}
        }
    }

    result
}

// ============================================================================
// Pattern Detection Command
// ============================================================================

#[tauri::command]
pub fn detect_candlestick_patterns(data: Vec<OHLCData>) -> Vec<PatternMatch> {
    patterns::detect_candlestick_patterns(&data)
}

// ============================================================================
// Signal Detection Commands
// ============================================================================

#[tauri::command]
pub fn detect_signals(data: Vec<OHLCData>, config: Option<SignalDetectionConfig>) -> Vec<TechnicalSignal> {
    let cfg = config.unwrap_or_default();
    signals::detect_signals(&data, &cfg)
}

#[tauri::command]
pub fn get_all_signals(data: Vec<OHLCData>, config: Option<SignalDetectionConfig>) -> Vec<TechnicalSignal> {
    let cfg = config.unwrap_or_default();
    signals::get_all_signals(&data, &cfg)
}

#[tauri::command]
pub fn detect_all_divergences(data: Vec<OHLCData>, lookback: Option<usize>) -> Vec<DivergenceSignal> {
    signals::detect_all_divergences(&data, lookback.unwrap_or(20))
}

// ============================================================================
// Screener Command
// ============================================================================

#[tauri::command]
pub fn run_screener(securities: Vec<ScreenerSecurityData>, filters: Vec<ScreenerFilter>) -> Vec<ScreenerResult> {
    screener::run_screener(&securities, &filters)
}
