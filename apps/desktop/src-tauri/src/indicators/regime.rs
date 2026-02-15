use super::types::*;
use super::calculations;
use super::patterns;

// ============================================================================
// Helper: Extract last valid value from LineData series
// ============================================================================

fn last_value(series: &[LineData]) -> Option<f64> {
    series.iter().rev().find_map(|d| d.value)
}

fn last_n_values(series: &[LineData], n: usize) -> Vec<f64> {
    series.iter().rev().filter_map(|d| d.value).take(n).collect::<Vec<_>>().into_iter().rev().collect()
}

// ============================================================================
// Regime Detection
// ============================================================================

pub fn detect_regime(data: &[OHLCData]) -> RegimeAnalysis {
    if data.len() < 50 {
        return RegimeAnalysis {
            regime: RegimeType::Sideways,
            confidence: 0.0,
            trend_strength: 0.0,
            volatility_level: VolatilityLevel::Normal,
            momentum: 0.0,
            supporting_evidence: vec!["Nicht genügend Daten (min. 50 Bars nötig)".to_string()],
        };
    }

    // Calculate indicators
    let sma_20 = calculations::calculate_sma(data, 20);
    let sma_50 = calculations::calculate_sma(data, 50);
    let sma_200 = calculations::calculate_sma(data, 200);
    let adx_result = calculations::calculate_adx(data, 14);
    let rsi_data = calculations::calculate_rsi(data, 14);
    let bollinger = calculations::calculate_bollinger(data, 20, 2.0);
    let atr_data = calculations::calculate_atr(data, 14);

    let close = data.last().map(|d| d.close).unwrap_or(0.0);
    let adx = last_value(&adx_result.adx).unwrap_or(0.0);
    let di_plus = last_value(&adx_result.di_plus).unwrap_or(0.0);
    let di_minus = last_value(&adx_result.di_minus).unwrap_or(0.0);
    let rsi = last_value(&rsi_data).unwrap_or(50.0);
    let sma20_val = last_value(&sma_20).unwrap_or(close);
    let sma50_val = last_value(&sma_50).unwrap_or(close);
    let sma200_val = last_value(&sma_200);
    let bb_upper = last_value(&bollinger.upper).unwrap_or(close);
    let bb_lower = last_value(&bollinger.lower).unwrap_or(close);
    let bb_middle = last_value(&bollinger.middle).unwrap_or(close);
    let _atr = last_value(&atr_data).unwrap_or(0.0);

    let mut evidence: Vec<String> = Vec::new();

    // --- Direction Score (-100 to +100) ---
    let mut direction: f64 = 0.0;

    // Close vs SMA-20: ±15
    if close > sma20_val {
        direction += 15.0;
        evidence.push(format!("Kurs ({:.2}) über SMA-20 ({:.2})", close, sma20_val));
    } else {
        direction -= 15.0;
        evidence.push(format!("Kurs ({:.2}) unter SMA-20 ({:.2})", close, sma20_val));
    }

    // Close vs SMA-50: ±20
    if close > sma50_val {
        direction += 20.0;
        evidence.push(format!("Kurs über SMA-50 ({:.2})", sma50_val));
    } else {
        direction -= 20.0;
        evidence.push(format!("Kurs unter SMA-50 ({:.2})", sma50_val));
    }

    // Close vs SMA-200: ±25
    if let Some(sma200) = sma200_val {
        if close > sma200 {
            direction += 25.0;
            evidence.push(format!("Kurs über SMA-200 ({:.2})", sma200));
        } else {
            direction -= 25.0;
            evidence.push(format!("Kurs unter SMA-200 ({:.2})", sma200));
        }
    }

    // SMA-Alignment (SMA20 > SMA50): ±10
    if sma20_val > sma50_val {
        direction += 10.0;
    } else {
        direction -= 10.0;
    }

    // SMA-20 slope (5-bar): ±10
    let sma20_recent = last_n_values(&sma_20, 6);
    if sma20_recent.len() >= 6 {
        let slope = sma20_recent[5] - sma20_recent[0];
        if slope > 0.0 {
            direction += 10.0;
            evidence.push("SMA-20 steigend".to_string());
        } else {
            direction -= 10.0;
            evidence.push("SMA-20 fallend".to_string());
        }
    }

    // DI+/DI- direction: ±10
    if di_plus > di_minus {
        direction += 10.0;
    } else {
        direction -= 10.0;
    }

    // RSI contribution: (RSI - 50) × 0.2
    direction += (rsi - 50.0) * 0.2;

    // Clamp direction
    direction = direction.clamp(-100.0, 100.0);

    // Momentum = RSI centered
    let momentum = (rsi - 50.0).clamp(-50.0, 50.0);

    // --- Squeeze Detection ---
    let bb_width = if bb_middle > 0.0 {
        ((bb_upper - bb_lower) / bb_middle) * 100.0
    } else {
        10.0
    };
    let is_squeeze = bb_width < 4.0 && adx < 20.0;

    if is_squeeze {
        evidence.push(format!("Bollinger-Squeeze erkannt (Breite: {:.1}%, ADX: {:.1})", bb_width, adx));
    }

    // --- Volatility Level ---
    let volatility_level = if bb_width < 3.0 {
        VolatilityLevel::Low
    } else if bb_width < 8.0 {
        VolatilityLevel::Normal
    } else if bb_width < 15.0 {
        VolatilityLevel::High
    } else {
        VolatilityLevel::Extreme
    };

    // --- Regime Classification ---
    let regime = if is_squeeze {
        RegimeType::Squeeze
    } else if adx >= 40.0 && direction > 30.0 {
        RegimeType::StrongUptrend
    } else if adx >= 25.0 && direction > 15.0 {
        RegimeType::Uptrend
    } else if adx >= 40.0 && direction < -30.0 {
        RegimeType::StrongDowntrend
    } else if adx >= 25.0 && direction < -15.0 {
        RegimeType::Downtrend
    } else {
        RegimeType::Sideways
    };

    evidence.push(format!("ADX: {:.1}, Direction Score: {:.1}", adx, direction));

    // --- Confidence ---
    // 50% ADX contribution + 50% direction contribution
    let adx_conf = (adx / 60.0).min(1.0); // ADX 60+ = full confidence
    let dir_conf = (direction.abs() / 80.0).min(1.0);
    let confidence = ((adx_conf * 0.5 + dir_conf * 0.5) as f64).clamp(0.1, 0.98);

    RegimeAnalysis {
        regime,
        confidence,
        trend_strength: adx,
        volatility_level,
        momentum,
        supporting_evidence: evidence,
    }
}

// ============================================================================
// Setup Scoring
// ============================================================================

pub fn score_setup(data: &[OHLCData], regime: &RegimeAnalysis) -> SetupScore {
    if data.len() < 20 {
        return SetupScore {
            total_score: 0.0,
            regime_score: 0.0,
            momentum_score: 0.0,
            pattern_score: 0.0,
            volume_score: 0.0,
            risk_score: 0.0,
            factors: vec![],
            setup_type: SetupType::NoSetup,
            setup_label: "Nicht genügend Daten".to_string(),
            direction: TradeDirection::Neutral,
        };
    }

    let rsi_data = calculations::calculate_rsi(data, 14);
    let macd_result = calculations::calculate_macd(data, 12, 26, 9);
    let rsi = last_value(&rsi_data).unwrap_or(50.0);

    // --- Factor 1: Regime (30%) ---
    let regime_raw = match regime.regime {
        RegimeType::StrongUptrend | RegimeType::StrongDowntrend => 90.0,
        RegimeType::Uptrend | RegimeType::Downtrend => 70.0,
        RegimeType::Squeeze => 60.0,
        RegimeType::Sideways => 30.0,
    };
    let regime_score = regime_raw * regime.confidence;
    let regime_factor = SetupFactorDetail {
        name: "Regime".to_string(),
        score: regime_score,
        weight: 0.30,
        description: format!("{:?} (Konfidenz: {:.0}%)", regime.regime, regime.confidence * 100.0),
    };

    // --- Factor 2: Momentum (25%) ---
    let momentum_score = calculate_momentum_score(rsi, &macd_result, regime);
    let momentum_factor = SetupFactorDetail {
        name: "Momentum".to_string(),
        score: momentum_score,
        weight: 0.25,
        description: format!("RSI: {:.1}, MACD-Trend", rsi),
    };

    // --- Factor 3: Pattern (15%) ---
    let pattern_matches = patterns::detect_candlestick_patterns(data);
    let pattern_score = calculate_pattern_score(&pattern_matches, regime);
    let pattern_desc = if pattern_matches.is_empty() {
        "Keine Muster erkannt".to_string()
    } else {
        let names: Vec<&str> = pattern_matches.iter().take(3).map(|p| p.name.as_str()).collect();
        names.join(", ")
    };
    let pattern_factor = SetupFactorDetail {
        name: "Muster".to_string(),
        score: pattern_score,
        weight: 0.15,
        description: pattern_desc,
    };

    // --- Factor 4: Volume (15%) ---
    let volume_score = calculate_volume_score(data);
    let volume_factor = SetupFactorDetail {
        name: "Volumen".to_string(),
        score: volume_score,
        weight: 0.15,
        description: "Relatives Volumen zum 20-Tage-Schnitt".to_string(),
    };

    // --- Factor 5: Risk (15%) ---
    let risk_score = match regime.volatility_level {
        VolatilityLevel::Normal => 80.0,
        VolatilityLevel::Low => 70.0,
        VolatilityLevel::High => 50.0,
        VolatilityLevel::Extreme => 30.0,
    };
    let risk_factor = SetupFactorDetail {
        name: "Risiko".to_string(),
        score: risk_score,
        weight: 0.15,
        description: format!("Volatilität: {:?}", regime.volatility_level),
    };

    // --- Total Score ---
    let total_score = (regime_score * 0.30
        + momentum_score * 0.25
        + pattern_score * 0.15
        + volume_score * 0.15
        + risk_score * 0.15)
        .clamp(0.0, 100.0);

    // --- Setup Type Classification ---
    let (setup_type, setup_label) = classify_setup(regime, rsi, &pattern_matches);

    // --- Direction ---
    let direction = match regime.regime {
        RegimeType::StrongUptrend | RegimeType::Uptrend => TradeDirection::Long,
        RegimeType::StrongDowntrend | RegimeType::Downtrend => TradeDirection::Short,
        RegimeType::Squeeze => {
            if regime.momentum > 0.0 { TradeDirection::Long } else { TradeDirection::Short }
        }
        RegimeType::Sideways => TradeDirection::Neutral,
    };

    SetupScore {
        total_score,
        regime_score,
        momentum_score,
        pattern_score,
        volume_score,
        risk_score,
        factors: vec![regime_factor, momentum_factor, pattern_factor, volume_factor, risk_factor],
        setup_type,
        setup_label,
        direction,
    }
}

fn calculate_momentum_score(rsi: f64, macd: &MACDResult, regime: &RegimeAnalysis) -> f64 {
    let mut score: f64 = 50.0;

    // RSI zone alignment with regime
    match regime.regime {
        RegimeType::StrongUptrend | RegimeType::Uptrend => {
            // In uptrend: RSI 40-60 is pullback zone (good entry), >70 overbought
            if (40.0..=60.0).contains(&rsi) {
                score += 30.0; // Pullback opportunity
            } else if (60.0..=70.0).contains(&rsi) {
                score += 15.0; // Still trending
            } else if rsi > 70.0 {
                score -= 10.0; // Overbought
            } else if rsi < 30.0 {
                score -= 20.0; // Likely breakdown
            }
        }
        RegimeType::StrongDowntrend | RegimeType::Downtrend => {
            if (40.0..=60.0).contains(&rsi) {
                score += 30.0; // Bounce zone (good short entry)
            } else if rsi < 30.0 {
                score -= 10.0; // Oversold, less room
            } else if rsi > 70.0 {
                score -= 20.0;
            }
        }
        _ => {
            // Sideways/Squeeze: extremes are good for mean reversion
            if rsi < 30.0 || rsi > 70.0 {
                score += 20.0;
            }
        }
    }

    // MACD histogram direction/trend
    let hist_values: Vec<f64> = macd.histogram.iter().rev()
        .filter_map(|h| h.value)
        .take(5)
        .collect();

    if hist_values.len() >= 2 {
        let latest = hist_values[0];
        let prev = hist_values[1];
        // Histogram increasing in regime direction = bullish
        let hist_improving = match regime.regime {
            RegimeType::StrongUptrend | RegimeType::Uptrend => latest > prev,
            RegimeType::StrongDowntrend | RegimeType::Downtrend => latest < prev,
            _ => latest.abs() > prev.abs(),
        };
        if hist_improving {
            score += 15.0;
        } else {
            score -= 10.0;
        }
    }

    score.clamp(0.0, 100.0)
}

fn calculate_pattern_score(patterns: &[PatternMatch], regime: &RegimeAnalysis) -> f64 {
    if patterns.is_empty() {
        return 40.0; // Neutral baseline
    }

    let mut score: f64 = 40.0;

    // Only consider recent patterns (last 5)
    let recent: Vec<&PatternMatch> = patterns.iter().rev().take(5).collect();

    for pat in &recent {
        let reliability_bonus: f64 = match pat.reliability {
            PatternReliability::High => 15.0,
            PatternReliability::Medium => 10.0,
            PatternReliability::Low => 5.0,
        };

        // Regime alignment bonus
        let aligned = match (&regime.regime, &pat.direction) {
            (RegimeType::StrongUptrend | RegimeType::Uptrend, PatternDirection::Bullish) => true,
            (RegimeType::StrongDowntrend | RegimeType::Downtrend, PatternDirection::Bearish) => true,
            _ => false,
        };

        if aligned {
            score += reliability_bonus * 1.5;
        } else {
            score += reliability_bonus * 0.5;
        }
    }

    score.clamp(0.0, 100.0)
}

fn calculate_volume_score(data: &[OHLCData]) -> f64 {
    let n = data.len();
    if n < 21 {
        return 50.0;
    }

    let current_vol = data[n - 1].volume.unwrap_or(0.0);
    if current_vol <= 0.0 {
        return 50.0; // No volume data
    }

    // 20-day average volume
    let avg_vol: f64 = data[n - 21..n - 1]
        .iter()
        .filter_map(|d| d.volume)
        .sum::<f64>()
        / 20.0;

    if avg_vol <= 0.0 {
        return 50.0;
    }

    let ratio = current_vol / avg_vol;

    if ratio > 2.0 {
        90.0
    } else if ratio > 1.5 {
        75.0
    } else if ratio > 1.0 {
        60.0
    } else if ratio > 0.5 {
        40.0
    } else {
        25.0
    }
}

fn classify_setup(regime: &RegimeAnalysis, rsi: f64, patterns: &[PatternMatch]) -> (SetupType, String) {
    let has_bullish_pattern = patterns.iter().any(|p| p.direction == PatternDirection::Bullish);

    match regime.regime {
        RegimeType::Squeeze => (SetupType::Squeeze, "Squeeze-Auflösung".to_string()),
        RegimeType::StrongUptrend | RegimeType::Uptrend => {
            if rsi < 45.0 {
                (SetupType::TrendFollowing, "Trend-Pullback".to_string())
            } else {
                (SetupType::TrendFollowing, "Trend-Fortsetzung".to_string())
            }
        }
        RegimeType::StrongDowntrend | RegimeType::Downtrend => {
            if has_bullish_pattern && rsi < 35.0 {
                (SetupType::Reversal, "Umkehr".to_string())
            } else if rsi > 55.0 {
                (SetupType::TrendFollowing, "Trend-Pullback (Short)".to_string())
            } else {
                (SetupType::TrendFollowing, "Trend-Fortsetzung (Short)".to_string())
            }
        }
        RegimeType::Sideways => {
            if rsi < 30.0 || rsi > 70.0 {
                (SetupType::MeanReversion, "Mean-Reversion".to_string())
            } else {
                (SetupType::NoSetup, "Kein klares Setup".to_string())
            }
        }
    }
}

// ============================================================================
// Risk & Position Sizing
// ============================================================================

pub fn calculate_risk(
    data: &[OHLCData],
    account_size: f64,
    risk_percent: Option<f64>,
    entry_price: Option<f64>,
    atr_multiplier: Option<f64>,
) -> RiskAnalysis {
    let risk_pct = risk_percent.unwrap_or(1.0);
    let multiplier = atr_multiplier.unwrap_or(2.0);

    let atr_data = calculations::calculate_atr(data, 14);
    let atr = last_value(&atr_data).unwrap_or(0.0);

    let entry = entry_price.unwrap_or_else(|| data.last().map(|d| d.close).unwrap_or(0.0));

    let stop_distance = atr * multiplier;
    let stop_price = entry - stop_distance; // Long bias
    let stop_percent = if entry > 0.0 { (stop_distance / entry) * 100.0 } else { 0.0 };

    let risk_budget = account_size * (risk_pct / 100.0);
    let position_size = if stop_distance > 0.0 {
        (risk_budget / stop_distance).floor()
    } else {
        0.0
    };
    let position_value = position_size * entry;

    let target_price = entry + 2.0 * stop_distance; // 1:2 R:R default
    let risk_reward = 2.0;

    let portfolio_impact = if account_size > 0.0 {
        (position_value / account_size) * 100.0
    } else {
        0.0
    };

    let max_loss_percent = if account_size > 0.0 {
        (risk_budget / account_size) * 100.0
    } else {
        0.0
    };

    RiskAnalysis {
        atr_value: atr,
        atr_stop_price: stop_price,
        atr_stop_percent: stop_percent,
        suggested_position_size: position_size,
        suggested_position_value: position_value,
        risk_per_trade: risk_budget,
        risk_reward_ratio: risk_reward,
        portfolio_impact_percent: portfolio_impact,
        max_loss_percent: max_loss_percent,
        entry_price: entry,
        stop_price,
        target_price,
    }
}

// ============================================================================
// Full Trading Analysis (Convenience)
// ============================================================================

pub fn full_trading_analysis(
    data: &[OHLCData],
    account_size: Option<f64>,
    risk_percent: Option<f64>,
    entry_price: Option<f64>,
) -> TradingAnalysis {
    let regime = detect_regime(data);
    let setup = score_setup(data, &regime);

    let risk = account_size.map(|size| {
        calculate_risk(data, size, risk_percent, entry_price, None)
    });

    TradingAnalysis {
        regime,
        setup,
        risk,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uptrend_data(n: usize) -> Vec<OHLCData> {
        (0..n)
            .map(|i| {
                let base = 100.0 + (i as f64) * 0.5;
                OHLCData {
                    time: format!("2024-01-{:02}", (i % 28) + 1),
                    open: base - 0.2,
                    high: base + 1.0,
                    low: base - 0.5,
                    close: base,
                    volume: Some(1_000_000.0),
                }
            })
            .collect()
    }

    fn make_downtrend_data(n: usize) -> Vec<OHLCData> {
        (0..n)
            .map(|i| {
                let base = 200.0 - (i as f64) * 0.5;
                OHLCData {
                    time: format!("2024-01-{:02}", (i % 28) + 1),
                    open: base + 0.2,
                    high: base + 0.5,
                    low: base - 1.0,
                    close: base,
                    volume: Some(1_000_000.0),
                }
            })
            .collect()
    }

    fn make_sideways_data(n: usize) -> Vec<OHLCData> {
        (0..n)
            .map(|i| {
                let base = 100.0 + if i % 2 == 0 { 0.5 } else { -0.5 };
                OHLCData {
                    time: format!("2024-01-{:02}", (i % 28) + 1),
                    open: base - 0.1,
                    high: base + 0.3,
                    low: base - 0.3,
                    close: base,
                    volume: Some(500_000.0),
                }
            })
            .collect()
    }

    #[test]
    fn test_detect_regime_uptrend() {
        let data = make_uptrend_data(100);
        let regime = detect_regime(&data);
        assert!(
            regime.regime == RegimeType::Uptrend || regime.regime == RegimeType::StrongUptrend,
            "Expected uptrend, got {:?}",
            regime.regime
        );
        assert!(regime.confidence > 0.0);
        assert!(regime.trend_strength >= 0.0);
    }

    #[test]
    fn test_detect_regime_insufficient_data() {
        let data = make_uptrend_data(30);
        let regime = detect_regime(&data);
        assert_eq!(regime.confidence, 0.0);
        assert_eq!(regime.regime, RegimeType::Sideways);
    }

    #[test]
    fn test_setup_score_range() {
        let data = make_uptrend_data(100);
        let regime = detect_regime(&data);
        let setup = score_setup(&data, &regime);
        assert!(setup.total_score >= 0.0 && setup.total_score <= 100.0,
            "Score {} out of range", setup.total_score);
        assert_eq!(setup.factors.len(), 5);
    }

    #[test]
    fn test_risk_calculation() {
        let data = make_uptrend_data(100);
        let risk = calculate_risk(&data, 100_000.0, Some(1.0), None, None);
        assert!(risk.suggested_position_size > 0.0);
        assert!(risk.stop_price < risk.entry_price);
        assert!(risk.target_price > risk.entry_price);
        assert!(risk.atr_value > 0.0);
        assert!(risk.risk_reward_ratio > 0.0);
    }

    #[test]
    fn test_full_trading_analysis() {
        let data = make_uptrend_data(100);
        let analysis = full_trading_analysis(&data, Some(50_000.0), Some(2.0), None);
        assert!(analysis.regime.confidence > 0.0);
        assert!(analysis.setup.total_score >= 0.0);
        assert!(analysis.risk.is_some());
        let risk = analysis.risk.unwrap();
        assert!(risk.entry_price > 0.0);
    }

    #[test]
    fn test_full_trading_analysis_no_account() {
        let data = make_sideways_data(100);
        let analysis = full_trading_analysis(&data, None, None, None);
        assert!(analysis.risk.is_none());
    }

    #[test]
    fn test_downtrend_regime() {
        let data = make_downtrend_data(100);
        let regime = detect_regime(&data);
        // Downtrend data should produce Downtrend, StrongDowntrend, or at minimum not Uptrend
        assert!(
            regime.regime != RegimeType::Uptrend && regime.regime != RegimeType::StrongUptrend,
            "Expected non-uptrend for downtrend data, got {:?}",
            regime.regime
        );
    }
}
