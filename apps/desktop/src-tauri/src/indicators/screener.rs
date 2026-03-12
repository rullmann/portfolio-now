use super::types::*;
use super::calculations::*;
use std::collections::HashMap;

struct CalculatedIndicators {
    price: f64,
    volume: f64,
    volume_avg20: f64,
    rsi: Option<f64>,
    macd: Option<f64>,
    macd_signal: Option<f64>,
    macd_histogram: Option<f64>,
    macd_histogram_prev: Option<f64>,
    bollinger_upper: Option<f64>,
    bollinger_lower: Option<f64>,
    #[allow(dead_code)]
    bollinger_middle: Option<f64>,
    bollinger_width: Option<f64>,
    stochastic_k: Option<f64>,
    stochastic_d: Option<f64>,
    stochastic_k_prev: Option<f64>,
    stochastic_d_prev: Option<f64>,
    adx: Option<f64>,
    di_plus: Option<f64>,
    di_minus: Option<f64>,
    obv: Option<f64>,
    sma20: Option<f64>,
    sma50: Option<f64>,
    sma200: Option<f64>,
    change_1d: Option<f64>,
    change_5d: Option<f64>,
    change_20d: Option<f64>,
}

fn last_value(data: &[LineData]) -> Option<f64> {
    data.last().and_then(|d| d.value)
}

fn prev_value(data: &[LineData], offset: usize) -> Option<f64> {
    if data.len() <= offset { return None; }
    data[data.len() - 1 - offset].value
}

fn sma_simple(data: &[OHLCData], period: usize) -> Option<f64> {
    if data.len() < period { return None; }
    let slice = &data[data.len() - period..];
    let sum: f64 = slice.iter().map(|d| d.close).sum();
    Some(sum / period as f64)
}

fn calc_indicators(ohlc: &[OHLCData]) -> Option<CalculatedIndicators> {
    if ohlc.len() < 2 { return None; }
    let n = ohlc.len();
    let last = &ohlc[n - 1];
    let prev = &ohlc[n - 2];

    let vol_slice = &ohlc[n.saturating_sub(20)..];
    let vol_avg = vol_slice.iter().map(|d| d.volume.unwrap_or(0.0)).sum::<f64>() / vol_slice.len() as f64;

    let price_5d_ago = if n >= 5 { Some(ohlc[n - 5].close) } else { None };
    let price_20d_ago = if n >= 20 { Some(ohlc[n - 20].close) } else { None };

    let rsi_data = calculate_rsi(ohlc, 14);
    let macd_data = calculate_macd(ohlc, 12, 26, 9);
    let boll = calculate_bollinger(ohlc, 20, 2.0);
    let stoch = calculate_stochastic(ohlc, 14, 3, 3);
    let adx_data = calculate_adx(ohlc, 14);
    let obv_data = calculate_obv(ohlc);

    let bu = last_value(&boll.upper);
    let bl = last_value(&boll.lower);
    let bm = last_value(&boll.middle);
    let bw = match (bu, bl, bm) {
        (Some(u), Some(l), Some(m)) if m != 0.0 => Some((u - l) / m * 100.0),
        _ => None,
    };

    Some(CalculatedIndicators {
        price: last.close,
        volume: last.volume.unwrap_or(0.0),
        volume_avg20: vol_avg,
        rsi: last_value(&rsi_data),
        macd: last_value(&macd_data.macd),
        macd_signal: last_value(&macd_data.signal),
        macd_histogram: last_value(&macd_data.histogram.iter().map(|h| LineData { time: h.time.clone(), value: h.value }).collect::<Vec<_>>()),
        macd_histogram_prev: prev_value(&macd_data.histogram.iter().map(|h| LineData { time: h.time.clone(), value: h.value }).collect::<Vec<_>>(), 1),
        bollinger_upper: bu,
        bollinger_lower: bl,
        bollinger_middle: bm,
        bollinger_width: bw,
        stochastic_k: last_value(&stoch.k),
        stochastic_d: last_value(&stoch.d),
        stochastic_k_prev: prev_value(&stoch.k, 1),
        stochastic_d_prev: prev_value(&stoch.d, 1),
        adx: last_value(&adx_data.adx),
        di_plus: last_value(&adx_data.di_plus),
        di_minus: last_value(&adx_data.di_minus),
        obv: last_value(&obv_data),
        sma20: sma_simple(ohlc, 20),
        sma50: sma_simple(ohlc, 50),
        sma200: sma_simple(ohlc, 200),
        change_1d: Some(((last.close - prev.close) / prev.close) * 100.0),
        change_5d: price_5d_ago.map(|p| ((last.close - p) / p) * 100.0),
        change_20d: price_20d_ago.map(|p| ((last.close - p) / p) * 100.0),
    })
}

fn get_indicator_value(indicator: &str, v: &CalculatedIndicators) -> Option<f64> {
    match indicator {
        "price" => Some(v.price),
        "volume" => if v.volume_avg20 > 0.0 { Some((v.volume / v.volume_avg20) * 100.0) } else { None },
        "rsi" => v.rsi,
        "macd" => v.macd,
        "macd_signal" => v.macd_signal,
        "macd_histogram" => v.macd_histogram,
        "bollinger_upper" => match (v.bollinger_upper, v.price) {
            (Some(u), p) if u != 0.0 => Some((p / u) * 100.0),
            _ => None,
        },
        "bollinger_lower" => match (v.bollinger_lower, v.price) {
            (Some(l), p) if l != 0.0 => Some((p / l) * 100.0),
            _ => None,
        },
        "bollinger_width" => v.bollinger_width,
        "stochastic_k" => v.stochastic_k,
        "stochastic_d" => v.stochastic_d,
        "adx" => v.adx,
        "di_plus" => v.di_plus,
        "di_minus" => v.di_minus,
        "obv" => v.obv,
        "sma_20" => v.sma20,
        "sma_50" => v.sma50,
        "sma_200" => v.sma200,
        "change_1d" => v.change_1d,
        "change_5d" => v.change_5d,
        "change_20d" => v.change_20d,
        _ => None,
    }
}

fn check_condition(filter: &ScreenerFilter, v: &CalculatedIndicators) -> bool {
    let value = match get_indicator_value(&filter.indicator, v) {
        Some(val) => val,
        None => return false,
    };

    match filter.condition.as_str() {
        "above" => {
            if filter.indicator == "di_plus" {
                return match (v.di_plus, v.di_minus) {
                    (Some(dp), Some(dm)) => dp > dm,
                    _ => false,
                };
            }
            if filter.indicator == "di_minus" {
                return match (v.di_minus, v.di_plus) {
                    (Some(dm), Some(dp)) => dm > dp,
                    _ => false,
                };
            }
            value > filter.value
        }
        "below" => value < filter.value,
        "crosses_above" => {
            if filter.indicator == "stochastic_k" {
                return v.stochastic_k_prev.map_or(false, |pk| pk <= filter.value && value > filter.value);
            }
            if filter.indicator == "stochastic_d" {
                return v.stochastic_d_prev.map_or(false, |pd| pd <= filter.value && value > filter.value);
            }
            false
        }
        "crosses_below" => {
            if filter.indicator == "stochastic_k" {
                return v.stochastic_k_prev.map_or(false, |pk| pk >= filter.value && value < filter.value);
            }
            if filter.indicator == "stochastic_d" {
                return v.stochastic_d_prev.map_or(false, |pd| pd >= filter.value && value < filter.value);
            }
            false
        }
        "between" => filter.value2.map_or(false, |v2| value >= filter.value && value <= v2),
        "increasing" => {
            if filter.indicator == "macd_histogram" {
                return match (v.macd_histogram, v.macd_histogram_prev) {
                    (Some(cur), Some(prev)) => cur > prev,
                    _ => false,
                };
            }
            false
        }
        "decreasing" => {
            if filter.indicator == "macd_histogram" {
                return match (v.macd_histogram, v.macd_histogram_prev) {
                    (Some(cur), Some(prev)) => cur < prev,
                    _ => false,
                };
            }
            false
        }
        _ => false,
    }
}

// Indicator label map for matched filter descriptions
fn indicator_label(ind: &str) -> &str {
    match ind {
        "price" => "Preis",
        "volume" => "Volumen (%)",
        "rsi" => "RSI (14)",
        "macd" => "MACD",
        "macd_signal" => "MACD Signal",
        "macd_histogram" => "MACD Histogramm",
        "bollinger_upper" => "Bollinger Upper (%)",
        "bollinger_lower" => "Bollinger Lower (%)",
        "bollinger_width" => "Bollinger Breite (%)",
        "stochastic_k" => "Stochastic %K",
        "stochastic_d" => "Stochastic %D",
        "adx" => "ADX (14)",
        "di_plus" => "+DI",
        "di_minus" => "-DI",
        "obv" => "OBV",
        "sma_20" => "SMA 20",
        "sma_50" => "SMA 50",
        "sma_200" => "SMA 200",
        "change_1d" => "Änderung 1T (%)",
        "change_5d" => "Änderung 5T (%)",
        "change_20d" => "Änderung 20T (%)",
        _ => ind,
    }
}

fn condition_label(cond: &str) -> &str {
    match cond {
        "above" => "über",
        "below" => "unter",
        "crosses_above" => "kreuzt über",
        "crosses_below" => "kreuzt unter",
        "between" => "zwischen",
        "increasing" => "steigend",
        "decreasing" => "fallend",
        _ => cond,
    }
}

pub fn run_screener(securities: &[ScreenerSecurityData], filters: &[ScreenerFilter]) -> Vec<ScreenerResult> {
    let active: Vec<&ScreenerFilter> = filters.iter().filter(|f| f.enabled).collect();
    if active.is_empty() { return Vec::new(); }

    let mut results = Vec::new();

    for sec in securities {
        if sec.ohlc_data.len() < 20 { continue; }

        let indicators = match calc_indicators(&sec.ohlc_data) {
            Some(ind) => ind,
            None => continue,
        };

        let mut matched = Vec::new();
        let mut all_match = true;

        for filter in &active {
            if check_condition(filter, &indicators) {
                let desc = match filter.value2 {
                    Some(v2) => format!("{} {} {} und {}", indicator_label(&filter.indicator), condition_label(&filter.condition), filter.value, v2),
                    None => format!("{} {} {}", indicator_label(&filter.indicator), condition_label(&filter.condition), filter.value),
                };
                matched.push(desc);
            } else {
                all_match = false;
                break;
            }
        }

        if all_match && !matched.is_empty() {
            let vol_pct = if indicators.volume_avg20 > 0.0 { Some((indicators.volume / indicators.volume_avg20) * 100.0) } else { None };

            let mut current_values = HashMap::new();
            current_values.insert("price".to_string(), Some(indicators.price));
            current_values.insert("rsi".to_string(), indicators.rsi);
            current_values.insert("macd".to_string(), indicators.macd);
            current_values.insert("adx".to_string(), indicators.adx);
            current_values.insert("volume".to_string(), vol_pct);
            current_values.insert("change1d".to_string(), indicators.change_1d);
            current_values.insert("change5d".to_string(), indicators.change_5d);
            current_values.insert("change20d".to_string(), indicators.change_20d);

            results.push(ScreenerResult {
                security_id: sec.security_id,
                security_name: sec.name.clone(),
                ticker: sec.ticker.clone(),
                isin: sec.isin.clone(),
                currency: sec.currency.clone(),
                matched_filters: matched,
                current_values,
                last_price: indicators.price,
                change_1d: indicators.change_1d,
                change_5d: indicators.change_5d,
                change_20d: indicators.change_20d,
                breakout_score: None,
                regime: None,
                setup_score: None,
            });
        }
    }

    results.sort_by(|a, b| {
        let ac = a.change_1d.unwrap_or(0.0).abs();
        let bc = b.change_1d.unwrap_or(0.0).abs();
        bc.partial_cmp(&ac).unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}
