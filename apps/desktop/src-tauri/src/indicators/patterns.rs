use super::types::*;

// ============================================================================
// Helper Functions
// ============================================================================

fn body_size(c: &OHLCData) -> f64 {
    (c.close - c.open).abs()
}

fn candle_range(c: &OHLCData) -> f64 {
    c.high - c.low
}

fn upper_wick(c: &OHLCData) -> f64 {
    c.high - c.open.max(c.close)
}

fn lower_wick(c: &OHLCData) -> f64 {
    c.open.min(c.close) - c.low
}

fn is_bullish(c: &OHLCData) -> bool {
    c.close > c.open
}

fn is_bearish(c: &OHLCData) -> bool {
    c.close < c.open
}

fn average_body(data: &[OHLCData], index: usize, period: usize) -> f64 {
    let start = index.saturating_sub(period);
    let mut sum = 0.0;
    let mut count = 0;
    for i in start..index {
        sum += body_size(&data[i]);
        count += 1;
    }
    if count > 0 { sum / count as f64 } else { body_size(&data[index]) }
}

fn is_in_downtrend(data: &[OHLCData], index: usize, period: usize) -> bool {
    if index < period { return false; }
    let start_price = data[index - period].close;
    let end_price = data[index].close;
    end_price < start_price * 0.98
}

fn is_in_uptrend(data: &[OHLCData], index: usize, period: usize) -> bool {
    if index < period { return false; }
    let start_price = data[index - period].close;
    let end_price = data[index].close;
    end_price > start_price * 1.02
}

// ============================================================================
// Single Candle Patterns
// ============================================================================

fn detect_doji(c: &OHLCData, avg_body: f64) -> bool {
    let body = body_size(c);
    let range = candle_range(c);
    body < avg_body * 0.1 && range > avg_body * 0.5
}

fn detect_hammer(c: &OHLCData, avg_body: f64, downtrend: bool) -> bool {
    let body = body_size(c);
    downtrend && lower_wick(c) >= body * 2.0 && upper_wick(c) <= body * 0.3 && body >= avg_body * 0.3
}

fn detect_inverted_hammer(c: &OHLCData, avg_body: f64, downtrend: bool) -> bool {
    let body = body_size(c);
    downtrend && upper_wick(c) >= body * 2.0 && lower_wick(c) <= body * 0.3 && body >= avg_body * 0.3
}

fn detect_hanging_man(c: &OHLCData, avg_body: f64, uptrend: bool) -> bool {
    let body = body_size(c);
    uptrend && lower_wick(c) >= body * 2.0 && upper_wick(c) <= body * 0.3 && body >= avg_body * 0.3
}

fn detect_shooting_star(c: &OHLCData, avg_body: f64, uptrend: bool) -> bool {
    let body = body_size(c);
    uptrend && upper_wick(c) >= body * 2.0 && lower_wick(c) <= body * 0.3 && body >= avg_body * 0.3
}

fn detect_spinning_top(c: &OHLCData, avg_body: f64) -> bool {
    let body = body_size(c);
    body < avg_body * 0.5 && upper_wick(c) > body && lower_wick(c) > body
}

fn detect_marubozu(c: &OHLCData, avg_body: f64) -> Option<bool> {
    let body = body_size(c);
    let uw = upper_wick(c);
    let lw = lower_wick(c);
    let range = candle_range(c);
    if body > avg_body * 1.5 && uw < range * 0.05 && lw < range * 0.05 {
        Some(is_bullish(c))
    } else {
        None
    }
}

// ============================================================================
// Two Candle Patterns
// ============================================================================

fn detect_engulfing_bullish(prev: &OHLCData, curr: &OHLCData, downtrend: bool) -> bool {
    downtrend && is_bearish(prev) && is_bullish(curr) && curr.open < prev.close && curr.close > prev.open
}

fn detect_engulfing_bearish(prev: &OHLCData, curr: &OHLCData, uptrend: bool) -> bool {
    uptrend && is_bullish(prev) && is_bearish(curr) && curr.open > prev.close && curr.close < prev.open
}

fn detect_harami_bullish(prev: &OHLCData, curr: &OHLCData, downtrend: bool) -> bool {
    downtrend && is_bearish(prev) && is_bullish(curr)
        && curr.open > prev.close && curr.close < prev.open
        && body_size(curr) < body_size(prev) * 0.5
}

fn detect_harami_bearish(prev: &OHLCData, curr: &OHLCData, uptrend: bool) -> bool {
    uptrend && is_bullish(prev) && is_bearish(curr)
        && curr.open < prev.close && curr.close > prev.open
        && body_size(curr) < body_size(prev) * 0.5
}

fn detect_piercing_line(prev: &OHLCData, curr: &OHLCData, downtrend: bool) -> bool {
    let mid = (prev.open + prev.close) / 2.0;
    downtrend && is_bearish(prev) && is_bullish(curr)
        && curr.open < prev.low && curr.close > mid && curr.close < prev.open
}

fn detect_dark_cloud_cover(prev: &OHLCData, curr: &OHLCData, uptrend: bool) -> bool {
    let mid = (prev.open + prev.close) / 2.0;
    uptrend && is_bullish(prev) && is_bearish(curr)
        && curr.open > prev.high && curr.close < mid && curr.close > prev.open
}

fn detect_tweezer_top(prev: &OHLCData, curr: &OHLCData, uptrend: bool) -> bool {
    let tolerance = candle_range(prev) * 0.05;
    uptrend && is_bullish(prev) && is_bearish(curr) && (prev.high - curr.high).abs() < tolerance
}

fn detect_tweezer_bottom(prev: &OHLCData, curr: &OHLCData, downtrend: bool) -> bool {
    let tolerance = candle_range(prev) * 0.05;
    downtrend && is_bearish(prev) && is_bullish(curr) && (prev.low - curr.low).abs() < tolerance
}

// ============================================================================
// Three Candle Patterns
// ============================================================================

fn detect_morning_star(first: &OHLCData, second: &OHLCData, third: &OHLCData, avg_body: f64, downtrend: bool) -> bool {
    downtrend && is_bearish(first) && body_size(first) > avg_body
        && body_size(second) < avg_body * 0.5 && second.close < first.close
        && is_bullish(third) && body_size(third) > avg_body
        && third.close > (first.open + first.close) / 2.0
}

fn detect_evening_star(first: &OHLCData, second: &OHLCData, third: &OHLCData, avg_body: f64, uptrend: bool) -> bool {
    uptrend && is_bullish(first) && body_size(first) > avg_body
        && body_size(second) < avg_body * 0.5 && second.close > first.close
        && is_bearish(third) && body_size(third) > avg_body
        && third.close < (first.open + first.close) / 2.0
}

fn detect_three_white_soldiers(first: &OHLCData, second: &OHLCData, third: &OHLCData, avg_body: f64) -> bool {
    is_bullish(first) && is_bullish(second) && is_bullish(third)
        && second.open > first.open && second.close > first.close
        && third.open > second.open && third.close > second.close
        && body_size(first) > avg_body * 0.7
        && body_size(second) > avg_body * 0.7
        && body_size(third) > avg_body * 0.7
        && upper_wick(first) < body_size(first) * 0.3
        && upper_wick(second) < body_size(second) * 0.3
        && upper_wick(third) < body_size(third) * 0.3
}

fn detect_three_black_crows(first: &OHLCData, second: &OHLCData, third: &OHLCData, avg_body: f64) -> bool {
    is_bearish(first) && is_bearish(second) && is_bearish(third)
        && second.open < first.open && second.close < first.close
        && third.open < second.open && third.close < second.close
        && body_size(first) > avg_body * 0.7
        && body_size(second) > avg_body * 0.7
        && body_size(third) > avg_body * 0.7
        && lower_wick(first) < body_size(first) * 0.3
        && lower_wick(second) < body_size(second) * 0.3
        && lower_wick(third) < body_size(third) * 0.3
}

fn detect_three_inside_up(first: &OHLCData, second: &OHLCData, third: &OHLCData, downtrend: bool) -> bool {
    downtrend && is_bearish(first) && is_bullish(second)
        && second.open > first.close && second.close < first.open
        && body_size(second) < body_size(first) * 0.5
        && is_bullish(third) && third.close > first.open
}

fn detect_three_inside_down(first: &OHLCData, second: &OHLCData, third: &OHLCData, uptrend: bool) -> bool {
    uptrend && is_bullish(first) && is_bearish(second)
        && second.open < first.close && second.close > first.open
        && body_size(second) < body_size(first) * 0.5
        && is_bearish(third) && third.close < first.open
}

// ============================================================================
// Pattern Info
// ============================================================================

fn pattern_info(p: &CandlestickPattern) -> (&'static str, &'static str) {
    match p {
        CandlestickPattern::Doji => ("Doji", "Unentschlossenheit, mögliche Trendwende"),
        CandlestickPattern::Hammer => ("Hammer", "Bullisches Umkehrmuster nach Abwärtstrend"),
        CandlestickPattern::InvertedHammer => ("Umgekehrter Hammer", "Mögliche bullische Umkehr"),
        CandlestickPattern::HangingMan => ("Hanging Man", "Bärisches Warnsignal nach Aufwärtstrend"),
        CandlestickPattern::ShootingStar => ("Shooting Star", "Bärisches Umkehrmuster nach Aufwärtstrend"),
        CandlestickPattern::SpinningTop => ("Spinning Top", "Unentschlossenheit im Markt"),
        CandlestickPattern::MarubozuBullish => ("Bullish Marubozu", "Starke bullische Kerze ohne Dochte"),
        CandlestickPattern::MarubozuBearish => ("Bearish Marubozu", "Starke bärische Kerze ohne Dochte"),
        CandlestickPattern::EngulfingBullish => ("Bullish Engulfing", "Starkes bullisches Umkehrmuster"),
        CandlestickPattern::EngulfingBearish => ("Bearish Engulfing", "Starkes bärisches Umkehrmuster"),
        CandlestickPattern::HaramiBullish => ("Bullish Harami", "Mögliche bullische Umkehr"),
        CandlestickPattern::HaramiBearish => ("Bearish Harami", "Mögliche bärische Umkehr"),
        CandlestickPattern::PiercingLine => ("Piercing Line", "Bullisches Umkehrmuster"),
        CandlestickPattern::DarkCloudCover => ("Dark Cloud Cover", "Bärisches Umkehrmuster"),
        CandlestickPattern::TweezerTop => ("Tweezer Top", "Bärisches Umkehrmuster mit gleichem Hoch"),
        CandlestickPattern::TweezerBottom => ("Tweezer Bottom", "Bullisches Umkehrmuster mit gleichem Tief"),
        CandlestickPattern::MorningStar => ("Morning Star", "Starkes bullisches Umkehrmuster"),
        CandlestickPattern::EveningStar => ("Evening Star", "Starkes bärisches Umkehrmuster"),
        CandlestickPattern::ThreeWhiteSoldiers => ("Three White Soldiers", "Starkes bullisches Fortsetzungsmuster"),
        CandlestickPattern::ThreeBlackCrows => ("Three Black Crows", "Starkes bärisches Fortsetzungsmuster"),
        CandlestickPattern::ThreeInsideUp => ("Three Inside Up", "Bullisches Bestätigungsmuster"),
        CandlestickPattern::ThreeInsideDown => ("Three Inside Down", "Bärisches Bestätigungsmuster"),
    }
}

// ============================================================================
// Main Pattern Detection
// ============================================================================

pub fn detect_candlestick_patterns(data: &[OHLCData]) -> Vec<PatternMatch> {
    let mut patterns = Vec::new();
    let n = data.len();
    if n < 10 { return patterns; }

    let lookback = 20.min(n - 5);
    let start_index = n - lookback;

    for i in start_index..n {
        let candle = &data[i];
        let avg_body = average_body(data, i, 10);
        let in_downtrend = is_in_downtrend(data, i, 5);
        let in_uptrend = is_in_uptrend(data, i, 5);

        // Single candle patterns
        if detect_doji(candle, avg_body) {
            let (name, desc) = pattern_info(&CandlestickPattern::Doji);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::Doji, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Neutral, reliability: PatternReliability::Medium,
                description: desc.to_string(),
            });
        }

        if detect_hammer(candle, avg_body, in_downtrend) {
            let (name, desc) = pattern_info(&CandlestickPattern::Hammer);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::Hammer, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                description: desc.to_string(),
            });
        }

        if detect_inverted_hammer(candle, avg_body, in_downtrend) {
            let (name, desc) = pattern_info(&CandlestickPattern::InvertedHammer);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::InvertedHammer, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Bullish, reliability: PatternReliability::Medium,
                description: desc.to_string(),
            });
        }

        if detect_hanging_man(candle, avg_body, in_uptrend) {
            let (name, desc) = pattern_info(&CandlestickPattern::HangingMan);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::HangingMan, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Bearish, reliability: PatternReliability::Medium,
                description: desc.to_string(),
            });
        }

        if detect_shooting_star(candle, avg_body, in_uptrend) {
            let (name, desc) = pattern_info(&CandlestickPattern::ShootingStar);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::ShootingStar, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                description: desc.to_string(),
            });
        }

        if detect_spinning_top(candle, avg_body) {
            let (name, desc) = pattern_info(&CandlestickPattern::SpinningTop);
            patterns.push(PatternMatch {
                pattern: CandlestickPattern::SpinningTop, name: name.to_string(),
                start_index: i, end_index: i,
                direction: PatternDirection::Neutral, reliability: PatternReliability::Low,
                description: desc.to_string(),
            });
        }

        if let Some(bullish) = detect_marubozu(candle, avg_body) {
            let pat = if bullish { CandlestickPattern::MarubozuBullish } else { CandlestickPattern::MarubozuBearish };
            let (name, desc) = pattern_info(&pat);
            patterns.push(PatternMatch {
                pattern: pat, name: name.to_string(),
                start_index: i, end_index: i,
                direction: if bullish { PatternDirection::Bullish } else { PatternDirection::Bearish },
                reliability: PatternReliability::High,
                description: desc.to_string(),
            });
        }

        // Two candle patterns
        if i > 0 {
            let prev = &data[i - 1];

            if detect_engulfing_bullish(prev, candle, in_downtrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::EngulfingBullish);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::EngulfingBullish, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_engulfing_bearish(prev, candle, in_uptrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::EngulfingBearish);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::EngulfingBearish, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_harami_bullish(prev, candle, in_downtrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::HaramiBullish);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::HaramiBullish, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::Medium,
                    description: desc.to_string(),
                });
            }

            if detect_harami_bearish(prev, candle, in_uptrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::HaramiBearish);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::HaramiBearish, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::Medium,
                    description: desc.to_string(),
                });
            }

            if detect_piercing_line(prev, candle, in_downtrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::PiercingLine);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::PiercingLine, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_dark_cloud_cover(prev, candle, in_uptrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::DarkCloudCover);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::DarkCloudCover, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_tweezer_top(prev, candle, in_uptrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::TweezerTop);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::TweezerTop, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::Medium,
                    description: desc.to_string(),
                });
            }

            if detect_tweezer_bottom(prev, candle, in_downtrend) {
                let (name, desc) = pattern_info(&CandlestickPattern::TweezerBottom);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::TweezerBottom, name: name.to_string(),
                    start_index: i - 1, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::Medium,
                    description: desc.to_string(),
                });
            }
        }

        // Three candle patterns
        if i > 1 {
            let first = &data[i - 2];
            let second = &data[i - 1];
            let third = candle;
            let in_downtrend_first = is_in_downtrend(data, i - 2, 5);
            let in_uptrend_first = is_in_uptrend(data, i - 2, 5);

            if detect_morning_star(first, second, third, avg_body, in_downtrend_first) {
                let (name, desc) = pattern_info(&CandlestickPattern::MorningStar);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::MorningStar, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_evening_star(first, second, third, avg_body, in_uptrend_first) {
                let (name, desc) = pattern_info(&CandlestickPattern::EveningStar);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::EveningStar, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_three_white_soldiers(first, second, third, avg_body) {
                let (name, desc) = pattern_info(&CandlestickPattern::ThreeWhiteSoldiers);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::ThreeWhiteSoldiers, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_three_black_crows(first, second, third, avg_body) {
                let (name, desc) = pattern_info(&CandlestickPattern::ThreeBlackCrows);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::ThreeBlackCrows, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_three_inside_up(first, second, third, in_downtrend_first) {
                let (name, desc) = pattern_info(&CandlestickPattern::ThreeInsideUp);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::ThreeInsideUp, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bullish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }

            if detect_three_inside_down(first, second, third, in_uptrend_first) {
                let (name, desc) = pattern_info(&CandlestickPattern::ThreeInsideDown);
                patterns.push(PatternMatch {
                    pattern: CandlestickPattern::ThreeInsideDown, name: name.to_string(),
                    start_index: i - 2, end_index: i,
                    direction: PatternDirection::Bearish, reliability: PatternReliability::High,
                    description: desc.to_string(),
                });
            }
        }
    }

    // Remove duplicates: keep highest reliability per end_index
    let mut best: std::collections::HashMap<usize, PatternMatch> = std::collections::HashMap::new();
    for p in patterns {
        let key = p.end_index;
        if let Some(existing) = best.get(&key) {
            if p.reliability > existing.reliability {
                best.insert(key, p);
            }
        } else {
            best.insert(key, p);
        }
    }

    let mut result: Vec<PatternMatch> = best.into_values().collect();
    result.sort_by(|a, b| b.end_index.cmp(&a.end_index));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        let data: Vec<OHLCData> = Vec::new();
        let patterns = detect_candlestick_patterns(&data);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_insufficient_data() {
        let data: Vec<OHLCData> = (0..5).map(|i| OHLCData {
            time: format!("2024-01-{:02}", i + 1),
            open: 100.0, high: 101.0, low: 99.0, close: 100.5, volume: Some(1000.0),
        }).collect();
        let patterns = detect_candlestick_patterns(&data);
        assert!(patterns.is_empty());
    }
}
