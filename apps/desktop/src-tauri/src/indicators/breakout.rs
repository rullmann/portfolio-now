use super::types::*;

/// Calculate the 20-day high from close prices
fn high_20d(data: &[OHLCData]) -> Option<f64> {
    if data.len() < 20 {
        return None;
    }
    let slice = &data[data.len() - 20..];
    slice.iter().map(|d| d.high).reduce(f64::max)
}

/// Simple SMA helper
fn sma(data: &[OHLCData], period: usize) -> Option<f64> {
    if data.len() < period {
        return None;
    }
    let slice = &data[data.len() - period..];
    Some(slice.iter().map(|d| d.close).sum::<f64>() / period as f64)
}

/// Average volume over last N days (excluding today)
fn avg_volume(data: &[OHLCData], period: usize) -> Option<f64> {
    let n = data.len();
    if n < period + 1 {
        return None;
    }
    let slice = &data[n - period - 1..n - 1];
    let sum: f64 = slice.iter().filter_map(|d| d.volume).sum();
    let count = slice.iter().filter(|d| d.volume.is_some()).count();
    if count == 0 {
        return None;
    }
    Some(sum / count as f64)
}

/// Rule 1: Trend Filter (Close vs MA50/MA200)
/// 2 = Clear uptrend (Close > MA50 AND MA50 > MA200)
/// 1 = Neutral/transition (Close > MA50, but MA50 not yet > MA200)
/// 0 = Downtrend (Close < MA50 and/or MA50 < MA200)
fn rule_trend_filter(data: &[OHLCData]) -> BreakoutRuleScore {
    let close = data.last().map(|d| d.close).unwrap_or(0.0);
    let ma50 = sma(data, 50);
    let ma200 = sma(data, 200);

    let (points, desc) = match (ma50, ma200) {
        (Some(m50), Some(m200)) => {
            if close > m50 && m50 > m200 {
                (
                    2,
                    format!(
                        "Aufwärtstrend: Kurs {:.2} > MA50 {:.2} > MA200 {:.2}",
                        close, m50, m200
                    ),
                )
            } else if close > m50 {
                (
                    1,
                    format!(
                        "Neutral: Kurs {:.2} > MA50 {:.2}, aber MA50 < MA200 {:.2}",
                        close, m50, m200
                    ),
                )
            } else {
                (
                    0,
                    format!("Abwärtstrend: Kurs {:.2} < MA50 {:.2}", close, m50),
                )
            }
        }
        (Some(m50), None) => {
            if close > m50 {
                (
                    1,
                    format!(
                        "Kurs {:.2} > MA50 {:.2} (MA200 nicht verfügbar)",
                        close, m50
                    ),
                )
            } else {
                (0, format!("Kurs {:.2} < MA50 {:.2}", close, m50))
            }
        }
        _ => (0, "Nicht genügend Daten für Trendanalyse".to_string()),
    };

    BreakoutRuleScore {
        rule_name: "Trendfilter".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Rule 2: Consolidation + Level (tight range near 20D high)
/// Measures: Range width of last 5 days + proximity to 20D high
/// 2 = Tight consolidation AND near 20D high
/// 1 = Some consolidation but not tight or not near level
/// 0 = No consolidation (chaotic/wide range)
fn rule_consolidation(data: &[OHLCData]) -> BreakoutRuleScore {
    let n = data.len();
    if n < 20 {
        return BreakoutRuleScore {
            rule_name: "Konsolidierung".to_string(),
            points: 0,
            max_points: 2,
            description: "Nicht genügend Daten".to_string(),
        };
    }

    let close = data[n - 1].close;
    let high20 = high_20d(data).unwrap_or(close);

    // Range of last 5 days (high-low range as % of close)
    let last5 = &data[n - 5..];
    let range_high = last5
        .iter()
        .map(|d| d.high)
        .reduce(f64::max)
        .unwrap_or(close);
    let range_low = last5
        .iter()
        .map(|d| d.low)
        .reduce(f64::min)
        .unwrap_or(close);
    let range_pct = if close > 0.0 {
        ((range_high - range_low) / close) * 100.0
    } else {
        100.0
    };

    // Distance from 20D high (% below)
    let dist_to_high = if high20 > 0.0 {
        ((high20 - close) / high20) * 100.0
    } else {
        100.0
    };

    let tight = range_pct < 5.0; // Less than 5% range = tight
    let near_high = dist_to_high < 2.0; // Within 2% of 20D high

    let (points, desc) = if tight && near_high {
        (
            2,
            format!(
                "Enge Konsolidierung ({:.1}% Range) nahe 20D-Hoch ({:.1}% entfernt)",
                range_pct, dist_to_high
            ),
        )
    } else if tight || near_high {
        (
            1,
            format!(
                "Teilweise Konsolidierung (Range: {:.1}%, Abstand zum Hoch: {:.1}%)",
                range_pct, dist_to_high
            ),
        )
    } else {
        (
            0,
            format!(
                "Keine Konsolidierung (Range: {:.1}%, Abstand: {:.1}%)",
                range_pct, dist_to_high
            ),
        )
    };

    BreakoutRuleScore {
        rule_name: "Konsolidierung".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Rule 3: Breakout Trigger (Close > 20D High)
/// 2 = Close above 20D high (real trigger)
/// 1 = Intraday above but close just below (knocking)
/// 0 = Below level, no trigger
fn rule_breakout_trigger(data: &[OHLCData]) -> BreakoutRuleScore {
    let n = data.len();
    if n < 21 {
        return BreakoutRuleScore {
            rule_name: "Breakout-Trigger".to_string(),
            points: 0,
            max_points: 2,
            description: "Nicht genügend Daten".to_string(),
        };
    }

    let today = &data[n - 1];
    // 20D high EXCLUDING today (the level to break)
    let prev_data = &data[..n - 1];
    let high20_prev = if prev_data.len() >= 20 {
        prev_data[prev_data.len() - 20..]
            .iter()
            .map(|d| d.high)
            .reduce(f64::max)
            .unwrap_or(0.0)
    } else {
        prev_data
            .iter()
            .map(|d| d.high)
            .reduce(f64::max)
            .unwrap_or(0.0)
    };

    let (points, desc) = if today.close > high20_prev {
        (
            2,
            format!(
                "Ausbruch bestätigt: Close {:.2} > 20D-Hoch {:.2}",
                today.close, high20_prev
            ),
        )
    } else if today.high > high20_prev && today.close <= high20_prev {
        let diff_pct = ((high20_prev - today.close) / high20_prev) * 100.0;
        (
            1,
            format!(
                "Intraday über 20D-Hoch, Close {:.1}% darunter",
                diff_pct
            ),
        )
    } else {
        let diff_pct = ((high20_prev - today.close) / high20_prev) * 100.0;
        (
            0,
            format!(
                "Unter 20D-Hoch ({:.2}), Abstand: {:.1}%",
                high20_prev, diff_pct
            ),
        )
    };

    BreakoutRuleScore {
        rule_name: "Breakout-Trigger".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Rule 4: Volume Confirmation
/// 2 = Volume significantly above average (>1.5x AvgVol20)
/// 1 = Volume slightly above average (1.1x-1.5x)
/// 0 = Volume normal or below
fn rule_volume_confirmation(data: &[OHLCData]) -> BreakoutRuleScore {
    let current_vol = data.last().and_then(|d| d.volume).unwrap_or(0.0);
    let avg_vol = avg_volume(data, 20).unwrap_or(0.0);

    if avg_vol <= 0.0 || current_vol <= 0.0 {
        return BreakoutRuleScore {
            rule_name: "Volumen".to_string(),
            points: 0,
            max_points: 2,
            description: "Keine Volumendaten verfügbar".to_string(),
        };
    }

    let ratio = current_vol / avg_vol;

    let (points, desc) = if ratio > 1.5 {
        (
            2,
            format!(
                "Starkes Volumen: {:.1}x Durchschnitt ({:.0} vs Ø {:.0})",
                ratio, current_vol, avg_vol
            ),
        )
    } else if ratio > 1.1 {
        (
            1,
            format!("Leicht erhöhtes Volumen: {:.1}x Durchschnitt", ratio),
        )
    } else {
        (
            0,
            format!("Normales Volumen: {:.1}x Durchschnitt", ratio),
        )
    };

    BreakoutRuleScore {
        rule_name: "Volumen".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Rule 5: Pullback Quality
/// Checks if there was a controlled pullback toward MA20/MA50 followed by a reversal
/// 2 = Controlled pullback + price turning up again
/// 1 = Pullback present but reversal not confirmed
/// 0 = No pullback or trend damaged
fn rule_pullback_quality(data: &[OHLCData]) -> BreakoutRuleScore {
    let n = data.len();
    if n < 50 {
        return BreakoutRuleScore {
            rule_name: "Pullback-Qualität".to_string(),
            points: 0,
            max_points: 2,
            description: "Nicht genügend Daten".to_string(),
        };
    }

    let close = data[n - 1].close;
    let ma20 = sma(data, 20).unwrap_or(close);
    let ma50 = sma(data, 50).unwrap_or(close);

    // Check last 10 days for a pullback pattern:
    // Did price dip toward MA20 or MA50, and then close higher for last 2 days?
    let last10 = &data[n - 10..];
    let min_close_10d = last10
        .iter()
        .map(|d| d.close)
        .reduce(f64::min)
        .unwrap_or(close);

    // Was there a dip toward MA20 or MA50?
    let dipped_to_ma20 = min_close_10d <= ma20 * 1.02; // Within 2% above MA20
    let dipped_to_ma50 = min_close_10d <= ma50 * 1.02;
    let had_pullback = dipped_to_ma20 || dipped_to_ma50;

    // Is price turning back up? (last 2 closes rising)
    let turning_up =
        n >= 3 && data[n - 1].close > data[n - 2].close && data[n - 2].close > data[n - 3].close;

    // Is trend still intact? (close above MA50)
    let trend_intact = close > ma50;

    let (points, desc) = if had_pullback && turning_up && trend_intact {
        (
            2,
            "Kontrollierter Pullback zu MA20/50 + Kursdrehung nach oben".to_string(),
        )
    } else if had_pullback && trend_intact {
        (
            1,
            "Pullback vorhanden, Drehung noch nicht bestätigt".to_string(),
        )
    } else if !trend_intact {
        (
            0,
            format!(
                "Trend beschädigt: Kurs {:.2} unter MA50 {:.2}",
                close, ma50
            ),
        )
    } else {
        (
            0,
            "Kein erkennbarer Pullback in den letzten 10 Tagen".to_string(),
        )
    };

    BreakoutRuleScore {
        rule_name: "Pullback-Qualität".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Rule 6: Relative Strength
/// Based on 3-month (60 trading days) performance percentile within the universe
/// 2 = Top 20%
/// 1 = 20-50%
/// 0 = Bottom 50%
/// `percentile` is 0.0..1.0 where 1.0 = best performer
fn rule_relative_strength(data: &[OHLCData], percentile: Option<f64>) -> BreakoutRuleScore {
    let n = data.len();

    // Calculate 3-month performance
    let perf_3m = if n >= 60 {
        let old = data[n - 60].close;
        if old > 0.0 {
            Some(((data[n - 1].close - old) / old) * 100.0)
        } else {
            None
        }
    } else {
        None
    };

    let pct = percentile.unwrap_or(0.5); // Default to middle if no universe ranking

    let (points, desc) = if pct >= 0.8 {
        (
            2,
            format!(
                "Top {:.0}% im Universum (3M: {:.1}%)",
                (1.0 - pct) * 100.0,
                perf_3m.unwrap_or(0.0)
            ),
        )
    } else if pct >= 0.5 {
        (
            1,
            format!(
                "Mittelfeld: Top {:.0}% (3M: {:.1}%)",
                (1.0 - pct) * 100.0,
                perf_3m.unwrap_or(0.0)
            ),
        )
    } else {
        (
            0,
            format!(
                "Unteres Feld: {:.0}. Perzentil (3M: {:.1}%)",
                pct * 100.0,
                perf_3m.unwrap_or(0.0)
            ),
        )
    };

    BreakoutRuleScore {
        rule_name: "Relative Stärke".to_string(),
        points,
        max_points: 2,
        description: desc,
    }
}

/// Main scoring function: calculates breakout score for a security
/// `relative_strength_percentile`: 0.0-1.0, position in the universe (1.0 = best)
pub fn score_breakout(
    data: &[OHLCData],
    relative_strength_percentile: Option<f64>,
) -> BreakoutScore {
    if data.len() < 20 {
        return BreakoutScore {
            total_points: 0,
            max_points: 12,
            classification: BreakoutClassification::Unlikely,
            classification_label: "Nicht genügend Daten".to_string(),
            rules: vec![],
            downgraded: false,
            downgrade_reason: None,
        };
    }

    let r1 = rule_trend_filter(data);
    let r2 = rule_consolidation(data);
    let r3 = rule_breakout_trigger(data);
    let r4 = rule_volume_confirmation(data);
    let r5 = rule_pullback_quality(data);
    let r6 = rule_relative_strength(data, relative_strength_percentile);

    let raw_total = r1.points + r2.points + r3.points + r4.points + r5.points + r6.points;

    // Classification from raw score
    let mut classification = if raw_total >= 10 {
        BreakoutClassification::VeryLikely
    } else if raw_total >= 7 {
        BreakoutClassification::Likely
    } else if raw_total >= 4 {
        BreakoutClassification::Possible
    } else {
        BreakoutClassification::Unlikely
    };

    let mut downgraded = false;
    let mut downgrade_reason = None;

    // Cap rule: If trend (rule 1) is 0 (downtrend), cap at "Possible"
    if r1.points == 0 {
        match classification {
            BreakoutClassification::VeryLikely | BreakoutClassification::Likely => {
                classification = BreakoutClassification::Possible;
                downgraded = true;
                downgrade_reason =
                    Some("Gekappt: Abwärtstrend → max. \"möglich\"".to_string());
            }
            _ => {}
        }
    }

    // Cap rule: Trigger (rule 3) = 2 but Volume (rule 4) = 0 → downgrade one level
    if r3.points == 2 && r4.points == 0 {
        classification = match classification {
            BreakoutClassification::VeryLikely => BreakoutClassification::Likely,
            BreakoutClassification::Likely => BreakoutClassification::Possible,
            other => other,
        };
        downgraded = true;
        let reason = downgrade_reason.unwrap_or_default();
        let new_reason = if reason.is_empty() {
            "Ausbruch ohne Volumen → eine Stufe runter".to_string()
        } else {
            format!("{} + Ausbruch ohne Volumen", reason)
        };
        downgrade_reason = Some(new_reason);
    }

    let classification_label = match classification {
        BreakoutClassification::VeryLikely => "Ausbruch sehr wahrscheinlich".to_string(),
        BreakoutClassification::Likely => "Ausbruch wahrscheinlich".to_string(),
        BreakoutClassification::Possible => "Ausbruch möglich".to_string(),
        BreakoutClassification::Unlikely => "Ausbruch unwahrscheinlich".to_string(),
    };

    BreakoutScore {
        total_points: raw_total,
        max_points: 12,
        classification,
        classification_label,
        rules: vec![r1, r2, r3, r4, r5, r6],
        downgraded,
        downgrade_reason,
    }
}

/// Calculate relative strength percentiles for a set of securities
/// Returns a map of index -> percentile (0.0 = worst, 1.0 = best)
pub fn calculate_relative_strength_percentiles(all_data: &[&[OHLCData]]) -> Vec<f64> {
    let mut perfs: Vec<(usize, f64)> = all_data
        .iter()
        .enumerate()
        .map(|(i, data)| {
            let n = data.len();
            let perf = if n >= 60 {
                let old = data[n - 60].close;
                if old > 0.0 {
                    ((data[n - 1].close - old) / old) * 100.0
                } else {
                    0.0
                }
            } else if n >= 2 {
                let old = data[0].close;
                if old > 0.0 {
                    ((data[n - 1].close - old) / old) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            (i, perf)
        })
        .collect();

    // Sort by performance ascending
    perfs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let total = perfs.len() as f64;
    let mut percentiles = vec![0.5f64; all_data.len()];

    for (rank, (idx, _)) in perfs.iter().enumerate() {
        percentiles[*idx] = if total > 1.0 {
            rank as f64 / (total - 1.0)
        } else {
            0.5
        };
    }

    percentiles
}
