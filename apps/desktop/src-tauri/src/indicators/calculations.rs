use super::types::*;

// ============================================================================
// Simple Moving Average (SMA)
// ============================================================================

pub fn calculate_sma(data: &[OHLCData], period: usize) -> Vec<LineData> {
    let mut result = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        if i < period - 1 {
            result.push(LineData { time: data[i].time.clone(), value: None });
        } else {
            let sum: f64 = (0..period).map(|j| data[i - j].close).sum();
            result.push(LineData { time: data[i].time.clone(), value: Some(sum / period as f64) });
        }
    }
    result
}

// ============================================================================
// Exponential Moving Average (EMA)
// ============================================================================

pub fn calculate_ema(data: &[OHLCData], period: usize) -> Vec<LineData> {
    let mut result = Vec::with_capacity(data.len());
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema: Option<f64> = None;

    for i in 0..data.len() {
        if i < period - 1 {
            result.push(LineData { time: data[i].time.clone(), value: None });
        } else if i == period - 1 {
            let sum: f64 = (0..period).map(|j| data[i - j].close).sum();
            let val = sum / period as f64;
            ema = Some(val);
            result.push(LineData { time: data[i].time.clone(), value: Some(val) });
        } else {
            let prev = ema.unwrap();
            let val = (data[i].close - prev) * multiplier + prev;
            ema = Some(val);
            result.push(LineData { time: data[i].time.clone(), value: Some(val) });
        }
    }
    result
}

// Helper: EMA from raw f64 values (for MACD signal line)
fn ema_from_values(values: &[Option<f64>], times: &[String], period: usize) -> Vec<LineData> {
    let mut result = Vec::with_capacity(values.len());
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut signal_ema: Option<f64> = None;
    let mut valid_count = 0usize;

    for i in 0..values.len() {
        let val = values[i];
        if val.is_none() {
            result.push(LineData { time: times[i].clone(), value: None });
            continue;
        }
        let v = val.unwrap();
        valid_count += 1;

        if valid_count < period {
            result.push(LineData { time: times[i].clone(), value: None });
        } else if valid_count == period {
            // First signal is SMA of valid values
            let mut sum = 0.0;
            let mut count = 0usize;
            let mut j = i as isize;
            while j >= 0 && count < period {
                if let Some(vv) = values[j as usize] {
                    sum += vv;
                    count += 1;
                }
                j -= 1;
            }
            let avg = sum / period as f64;
            signal_ema = Some(avg);
            result.push(LineData { time: times[i].clone(), value: Some(avg) });
        } else {
            let prev = signal_ema.unwrap();
            let new_val = (v - prev) * multiplier + prev;
            signal_ema = Some(new_val);
            result.push(LineData { time: times[i].clone(), value: Some(new_val) });
        }
    }
    result
}

// ============================================================================
// Relative Strength Index (RSI) - Wilder's smoothing
// ============================================================================

pub fn calculate_rsi(data: &[OHLCData], period: usize) -> Vec<LineData> {
    let n = data.len();
    if n < period + 1 {
        return data.iter().map(|d| LineData { time: d.time.clone(), value: None }).collect();
    }

    let mut result = Vec::with_capacity(n);

    // Calculate price changes
    let mut changes = Vec::with_capacity(n - 1);
    for i in 1..n {
        changes.push(data[i].close - data[i - 1].close);
    }

    let gains: Vec<f64> = changes.iter().map(|&c| if c > 0.0 { c } else { 0.0 }).collect();
    let losses: Vec<f64> = changes.iter().map(|&c| if c < 0.0 { -c } else { 0.0 }).collect();

    let mut avg_gain: f64 = gains[..period].iter().sum::<f64>() / period as f64;
    let mut avg_loss: f64 = losses[..period].iter().sum::<f64>() / period as f64;

    // Fill initial nulls
    for i in 0..=period {
        result.push(LineData { time: data[i].time.clone(), value: None });
    }

    // First RSI
    let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
    let rsi = 100.0 - (100.0 / (1.0 + rs));
    result[period] = LineData { time: data[period].time.clone(), value: Some(rsi) };

    // Subsequent RSI using Wilder's smoothing
    for i in (period + 1)..n {
        let ci = i - 1;
        avg_gain = (avg_gain * (period as f64 - 1.0) + gains[ci]) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + losses[ci]) / period as f64;

        let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
        let rsi = 100.0 - (100.0 / (1.0 + rs));
        result.push(LineData { time: data[i].time.clone(), value: Some(rsi) });
    }

    result
}

// ============================================================================
// MACD (Moving Average Convergence Divergence)
// ============================================================================

pub fn calculate_macd(data: &[OHLCData], fast_period: usize, slow_period: usize, signal_period: usize) -> MACDResult {
    let fast_ema = calculate_ema(data, fast_period);
    let slow_ema = calculate_ema(data, slow_period);

    let times: Vec<String> = data.iter().map(|d| d.time.clone()).collect();

    // MACD Line = Fast EMA - Slow EMA
    let macd_values: Vec<Option<f64>> = (0..data.len()).map(|i| {
        match (fast_ema[i].value, slow_ema[i].value) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        }
    }).collect();

    let macd_line: Vec<LineData> = macd_values.iter().enumerate().map(|(i, v)| {
        LineData { time: times[i].clone(), value: *v }
    }).collect();

    // Signal Line = EMA of MACD Line
    let signal_line = ema_from_values(&macd_values, &times, signal_period);

    // Histogram = MACD - Signal
    let histogram: Vec<HistogramData> = (0..data.len()).map(|i| {
        let value = match (macd_values[i], signal_line[i].value) {
            (Some(m), Some(s)) => Some(m - s),
            _ => None,
        };
        let color = value.map(|v| {
            if v >= 0.0 { "#26a69a".to_string() } else { "#ef5350".to_string() }
        });
        HistogramData { time: times[i].clone(), value, color }
    }).collect();

    MACDResult { macd: macd_line, signal: signal_line, histogram }
}

// ============================================================================
// Bollinger Bands
// ============================================================================

pub fn calculate_bollinger(data: &[OHLCData], period: usize, std_dev: f64) -> BollingerResult {
    let middle = calculate_sma(data, period);
    let mut upper = Vec::with_capacity(data.len());
    let mut lower = Vec::with_capacity(data.len());

    for i in 0..data.len() {
        if i < period - 1 || middle[i].value.is_none() {
            upper.push(LineData { time: data[i].time.clone(), value: None });
            lower.push(LineData { time: data[i].time.clone(), value: None });
        } else {
            let mid = middle[i].value.unwrap();
            let mut sum = 0.0;
            for j in 0..period {
                let diff = data[i - j].close - mid;
                sum += diff * diff;
            }
            let std = (sum / period as f64).sqrt();
            upper.push(LineData { time: data[i].time.clone(), value: Some(mid + std_dev * std) });
            lower.push(LineData { time: data[i].time.clone(), value: Some(mid - std_dev * std) });
        }
    }

    BollingerResult { upper, middle, lower }
}

// ============================================================================
// Average True Range (ATR) - Wilder's smoothing
// ============================================================================

pub fn calculate_atr(data: &[OHLCData], period: usize) -> Vec<LineData> {
    let n = data.len();
    if n < 2 {
        return data.iter().map(|d| LineData { time: d.time.clone(), value: None }).collect();
    }

    // Calculate True Range
    let mut tr = vec![data[0].high - data[0].low];
    for i in 1..n {
        let tr1 = data[i].high - data[i].low;
        let tr2 = (data[i].high - data[i - 1].close).abs();
        let tr3 = (data[i].low - data[i - 1].close).abs();
        tr.push(tr1.max(tr2).max(tr3));
    }

    let mut result = vec![LineData { time: data[0].time.clone(), value: None }];

    for i in 1..n {
        if i < period {
            result.push(LineData { time: data[i].time.clone(), value: None });
        } else if i == period {
            let sum: f64 = tr[1..=period].iter().sum();
            result.push(LineData { time: data[i].time.clone(), value: Some(sum / period as f64) });
        } else {
            let prev_atr = result[i - 1].value.unwrap();
            let atr = (prev_atr * (period as f64 - 1.0) + tr[i]) / period as f64;
            result.push(LineData { time: data[i].time.clone(), value: Some(atr) });
        }
    }
    result
}

// ============================================================================
// Volume Weighted Average Price (VWAP)
// ============================================================================

pub fn calculate_vwap(data: &[OHLCData]) -> Vec<LineData> {
    let mut cum_tpv = 0.0;
    let mut cum_vol = 0.0;

    data.iter().map(|d| {
        let vol = d.volume.unwrap_or(0.0);
        if vol == 0.0 {
            return LineData { time: d.time.clone(), value: None };
        }
        let tp = (d.high + d.low + d.close) / 3.0;
        cum_tpv += tp * vol;
        cum_vol += vol;
        LineData {
            time: d.time.clone(),
            value: if cum_vol > 0.0 { Some(cum_tpv / cum_vol) } else { None },
        }
    }).collect()
}

// ============================================================================
// Stochastic Oscillator
// ============================================================================

pub fn calculate_stochastic(data: &[OHLCData], k_period: usize, k_slow_period: usize, d_period: usize) -> StochasticResult {
    let n = data.len();
    let mut raw_k: Vec<Option<f64>> = Vec::with_capacity(n);

    for i in 0..n {
        if i < k_period - 1 {
            raw_k.push(None);
        } else {
            let mut lowest = f64::INFINITY;
            let mut highest = f64::NEG_INFINITY;
            for j in 0..k_period {
                lowest = lowest.min(data[i - j].low);
                highest = highest.max(data[i - j].high);
            }
            let range = highest - lowest;
            raw_k.push(Some(if range == 0.0 { 50.0 } else { 100.0 * (data[i].close - lowest) / range }));
        }
    }

    // Smooth %K with SMA
    let mut k = Vec::with_capacity(n);
    for i in 0..n {
        if raw_k[i].is_none() || i < k_period - 1 + k_slow_period - 1 {
            k.push(LineData { time: data[i].time.clone(), value: None });
        } else {
            let mut sum = 0.0;
            let mut count = 0;
            for j in 0..k_slow_period {
                if let Some(v) = raw_k[i - j] {
                    sum += v;
                    count += 1;
                }
            }
            k.push(LineData {
                time: data[i].time.clone(),
                value: if count > 0 { Some(sum / count as f64) } else { None },
            });
        }
    }

    // %D = SMA of %K
    let mut d = Vec::with_capacity(n);
    for i in 0..n {
        if k[i].value.is_none() || i < k_period - 1 + k_slow_period - 1 + d_period - 1 {
            d.push(LineData { time: data[i].time.clone(), value: None });
        } else {
            let mut sum = 0.0;
            let mut count = 0;
            for j in 0..d_period {
                if let Some(v) = k[i - j].value {
                    sum += v;
                    count += 1;
                }
            }
            d.push(LineData {
                time: data[i].time.clone(),
                value: if count > 0 { Some(sum / count as f64) } else { None },
            });
        }
    }

    StochasticResult { k, d }
}

// ============================================================================
// On-Balance Volume (OBV)
// ============================================================================

pub fn calculate_obv(data: &[OHLCData]) -> Vec<LineData> {
    let mut result = Vec::with_capacity(data.len());
    let mut obv = 0.0;

    for i in 0..data.len() {
        let volume = data[i].volume.unwrap_or(0.0);
        if i == 0 {
            obv = volume;
        } else if data[i].close > data[i - 1].close {
            obv += volume;
        } else if data[i].close < data[i - 1].close {
            obv -= volume;
        }
        result.push(LineData { time: data[i].time.clone(), value: Some(obv) });
    }
    result
}

// ============================================================================
// Average Directional Index (ADX) with +DI and -DI
// ============================================================================

pub fn calculate_adx(data: &[OHLCData], period: usize) -> ADXResult {
    let n = data.len();
    if n < period + 1 {
        let empty: Vec<LineData> = data.iter().map(|d| LineData { time: d.time.clone(), value: None }).collect();
        return ADXResult { adx: empty.clone(), di_plus: empty.clone(), di_minus: empty };
    }

    let mut di_plus_res = Vec::with_capacity(n);
    let mut di_minus_res = Vec::with_capacity(n);
    let mut adx_res = Vec::with_capacity(n);

    // Calculate TR, +DM, -DM
    let mut tr = Vec::with_capacity(n - 1);
    let mut plus_dm = Vec::with_capacity(n - 1);
    let mut minus_dm = Vec::with_capacity(n - 1);

    for i in 1..n {
        let h = data[i].high;
        let l = data[i].low;
        let ph = data[i - 1].high;
        let pl = data[i - 1].low;
        let pc = data[i - 1].close;

        tr.push((h - l).max((h - pc).abs()).max((l - pc).abs()));

        let up_move = h - ph;
        let down_move = pl - l;

        plus_dm.push(if up_move > down_move && up_move > 0.0 { up_move } else { 0.0 });
        minus_dm.push(if down_move > up_move && down_move > 0.0 { down_move } else { 0.0 });
    }

    // First null
    di_plus_res.push(LineData { time: data[0].time.clone(), value: None });
    di_minus_res.push(LineData { time: data[0].time.clone(), value: None });
    adx_res.push(LineData { time: data[0].time.clone(), value: None });

    let mut smoothed_tr: f64 = tr[..period].iter().sum();
    let mut smoothed_plus_dm: f64 = plus_dm[..period].iter().sum();
    let mut smoothed_minus_dm: f64 = minus_dm[..period].iter().sum();

    let mut dx_values: Vec<f64> = Vec::new();

    for i in 1..n {
        if i < period {
            di_plus_res.push(LineData { time: data[i].time.clone(), value: None });
            di_minus_res.push(LineData { time: data[i].time.clone(), value: None });
            adx_res.push(LineData { time: data[i].time.clone(), value: None });
        } else if i == period {
            let plus_di = if smoothed_tr == 0.0 { 0.0 } else { 100.0 * (smoothed_plus_dm / smoothed_tr) };
            let minus_di = if smoothed_tr == 0.0 { 0.0 } else { 100.0 * (smoothed_minus_dm / smoothed_tr) };

            di_plus_res.push(LineData { time: data[i].time.clone(), value: Some(plus_di) });
            di_minus_res.push(LineData { time: data[i].time.clone(), value: Some(minus_di) });

            let di_sum = plus_di + minus_di;
            dx_values.push(if di_sum == 0.0 { 0.0 } else { 100.0 * (plus_di - minus_di).abs() / di_sum });

            adx_res.push(LineData { time: data[i].time.clone(), value: None });
        } else {
            // Wilder's smoothing
            smoothed_tr = smoothed_tr - (smoothed_tr / period as f64) + tr[i - 1];
            smoothed_plus_dm = smoothed_plus_dm - (smoothed_plus_dm / period as f64) + plus_dm[i - 1];
            smoothed_minus_dm = smoothed_minus_dm - (smoothed_minus_dm / period as f64) + minus_dm[i - 1];

            let plus_di = if smoothed_tr == 0.0 { 0.0 } else { 100.0 * (smoothed_plus_dm / smoothed_tr) };
            let minus_di = if smoothed_tr == 0.0 { 0.0 } else { 100.0 * (smoothed_minus_dm / smoothed_tr) };

            di_plus_res.push(LineData { time: data[i].time.clone(), value: Some(plus_di) });
            di_minus_res.push(LineData { time: data[i].time.clone(), value: Some(minus_di) });

            let di_sum = plus_di + minus_di;
            dx_values.push(if di_sum == 0.0 { 0.0 } else { 100.0 * (plus_di - minus_di).abs() / di_sum });

            if dx_values.len() >= period {
                if dx_values.len() == period {
                    let adx_val: f64 = dx_values.iter().sum::<f64>() / period as f64;
                    adx_res.push(LineData { time: data[i].time.clone(), value: Some(adx_val) });
                } else {
                    let prev_adx = adx_res.last().unwrap().value.unwrap();
                    let adx_val = (prev_adx * (period as f64 - 1.0) + dx_values.last().unwrap()) / period as f64;
                    adx_res.push(LineData { time: data[i].time.clone(), value: Some(adx_val) });
                }
            } else {
                adx_res.push(LineData { time: data[i].time.clone(), value: None });
            }
        }
    }

    ADXResult { adx: adx_res, di_plus: di_plus_res, di_minus: di_minus_res }
}

// ============================================================================
// Ichimoku Cloud
// ============================================================================

pub fn calculate_ichimoku(data: &[OHLCData], tenkan_period: usize, kijun_period: usize, senkou_b_period: usize) -> IchimokuResult {
    let n = data.len();

    let calc_midpoint = |end_idx: usize, period: usize| -> Option<f64> {
        if end_idx < period - 1 { return None; }
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        for j in 0..period {
            high = high.max(data[end_idx - j].high);
            low = low.min(data[end_idx - j].low);
        }
        Some((high + low) / 2.0)
    };

    let mut tenkan = Vec::with_capacity(n);
    let mut kijun = Vec::with_capacity(n);
    let mut senkou_a = Vec::with_capacity(n);
    let mut senkou_b = Vec::with_capacity(n);
    let mut chikou = Vec::with_capacity(n);

    for i in 0..n {
        tenkan.push(LineData { time: data[i].time.clone(), value: calc_midpoint(i, tenkan_period) });
        kijun.push(LineData { time: data[i].time.clone(), value: calc_midpoint(i, kijun_period) });
        chikou.push(LineData { time: data[i].time.clone(), value: Some(data[i].close) });
    }

    for i in 0..n {
        let tenkan_val = tenkan[i].value;
        let kijun_val = kijun[i].value;

        senkou_a.push(LineData {
            time: data[i].time.clone(),
            value: match (tenkan_val, kijun_val) {
                (Some(t), Some(k)) => Some((t + k) / 2.0),
                _ => None,
            },
        });

        senkou_b.push(LineData {
            time: data[i].time.clone(),
            value: calc_midpoint(i, senkou_b_period),
        });
    }

    IchimokuResult { tenkan, kijun, senkou_a, senkou_b, chikou }
}

// ============================================================================
// Pivot Points
// ============================================================================

pub fn calculate_pivot_points(data: &[OHLCData], pivot_type: &str) -> PivotPointsResult {
    let n = data.len();
    let mut pivot = Vec::with_capacity(n);
    let mut r1 = Vec::with_capacity(n);
    let mut r2 = Vec::with_capacity(n);
    let mut r3 = Vec::with_capacity(n);
    let mut s1 = Vec::with_capacity(n);
    let mut s2 = Vec::with_capacity(n);
    let mut s3 = Vec::with_capacity(n);

    for i in 0..n {
        if i == 0 {
            let null = LineData { time: data[i].time.clone(), value: None };
            pivot.push(null.clone()); r1.push(null.clone()); r2.push(null.clone()); r3.push(null.clone());
            s1.push(null.clone()); s2.push(null.clone()); s3.push(null);
            continue;
        }

        let h = data[i - 1].high;
        let l = data[i - 1].low;
        let c = data[i - 1].close;

        let (p, rv1, rv2, rv3, sv1, sv2, sv3) = match pivot_type {
            "fibonacci" => {
                let p = (h + l + c) / 3.0;
                let range = h - l;
                (p, p + 0.382 * range, p + 0.618 * range, p + range,
                 p - 0.382 * range, p - 0.618 * range, p - range)
            }
            "woodie" => {
                let p = (h + l + 2.0 * c) / 4.0;
                let rv1 = 2.0 * p - l;
                let rv2 = p + (h - l);
                let rv3 = rv1 + (h - l);
                let sv1 = 2.0 * p - h;
                let sv2 = p - (h - l);
                let sv3 = sv1 - (h - l);
                (p, rv1, rv2, rv3, sv1, sv2, sv3)
            }
            _ => {
                // standard
                let p = (h + l + c) / 3.0;
                (p, 2.0 * p - l, p + (h - l), h + 2.0 * (p - l),
                 2.0 * p - h, p - (h - l), l - 2.0 * (h - p))
            }
        };

        let t = &data[i].time;
        pivot.push(LineData { time: t.clone(), value: Some(p) });
        r1.push(LineData { time: t.clone(), value: Some(rv1) });
        r2.push(LineData { time: t.clone(), value: Some(rv2) });
        r3.push(LineData { time: t.clone(), value: Some(rv3) });
        s1.push(LineData { time: t.clone(), value: Some(sv1) });
        s2.push(LineData { time: t.clone(), value: Some(sv2) });
        s3.push(LineData { time: t.clone(), value: Some(sv3) });
    }

    PivotPointsResult { pivot, r1, r2, r3, s1, s2, s3 }
}

// ============================================================================
// Fibonacci Retracements
// ============================================================================

pub fn calculate_fibonacci(data: &[OHLCData], lookback_period: usize) -> FibonacciResult {
    let lookback = lookback_period.min(data.len());
    let start = data.len() - lookback;
    let recent = &data[start..];

    let mut swing_high = SwingPoint { price: f64::NEG_INFINITY, time: String::new() };
    let mut swing_low = SwingPoint { price: f64::INFINITY, time: String::new() };

    for d in recent {
        if d.high > swing_high.price {
            swing_high = SwingPoint { price: d.high, time: d.time.clone() };
        }
        if d.low < swing_low.price {
            swing_low = SwingPoint { price: d.low, time: d.time.clone() };
        }
    }

    let high_idx = recent.iter().position(|d| d.time == swing_high.time).unwrap_or(0);
    let low_idx = recent.iter().position(|d| d.time == swing_low.time).unwrap_or(0);
    let is_uptrend = low_idx < high_idx;
    let range = swing_high.price - swing_low.price;

    let fib_levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
    let levels = fib_levels.iter().map(|&level| {
        let price = if is_uptrend {
            swing_high.price - level * range
        } else {
            swing_low.price + level * range
        };
        FibonacciLevel {
            level: level * 100.0,
            price,
            label: format!("{:.1}%", level * 100.0),
        }
    }).collect();

    FibonacciResult { levels, swing_high, swing_low }
}

// ============================================================================
// Heikin-Ashi Conversion
// ============================================================================

pub fn convert_to_heikin_ashi(data: &[OHLCData]) -> Vec<OHLCData> {
    let mut result = Vec::with_capacity(data.len());

    for i in 0..data.len() {
        let c = &data[i];
        let ha_close = (c.open + c.high + c.low + c.close) / 4.0;

        let ha_open = if i == 0 {
            (c.open + c.close) / 2.0
        } else {
            let prev: &OHLCData = &result[i - 1];
            (prev.open + prev.close) / 2.0
        };

        let ha_high = c.high.max(ha_open).max(ha_close);
        let ha_low = c.low.min(ha_open).min(ha_close);

        result.push(OHLCData {
            time: c.time.clone(),
            open: ha_open,
            high: ha_high,
            low: ha_low,
            close: ha_close,
            volume: c.volume,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(prices: &[(f64, f64, f64, f64)]) -> Vec<OHLCData> {
        prices.iter().enumerate().map(|(i, &(o, h, l, c))| OHLCData {
            time: format!("2024-01-{:02}", i + 1),
            open: o, high: h, low: l, close: c,
            volume: Some(1000.0),
        }).collect()
    }

    #[test]
    fn test_sma_basic() {
        let data = make_data(&[
            (10.0, 12.0, 9.0, 11.0),
            (11.0, 13.0, 10.0, 12.0),
            (12.0, 14.0, 11.0, 13.0),
            (13.0, 15.0, 12.0, 14.0),
            (14.0, 16.0, 13.0, 15.0),
        ]);
        let sma = calculate_sma(&data, 3);
        assert!(sma[0].value.is_none());
        assert!(sma[1].value.is_none());
        assert!((sma[2].value.unwrap() - 12.0).abs() < 0.001);
        assert!((sma[3].value.unwrap() - 13.0).abs() < 0.001);
        assert!((sma[4].value.unwrap() - 14.0).abs() < 0.001);
    }

    #[test]
    fn test_ema_basic() {
        let data = make_data(&[
            (10.0, 12.0, 9.0, 10.0),
            (10.0, 12.0, 9.0, 11.0),
            (10.0, 12.0, 9.0, 12.0),
            (10.0, 12.0, 9.0, 13.0),
        ]);
        let ema = calculate_ema(&data, 3);
        assert!(ema[0].value.is_none());
        assert!(ema[1].value.is_none());
        assert!(ema[2].value.is_some());
    }

    #[test]
    fn test_rsi_bounds() {
        let mut prices = Vec::new();
        // Generate 30 data points
        for i in 0..30 {
            let v = 100.0 + (i as f64) * 0.5;
            prices.push((v, v + 1.0, v - 1.0, v));
        }
        let data = make_data(&prices);
        let rsi = calculate_rsi(&data, 14);
        for pt in &rsi {
            if let Some(v) = pt.value {
                assert!(v >= 0.0 && v <= 100.0, "RSI out of bounds: {}", v);
            }
        }
    }

    #[test]
    fn test_heikin_ashi_length() {
        let data = make_data(&[
            (10.0, 12.0, 9.0, 11.0),
            (11.0, 13.0, 10.0, 12.0),
            (12.0, 14.0, 11.0, 13.0),
        ]);
        let ha = convert_to_heikin_ashi(&data);
        assert_eq!(ha.len(), data.len());
    }
}
