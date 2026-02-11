use super::types::*;
use super::calculations::*;

// ============================================================================
// Main Signal Detection
// ============================================================================

pub fn detect_signals(data: &[OHLCData], config: &SignalDetectionConfig) -> Vec<TechnicalSignal> {
    let mut signals = Vec::new();
    let n = data.len();
    if n < 30 { return signals; }

    let rsi = calculate_rsi(data, 14);
    let macd = calculate_macd(data, 12, 26, 9);
    let bollinger = calculate_bollinger(data, 20, 2.0);
    let stochastic = calculate_stochastic(data, 14, 3, 3);
    let adx = calculate_adx(data, 14);

    let lookback = 5.min(n - 30);

    for i in (n - lookback)..n {
        let bar = &data[i];

        // RSI Signals
        let rsi_val = rsi[i].value;
        let prev_rsi_val = rsi[i - 1].value;

        if let (Some(rv), Some(prv)) = (rsi_val, prev_rsi_val) {
            if rv <= config.rsi_oversold && prv > config.rsi_oversold {
                signals.push(TechnicalSignal {
                    signal_type: SignalType::RsiOversold,
                    direction: SignalDirection::Bullish,
                    strength: if rv < 20.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                    date: bar.time.clone(), price: bar.close,
                    indicator: "RSI".to_string(), value: Some(rv),
                    description: format!("RSI ist überverkauft ({:.1})", rv),
                });
            }
            if rv >= config.rsi_overbought && prv < config.rsi_overbought {
                signals.push(TechnicalSignal {
                    signal_type: SignalType::RsiOverbought,
                    direction: SignalDirection::Bearish,
                    strength: if rv > 80.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                    date: bar.time.clone(), price: bar.close,
                    indicator: "RSI".to_string(), value: Some(rv),
                    description: format!("RSI ist überkauft ({:.1})", rv),
                });
            }
        }

        // MACD Signals
        let macd_val = macd.macd[i].value;
        let signal_val = macd.signal[i].value;
        let prev_macd_val = macd.macd[i - 1].value;
        let prev_signal_val = macd.signal[i - 1].value;

        if let (Some(mv), Some(sv), Some(pmv), Some(psv)) = (macd_val, signal_val, prev_macd_val, prev_signal_val) {
            if mv > sv && pmv <= psv {
                signals.push(TechnicalSignal {
                    signal_type: SignalType::MacdBullishCross,
                    direction: SignalDirection::Bullish,
                    strength: if mv < 0.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                    date: bar.time.clone(), price: bar.close,
                    indicator: "MACD".to_string(), value: None,
                    description: "MACD kreuzt Signal-Linie von unten (Kaufsignal)".to_string(),
                });
            }
            if mv < sv && pmv >= psv {
                signals.push(TechnicalSignal {
                    signal_type: SignalType::MacdBearishCross,
                    direction: SignalDirection::Bearish,
                    strength: if mv > 0.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                    date: bar.time.clone(), price: bar.close,
                    indicator: "MACD".to_string(), value: None,
                    description: "MACD kreuzt Signal-Linie von oben (Verkaufssignal)".to_string(),
                });
            }
        }

        // Bollinger Band Signals
        let upper = bollinger.upper[i].value;
        let lower = bollinger.lower[i].value;
        let middle = bollinger.middle[i].value;

        if let (Some(ub), Some(lb), Some(mb)) = (upper, lower, middle) {
            let bandwidth = (ub - lb) / mb;

            // Squeeze detection
            let mut is_lowest = true;
            let squeeze_start = i.saturating_sub(config.bollinger_squeeze_period);
            for j in squeeze_start..i {
                if let (Some(pu), Some(pl), Some(pm)) = (bollinger.upper[j].value, bollinger.lower[j].value, bollinger.middle[j].value) {
                    if (pu - pl) / pm < bandwidth {
                        is_lowest = false;
                        break;
                    }
                }
            }

            if is_lowest && bandwidth < 0.1 {
                signals.push(TechnicalSignal {
                    signal_type: SignalType::BollingerSqueeze,
                    direction: SignalDirection::Neutral,
                    strength: if bandwidth < 0.05 { SignalStrength::Strong } else { SignalStrength::Moderate },
                    date: bar.time.clone(), price: bar.close,
                    indicator: "Bollinger".to_string(), value: None,
                    description: "Bollinger Squeeze - Volatilitätsausbruch erwartet".to_string(),
                });
            }

            // Breakouts
            if let Some(prev_upper) = bollinger.upper[i - 1].value {
                if bar.close > ub && data[i - 1].close <= prev_upper {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::BollingerBreakoutUp,
                        direction: SignalDirection::Bullish,
                        strength: SignalStrength::Moderate,
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Bollinger".to_string(), value: None,
                        description: "Kurs bricht über oberes Bollinger Band aus".to_string(),
                    });
                }
            }
            if let Some(prev_lower) = bollinger.lower[i - 1].value {
                if bar.close < lb && data[i - 1].close >= prev_lower {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::BollingerBreakoutDown,
                        direction: SignalDirection::Bearish,
                        strength: SignalStrength::Moderate,
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Bollinger".to_string(), value: None,
                        description: "Kurs bricht unter unteres Bollinger Band aus".to_string(),
                    });
                }
            }
        }

        // Stochastic Signals
        let k_val = stochastic.k[i].value;
        let d_val = stochastic.d[i].value;
        let prev_k = stochastic.k[i - 1].value;
        let prev_d = stochastic.d[i - 1].value;

        if let (Some(kv), Some(_dv)) = (k_val, d_val) {
            if let Some(pkv) = prev_k {
                if kv <= 20.0 && pkv > 20.0 {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::StochasticOversold,
                        direction: SignalDirection::Bullish,
                        strength: if kv < 10.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Stochastic".to_string(), value: Some(kv),
                        description: format!("Stochastic überverkauft (%K: {:.1})", kv),
                    });
                }
                if kv >= 80.0 && pkv < 80.0 {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::StochasticOverbought,
                        direction: SignalDirection::Bearish,
                        strength: if kv > 90.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Stochastic".to_string(), value: Some(kv),
                        description: format!("Stochastic überkauft (%K: {:.1})", kv),
                    });
                }
            }

            if let (Some(dv), Some(pkv), Some(pdv)) = (d_val, prev_k, prev_d) {
                if kv > dv && pkv <= pdv && kv < 50.0 {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::StochasticBullishCross,
                        direction: SignalDirection::Bullish,
                        strength: if kv < 20.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Stochastic".to_string(), value: None,
                        description: "%K kreuzt %D von unten (Kaufsignal)".to_string(),
                    });
                }
                if kv < dv && pkv >= pdv && kv > 50.0 {
                    signals.push(TechnicalSignal {
                        signal_type: SignalType::StochasticBearishCross,
                        direction: SignalDirection::Bearish,
                        strength: if kv > 80.0 { SignalStrength::Strong } else { SignalStrength::Moderate },
                        date: bar.time.clone(), price: bar.close,
                        indicator: "Stochastic".to_string(), value: None,
                        description: "%K kreuzt %D von oben (Verkaufssignal)".to_string(),
                    });
                }
            }
        }

        // ADX Signals
        let adx_val = adx.adx[i].value;
        let prev_adx_val = adx.adx[i - 1].value;
        let di_plus = adx.di_plus[i].value;
        let di_minus = adx.di_minus[i].value;

        if let (Some(av), Some(pav)) = (adx_val, prev_adx_val) {
            if av >= config.adx_trend_threshold && pav < config.adx_trend_threshold {
                let dir = match (di_plus, di_minus) {
                    (Some(dp), Some(dm)) => if dp > dm { SignalDirection::Bullish } else { SignalDirection::Bearish },
                    _ => SignalDirection::Neutral,
                };
                let dir_text = match &dir {
                    SignalDirection::Bullish => "Aufwärts",
                    SignalDirection::Bearish => "Abwärts",
                    SignalDirection::Neutral => "Unbestimmt",
                };
                signals.push(TechnicalSignal {
                    signal_type: SignalType::AdxTrendStart,
                    direction: dir, strength: SignalStrength::Moderate,
                    date: bar.time.clone(), price: bar.close,
                    indicator: "ADX".to_string(), value: Some(av),
                    description: format!("Trend beginnt (ADX: {:.1}, {})", av, dir_text),
                });
            }
            if av >= config.adx_strong_threshold && pav < config.adx_strong_threshold {
                let dir = match (di_plus, di_minus) {
                    (Some(dp), Some(dm)) => if dp > dm { SignalDirection::Bullish } else { SignalDirection::Bearish },
                    _ => SignalDirection::Neutral,
                };
                signals.push(TechnicalSignal {
                    signal_type: SignalType::AdxTrendStrong,
                    direction: dir, strength: SignalStrength::Strong,
                    date: bar.time.clone(), price: bar.close,
                    indicator: "ADX".to_string(), value: Some(av),
                    description: format!("Starker Trend (ADX: {:.1})", av),
                });
            }
        }
    }

    signals.sort_by(|a, b| b.date.cmp(&a.date));
    signals
}

// ============================================================================
// Divergence Detection
// ============================================================================

struct PivotPoint {
    index: usize,
    price: f64,
    indicator_value: f64,
    pivot_type: PivotType,
}

enum PivotType { High, Low }

fn find_pivot_points(prices: &[f64], indicator_values: &[Option<f64>], lookback: usize) -> Vec<PivotPoint> {
    let mut pivots = Vec::new();
    let n = prices.len();

    for i in lookback..(n.saturating_sub(lookback)) {
        let iv = match indicator_values[i] {
            Some(v) => v,
            None => continue,
        };

        let mut is_high = true;
        let mut is_low = true;

        for j in 1..=lookback {
            let left = match indicator_values.get(i.wrapping_sub(j)).and_then(|v| *v) {
                Some(v) => v,
                None => { is_high = false; is_low = false; break; }
            };
            let right = match indicator_values.get(i + j).and_then(|v| *v) {
                Some(v) => v,
                None => { is_high = false; is_low = false; break; }
            };

            if iv <= left || iv <= right { is_high = false; }
            if iv >= left || iv >= right { is_low = false; }
        }

        if is_high {
            pivots.push(PivotPoint { index: i, price: prices[i], indicator_value: iv, pivot_type: PivotType::High });
        }
        if is_low {
            pivots.push(PivotPoint { index: i, price: prices[i], indicator_value: iv, pivot_type: PivotType::Low });
        }
    }
    pivots
}

pub fn detect_divergence(data: &[OHLCData], indicator_data: &[LineData], indicator_name: &str, lookback: usize) -> Vec<DivergenceSignal> {
    let mut divergences = Vec::new();
    let n = data.len();
    if n < lookback * 2 { return divergences; }

    let prices: Vec<f64> = data.iter().map(|d| d.close).collect();
    let indicator_values: Vec<Option<f64>> = indicator_data.iter().map(|d| d.value).collect();

    let start = n.saturating_sub(lookback * 3);
    let slice_prices = &prices[start..];
    let slice_indicator = &indicator_values[start..];

    let pivots: Vec<PivotPoint> = find_pivot_points(slice_prices, slice_indicator, 3)
        .into_iter()
        .map(|mut p| { p.index += start; p })
        .collect();

    let pivot_highs: Vec<&PivotPoint> = pivots.iter().filter(|p| matches!(p.pivot_type, PivotType::High)).collect();
    let pivot_lows: Vec<&PivotPoint> = pivots.iter().filter(|p| matches!(p.pivot_type, PivotType::Low)).collect();

    // Bearish divergence: higher high in price, lower high in indicator
    for i in 1..pivot_highs.len() {
        let prev = pivot_highs[i - 1];
        let curr = pivot_highs[i];
        if curr.index - prev.index > lookback { continue; }

        if curr.price > prev.price && curr.indicator_value < prev.indicator_value {
            let price_diff = (curr.price - prev.price) / prev.price;
            let ind_diff = (prev.indicator_value - curr.indicator_value) / prev.indicator_value;
            let confidence = (1.0_f64).min((price_diff + ind_diff) * 5.0);
            if confidence > 0.3 {
                divergences.push(DivergenceSignal {
                    div_type: "bearish".to_string(),
                    indicator: indicator_name.to_string(),
                    start_date: data[prev.index].time.clone(),
                    end_date: data[curr.index].time.clone(),
                    price_start: prev.price, price_end: curr.price,
                    indicator_start: prev.indicator_value, indicator_end: curr.indicator_value,
                    confidence,
                });
            }
        }
    }

    // Bullish divergence: lower low in price, higher low in indicator
    for i in 1..pivot_lows.len() {
        let prev = pivot_lows[i - 1];
        let curr = pivot_lows[i];
        if curr.index - prev.index > lookback { continue; }

        if curr.price < prev.price && curr.indicator_value > prev.indicator_value {
            let price_diff = (prev.price - curr.price) / prev.price;
            let ind_diff = (curr.indicator_value - prev.indicator_value) / prev.indicator_value.abs().max(1.0);
            let confidence = (1.0_f64).min((price_diff + ind_diff) * 5.0);
            if confidence > 0.3 {
                divergences.push(DivergenceSignal {
                    div_type: "bullish".to_string(),
                    indicator: indicator_name.to_string(),
                    start_date: data[prev.index].time.clone(),
                    end_date: data[curr.index].time.clone(),
                    price_start: prev.price, price_end: curr.price,
                    indicator_start: prev.indicator_value, indicator_end: curr.indicator_value,
                    confidence,
                });
            }
        }
    }

    divergences
}

pub fn detect_all_divergences(data: &[OHLCData], lookback: usize) -> Vec<DivergenceSignal> {
    let mut all = Vec::new();
    if data.len() < 30 { return all; }

    let rsi = calculate_rsi(data, 14);
    all.extend(detect_divergence(data, &rsi, "rsi", lookback));

    let macd = calculate_macd(data, 12, 26, 9);
    all.extend(detect_divergence(data, &macd.macd, "macd", lookback));

    let obv = calculate_obv(data);
    all.extend(detect_divergence(data, &obv, "obv", lookback));

    let stochastic = calculate_stochastic(data, 14, 3, 3);
    all.extend(detect_divergence(data, &stochastic.k, "stochastic", lookback));

    all.sort_by(|a, b| b.end_date.cmp(&a.end_date));
    all
}

fn divergence_to_signal(div: &DivergenceSignal, price: f64) -> TechnicalSignal {
    let indicator_names: std::collections::HashMap<&str, &str> = [
        ("rsi", "RSI"), ("macd", "MACD"), ("obv", "OBV"), ("stochastic", "Stochastic"),
    ].into_iter().collect();

    let fallback = div.indicator.as_str();
    let name = indicator_names.get(div.indicator.as_str()).unwrap_or(&fallback);

    TechnicalSignal {
        signal_type: if div.div_type == "bullish" { SignalType::DivergenceBullish } else { SignalType::DivergenceBearish },
        direction: if div.div_type == "bullish" { SignalDirection::Bullish } else { SignalDirection::Bearish },
        strength: if div.confidence > 0.7 { SignalStrength::Strong } else if div.confidence > 0.5 { SignalStrength::Moderate } else { SignalStrength::Weak },
        date: div.end_date.clone(),
        price,
        indicator: name.to_string(),
        value: None,
        description: if div.div_type == "bullish" {
            format!("Bullische Divergenz bei {} - Preis fällt, Indikator steigt", name)
        } else {
            format!("Bärische Divergenz bei {} - Preis steigt, Indikator fällt", name)
        },
    }
}

pub fn get_all_signals(data: &[OHLCData], config: &SignalDetectionConfig) -> Vec<TechnicalSignal> {
    let mut signals = detect_signals(data, config);

    let divergences = detect_all_divergences(data, config.divergence_lookback);
    for div in &divergences {
        if let Some(bar) = data.iter().find(|d| d.time == div.end_date) {
            signals.push(divergence_to_signal(div, bar.close));
        }
    }

    signals.sort_by(|a, b| b.date.cmp(&a.date));
    signals
}
