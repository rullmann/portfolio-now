//! Advanced Analysis Module
//!
//! Provides professional-grade financial analysis calculations:
//! - CVaR (Conditional Value at Risk / Expected Shortfall)
//! - Monte Carlo Simulation (Geometric Brownian Motion)
//! - Drawdown Series (full underwater equity curve)
//! - Rolling Returns (1M, 3M, 6M, 1Y, 3Y, 5Y windows)
//! - GARCH(1,1) Volatility Forecasting
//!
//! All calculations are pure functions operating on price/return data.
//! No database access — data is passed in from commands.

use serde::Serialize;

// ============================================================================
// Types
// ============================================================================

/// A date-value pair for time series data
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateValue {
    pub date: String,
    pub value: f64,
}

/// VaR and CVaR result at multiple confidence levels
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaRResult {
    /// Value at Risk at 95% confidence (daily, as negative percentage)
    pub var_95: f64,
    /// Conditional VaR (Expected Shortfall) at 95%
    pub cvar_95: f64,
    /// Value at Risk at 99% confidence
    pub var_99: f64,
    /// Conditional VaR at 99%
    pub cvar_99: f64,
    /// Number of return observations used
    pub observations: usize,
    /// Annualized VaR 95%
    pub var_95_annual: f64,
    /// Annualized CVaR 95%
    pub cvar_95_annual: f64,
    /// AI context: human-readable risk assessment
    pub ai_summary: String,
}

/// A single drawdown period
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawdownPeriod {
    pub start_date: String,
    pub trough_date: String,
    pub recovery_date: Option<String>,
    pub depth: f64,           // Max drawdown depth as negative percentage
    pub duration_days: i64,   // Days from start to trough
    pub recovery_days: Option<i64>, // Days from trough to recovery (None if not recovered)
}

/// Full drawdown analysis result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawdownResult {
    /// Daily drawdown series (for charting)
    pub series: Vec<DateValue>,
    /// Top drawdown periods sorted by depth
    pub worst_drawdowns: Vec<DrawdownPeriod>,
    /// Current drawdown (0.0 if at all-time high)
    pub current_drawdown: f64,
    /// Average drawdown
    pub average_drawdown: f64,
    /// AI context
    pub ai_summary: String,
}

/// Rolling returns result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollingReturnsResult {
    /// Window size in trading days
    pub window_days: usize,
    /// Window label (e.g. "1Y", "3M")
    pub window_label: String,
    /// Rolling return series
    pub series: Vec<DateValue>,
    /// Best rolling return
    pub best: f64,
    /// Worst rolling return
    pub worst: f64,
    /// Average rolling return
    pub average: f64,
    /// Median rolling return
    pub median: f64,
    /// Percentage of positive periods
    pub positive_pct: f64,
    /// AI context
    pub ai_summary: String,
}

/// Monte Carlo simulation result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloResult {
    /// Percentile paths for fan chart (5%, 25%, 50%, 75%, 95%)
    pub percentile_5: Vec<DateValue>,
    pub percentile_25: Vec<DateValue>,
    pub percentile_50: Vec<DateValue>,
    pub percentile_75: Vec<DateValue>,
    pub percentile_95: Vec<DateValue>,
    /// Final value distribution statistics
    pub final_mean: f64,
    pub final_median: f64,
    pub final_std: f64,
    /// Probability of loss
    pub prob_loss: f64,
    /// Probability of >10% gain
    pub prob_gain_10: f64,
    /// Probability of >50% gain
    pub prob_gain_50: f64,
    /// Number of simulations run
    pub num_simulations: usize,
    /// Number of days simulated
    pub days_forward: usize,
    /// AI context
    pub ai_summary: String,
}

/// GARCH(1,1) volatility forecast result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GarchResult {
    /// Historical conditional volatility (annualized)
    pub historical_volatility: Vec<DateValue>,
    /// Forecast volatility (annualized)
    pub forecast_volatility: Vec<DateValue>,
    /// GARCH parameters
    pub omega: f64,
    pub alpha: f64,
    pub beta: f64,
    /// Persistence (alpha + beta, should be < 1)
    pub persistence: f64,
    /// Long-run variance
    pub long_run_volatility: f64,
    /// Current conditional volatility
    pub current_volatility: f64,
    /// Did the estimation converge?
    pub converged: bool,
    /// AI context
    pub ai_summary: String,
}

// ============================================================================
// Helper: Calculate log returns from prices
// ============================================================================

fn log_returns(prices: &[(String, f64)]) -> Vec<f64> {
    prices.windows(2)
        .filter_map(|w| {
            if w[0].1 > 0.0 && w[1].1 > 0.0 {
                Some((w[1].1 / w[0].1).ln())
            } else {
                None
            }
        })
        .collect()
}

fn simple_returns(prices: &[(String, f64)]) -> Vec<f64> {
    prices.windows(2)
        .filter_map(|w| {
            if w[0].1 > 0.0 {
                Some((w[1].1 - w[0].1) / w[0].1)
            } else {
                None
            }
        })
        .collect()
}

// ============================================================================
// 1. CVaR (Conditional Value at Risk)
// ============================================================================

/// Calculate VaR and CVaR from price series
///
/// Uses historical simulation method (non-parametric):
/// - Sort returns ascending
/// - VaR(α) = return at (1-α) percentile
/// - CVaR(α) = mean of returns below VaR(α)
pub fn calculate_var_cvar(prices: &[(String, f64)]) -> VaRResult {
    let returns = simple_returns(prices);

    if returns.len() < 30 {
        return VaRResult {
            var_95: 0.0, cvar_95: 0.0, var_99: 0.0, cvar_99: 0.0,
            observations: returns.len(),
            var_95_annual: 0.0, cvar_95_annual: 0.0,
            ai_summary: "Nicht genug Datenpunkte für VaR/CVaR-Berechnung (min. 30 benötigt).".into(),
        };
    }

    let mut sorted = returns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();

    // VaR at 95% = 5th percentile of returns
    let idx_95 = ((n as f64) * 0.05).floor() as usize;
    let idx_99 = ((n as f64) * 0.01).floor() as usize;

    let var_95 = sorted[idx_95];
    let var_99 = sorted[idx_99.max(0)];

    // CVaR = mean of returns <= VaR
    let cvar_95 = if idx_95 > 0 {
        sorted[..=idx_95].iter().sum::<f64>() / (idx_95 + 1) as f64
    } else {
        var_95
    };

    let cvar_99 = if idx_99 > 0 {
        sorted[..=idx_99].iter().sum::<f64>() / (idx_99 + 1) as f64
    } else {
        var_99
    };

    // Annualize: daily VaR * sqrt(252)
    let sqrt_252 = 252.0_f64.sqrt();
    let var_95_annual = var_95 * sqrt_252;
    let cvar_95_annual = cvar_95 * sqrt_252;

    let ai_summary = format!(
        "VaR/CVaR-Analyse basierend auf {} Tagen: \
         Täglicher VaR(95%) = {:.2}%, CVaR(95%) = {:.2}%. \
         Das bedeutet: An 95% der Tage verliert das Portfolio maximal {:.2}%, \
         aber an den schlechtesten 5% der Tage beträgt der durchschnittliche Verlust {:.2}%. \
         Annualisiert: VaR(95%) = {:.1}%, CVaR(95%) = {:.1}%.",
        n,
        var_95 * 100.0, cvar_95 * 100.0,
        -var_95 * 100.0, -cvar_95 * 100.0,
        var_95_annual * 100.0, cvar_95_annual * 100.0,
    );

    VaRResult {
        var_95: var_95 * 100.0,
        cvar_95: cvar_95 * 100.0,
        var_99: var_99 * 100.0,
        cvar_99: cvar_99 * 100.0,
        observations: n,
        var_95_annual: var_95_annual * 100.0,
        cvar_95_annual: cvar_95_annual * 100.0,
        ai_summary,
    }
}

// ============================================================================
// 2. Drawdown Analysis
// ============================================================================

/// Calculate full drawdown series and identify worst drawdown periods
pub fn calculate_drawdowns(prices: &[(String, f64)]) -> DrawdownResult {
    if prices.len() < 2 {
        return DrawdownResult {
            series: vec![], worst_drawdowns: vec![],
            current_drawdown: 0.0, average_drawdown: 0.0,
            ai_summary: "Nicht genug Datenpunkte.".into(),
        };
    }

    // Calculate drawdown series
    let mut peak = prices[0].1;
    let mut series = Vec::with_capacity(prices.len());
    let mut dd_sum = 0.0;

    for (date, price) in prices {
        if *price > peak {
            peak = *price;
        }
        let dd = if peak > 0.0 { (*price - peak) / peak } else { 0.0 };
        series.push(DateValue { date: date.clone(), value: dd * 100.0 });
        dd_sum += dd;
    }

    let current_drawdown = series.last().map(|d| d.value).unwrap_or(0.0);
    let average_drawdown = dd_sum / prices.len() as f64 * 100.0;

    // Identify distinct drawdown periods
    let mut periods: Vec<DrawdownPeriod> = Vec::new();
    let mut in_dd = false;
    let mut dd_start_idx = 0;
    let mut dd_trough_idx = 0;
    let mut dd_trough_val = 0.0_f64;

    for (i, (_, price)) in prices.iter().enumerate() {
        let dd = if prices[0..=i].iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) > 0.0 {
            let peak_so_far = prices[0..=i].iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            (*price - peak_so_far) / peak_so_far
        } else {
            0.0
        };

        if dd < -0.001 && !in_dd {
            // Start of new drawdown
            in_dd = true;
            dd_start_idx = if i > 0 { i - 1 } else { 0 };
            dd_trough_idx = i;
            dd_trough_val = dd;
        } else if in_dd && dd < dd_trough_val {
            dd_trough_idx = i;
            dd_trough_val = dd;
        } else if in_dd && dd >= -0.001 {
            // Recovery
            let start_date_str = prices[dd_start_idx].0.clone();
            let trough_date_str = prices[dd_trough_idx].0.clone();

            if let (Ok(sd), Ok(td)) = (
                chrono::NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d"),
                chrono::NaiveDate::parse_from_str(&trough_date_str, "%Y-%m-%d"),
            ) {
                let recovery_date = chrono::NaiveDate::parse_from_str(&prices[i].0, "%Y-%m-%d").ok();
                periods.push(DrawdownPeriod {
                    start_date: start_date_str,
                    trough_date: trough_date_str,
                    recovery_date: Some(prices[i].0.clone()),
                    depth: dd_trough_val * 100.0,
                    duration_days: (td - sd).num_days(),
                    recovery_days: recovery_date.map(|rd| (rd - td).num_days()),
                });
            }
            in_dd = false;
        }
    }

    // Handle ongoing drawdown
    if in_dd {
        let start_date_str = prices[dd_start_idx].0.clone();
        let trough_date_str = prices[dd_trough_idx].0.clone();
        if let (Ok(sd), Ok(td)) = (
            chrono::NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d"),
            chrono::NaiveDate::parse_from_str(&trough_date_str, "%Y-%m-%d"),
        ) {
            periods.push(DrawdownPeriod {
                start_date: start_date_str,
                trough_date: trough_date_str,
                recovery_date: None,
                depth: dd_trough_val * 100.0,
                duration_days: (td - sd).num_days(),
                recovery_days: None,
            });
        }
    }

    // Sort by depth (most negative first)
    periods.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));
    let worst_drawdowns: Vec<DrawdownPeriod> = periods.into_iter().take(10).collect();

    let ai_summary = if worst_drawdowns.is_empty() {
        "Keine signifikanten Drawdowns gefunden.".into()
    } else {
        let worst = &worst_drawdowns[0];
        format!(
            "Drawdown-Analyse: {} Drawdown-Perioden identifiziert. \
             Schlimmster Drawdown: {:.1}% ({} bis {}{}). \
             Aktueller Drawdown: {:.1}%. Durchschnittlicher Drawdown: {:.1}%.",
            worst_drawdowns.len(),
            worst.depth, worst.start_date, worst.trough_date,
            worst.recovery_date.as_ref().map(|d| format!(", erholt am {}", d)).unwrap_or_else(|| ", noch nicht erholt".into()),
            current_drawdown, average_drawdown,
        )
    };

    DrawdownResult {
        series,
        worst_drawdowns,
        current_drawdown,
        average_drawdown,
        ai_summary,
    }
}

// ============================================================================
// 3. Rolling Returns
// ============================================================================

/// Calculate rolling returns for a given window size
///
/// Window sizes in trading days:
/// 1M ≈ 21, 3M ≈ 63, 6M ≈ 126, 1Y ≈ 252, 3Y ≈ 756, 5Y ≈ 1260
pub fn calculate_rolling_returns(prices: &[(String, f64)], window_days: usize, window_label: &str) -> RollingReturnsResult {
    if prices.len() < window_days + 1 {
        return RollingReturnsResult {
            window_days, window_label: window_label.to_string(),
            series: vec![], best: 0.0, worst: 0.0, average: 0.0, median: 0.0, positive_pct: 0.0,
            ai_summary: format!("Nicht genug Daten für {}-Rolling-Returns (benötigt {} Tage, hat {}).", window_label, window_days, prices.len()),
        };
    }

    let mut series = Vec::with_capacity(prices.len() - window_days);
    let mut returns_vec = Vec::with_capacity(prices.len() - window_days);

    for i in window_days..prices.len() {
        let start_price = prices[i - window_days].1;
        let end_price = prices[i].1;
        if start_price > 0.0 {
            let ret = (end_price / start_price - 1.0) * 100.0;
            series.push(DateValue { date: prices[i].0.clone(), value: ret });
            returns_vec.push(ret);
        }
    }

    if returns_vec.is_empty() {
        return RollingReturnsResult {
            window_days, window_label: window_label.to_string(),
            series: vec![], best: 0.0, worst: 0.0, average: 0.0, median: 0.0, positive_pct: 0.0,
            ai_summary: "Keine gültigen Rolling Returns berechnet.".into(),
        };
    }

    let best = returns_vec.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let worst = returns_vec.iter().cloned().fold(f64::INFINITY, f64::min);
    let average = returns_vec.iter().sum::<f64>() / returns_vec.len() as f64;
    let positive_count = returns_vec.iter().filter(|r| **r > 0.0).count();
    let positive_pct = positive_count as f64 / returns_vec.len() as f64 * 100.0;

    let mut sorted = returns_vec.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    let ai_summary = format!(
        "Rolling {}-Returns ({} Perioden): Bester: {:.1}%, Schlechtester: {:.1}%, \
         Durchschnitt: {:.1}%, Median: {:.1}%. In {:.0}% der Perioden war die Rendite positiv.",
        window_label, returns_vec.len(), best, worst, average, median, positive_pct,
    );

    RollingReturnsResult {
        window_days,
        window_label: window_label.to_string(),
        series,
        best,
        worst,
        average,
        median,
        positive_pct,
        ai_summary,
    }
}

// ============================================================================
// 4. Monte Carlo Simulation
// ============================================================================

/// Run Monte Carlo simulation using Geometric Brownian Motion
///
/// Uses Box-Muller transform for normal random numbers (no external RNG crate needed)
pub fn run_monte_carlo(
    prices: &[(String, f64)],
    num_simulations: usize,
    days_forward: usize,
) -> MonteCarloResult {
    let returns = log_returns(prices);

    if returns.len() < 30 {
        return MonteCarloResult {
            percentile_5: vec![], percentile_25: vec![], percentile_50: vec![],
            percentile_75: vec![], percentile_95: vec![],
            final_mean: 0.0, final_median: 0.0, final_std: 0.0,
            prob_loss: 0.0, prob_gain_10: 0.0, prob_gain_50: 0.0,
            num_simulations, days_forward,
            ai_summary: "Nicht genug Daten für Monte Carlo (min. 30 Tage benötigt).".into(),
        };
    }

    let n = returns.len() as f64;
    let mu = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mu).powi(2)).sum::<f64>() / (n - 1.0);
    let sigma = variance.sqrt();

    let last_price = prices.last().map(|p| p.1).unwrap_or(100.0);
    // Simple LCG-based pseudo-random number generator (deterministic, fast)
    // Seed from last price + data length for reproducibility
    let mut rng_state: u64 = (last_price * 1000.0) as u64 ^ (prices.len() as u64 * 2654435761);

    let next_rand = |state: &mut u64| -> f64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*state >> 33) as f64 / (u32::MAX as f64)
    };

    // Box-Muller for normal distribution
    let next_normal = |state: &mut u64| -> f64 {
        let u1 = next_rand(state).max(1e-10);
        let u2 = next_rand(state);
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    };

    // Run simulations, store final values and daily paths
    let mut all_paths: Vec<Vec<f64>> = Vec::with_capacity(num_simulations);
    let mut final_values = Vec::with_capacity(num_simulations);

    // GBM drift: (mu - sigma²/2) per day
    let drift = mu - variance / 2.0;

    for _ in 0..num_simulations {
        let mut path = Vec::with_capacity(days_forward + 1);
        path.push(last_price);
        let mut price = last_price;

        for _ in 0..days_forward {
            let z = next_normal(&mut rng_state);
            price *= (drift + sigma * z).exp();
            path.push(price);
        }

        final_values.push(price);
        all_paths.push(path);
    }

    // Calculate percentile paths
    let make_percentile_path = |pct: f64| -> Vec<DateValue> {
        let mut path = Vec::with_capacity(days_forward + 1);
        for day in 0..=days_forward {
            let mut day_values: Vec<f64> = all_paths.iter().map(|p| p[day]).collect();
            day_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = ((day_values.len() as f64 * pct / 100.0).floor() as usize).min(day_values.len() - 1);
            path.push(DateValue {
                date: format!("T+{}", day),
                value: day_values[idx],
            });
        }
        path
    };

    let percentile_5 = make_percentile_path(5.0);
    let percentile_25 = make_percentile_path(25.0);
    let percentile_50 = make_percentile_path(50.0);
    let percentile_75 = make_percentile_path(75.0);
    let percentile_95 = make_percentile_path(95.0);

    // Final value statistics
    final_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let final_mean = final_values.iter().sum::<f64>() / final_values.len() as f64;
    let final_median = final_values[final_values.len() / 2];
    let final_std = {
        let mean = final_mean;
        (final_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (final_values.len() - 1) as f64).sqrt()
    };

    let prob_loss = final_values.iter().filter(|v| **v < last_price).count() as f64 / num_simulations as f64 * 100.0;
    let prob_gain_10 = final_values.iter().filter(|v| **v > last_price * 1.1).count() as f64 / num_simulations as f64 * 100.0;
    let prob_gain_50 = final_values.iter().filter(|v| **v > last_price * 1.5).count() as f64 / num_simulations as f64 * 100.0;

    let expected_return = (final_mean / last_price - 1.0) * 100.0;
    let median_return = (final_median / last_price - 1.0) * 100.0;

    let ai_summary = format!(
        "Monte Carlo Simulation ({} Simulationen, {} Tage, Startpreis {:.2}): \
         Erwartete Rendite: {:.1}%, Median-Rendite: {:.1}%. \
         Verlustwahrscheinlichkeit: {:.0}%. Wahrscheinlichkeit >10% Gewinn: {:.0}%. \
         5%-Szenario (worst case): {:.2}, 95%-Szenario (best case): {:.2}. \
         Historische Tagesvolatilität: {:.2}%, annualisiert: {:.1}%.",
        num_simulations, days_forward, last_price,
        expected_return, median_return,
        prob_loss, prob_gain_10,
        percentile_5.last().map(|d| d.value).unwrap_or(0.0),
        percentile_95.last().map(|d| d.value).unwrap_or(0.0),
        sigma * 100.0, sigma * 252.0_f64.sqrt() * 100.0,
    );

    MonteCarloResult {
        percentile_5, percentile_25, percentile_50, percentile_75, percentile_95,
        final_mean, final_median, final_std,
        prob_loss, prob_gain_10, prob_gain_50,
        num_simulations, days_forward,
        ai_summary,
    }
}

// ============================================================================
// 5. GARCH(1,1) Volatility Forecasting
// ============================================================================

/// Estimate GARCH(1,1) parameters and produce volatility forecast
///
/// Model: σ²_t = ω + α·ε²_{t-1} + β·σ²_{t-1}
///
/// Uses quasi-maximum likelihood with gradient descent optimization.
/// Parameters: ω > 0, α ≥ 0, β ≥ 0, α + β < 1 (stationarity)
pub fn calculate_garch(
    prices: &[(String, f64)],
    forecast_days: usize,
) -> GarchResult {
    let returns = log_returns(prices);

    if returns.len() < 50 {
        return GarchResult {
            historical_volatility: vec![], forecast_volatility: vec![],
            omega: 0.0, alpha: 0.0, beta: 0.0,
            persistence: 0.0, long_run_volatility: 0.0, current_volatility: 0.0,
            converged: false,
            ai_summary: "Nicht genug Daten für GARCH (min. 50 Tage benötigt).".into(),
        };
    }

    let n = returns.len();
    let mean_ret = returns.iter().sum::<f64>() / n as f64;
    let residuals: Vec<f64> = returns.iter().map(|r| r - mean_ret).collect();
    let sample_var = residuals.iter().map(|e| e.powi(2)).sum::<f64>() / (n - 1) as f64;

    // Initial parameters
    let mut omega = sample_var * 0.05;
    let mut alpha = 0.1;
    let mut beta = 0.85;

    // Optimization via coordinate descent (simple, robust)
    let mut converged = false;
    let mut best_ll = f64::NEG_INFINITY;

    for iteration in 0..200 {
        // Calculate conditional variances
        let mut sigma2 = Vec::with_capacity(n);
        sigma2.push(sample_var);

        for t in 1..n {
            let s2 = omega + alpha * residuals[t - 1].powi(2) + beta * sigma2[t - 1];
            sigma2.push(s2.max(1e-12));
        }

        // Log-likelihood
        let ll: f64 = sigma2.iter().zip(residuals.iter())
            .map(|(s2, e)| -0.5 * (s2.ln() + e.powi(2) / s2))
            .sum();

        if (ll - best_ll).abs() < 1e-6 && iteration > 10 {
            converged = true;
            break;
        }
        best_ll = ll;

        // Numerical gradient for each parameter
        let delta = 1e-8;

        // Gradient for omega
        let ll_omega = |o: f64| -> f64 {
            let mut s2 = vec![sample_var];
            for t in 1..n {
                s2.push((o + alpha * residuals[t-1].powi(2) + beta * s2[t-1]).max(1e-12));
            }
            s2.iter().zip(residuals.iter()).map(|(s, e)| -0.5 * (s.ln() + e.powi(2) / s)).sum()
        };
        let g_omega = (ll_omega(omega + delta) - ll_omega(omega - delta)) / (2.0 * delta);

        // Gradient for alpha
        let ll_alpha = |a: f64| -> f64 {
            let mut s2 = vec![sample_var];
            for t in 1..n {
                s2.push((omega + a * residuals[t-1].powi(2) + beta * s2[t-1]).max(1e-12));
            }
            s2.iter().zip(residuals.iter()).map(|(s, e)| -0.5 * (s.ln() + e.powi(2) / s)).sum()
        };
        let g_alpha = (ll_alpha(alpha + delta) - ll_alpha(alpha - delta)) / (2.0 * delta);

        // Gradient for beta
        let ll_beta = |b: f64| -> f64 {
            let mut s2 = vec![sample_var];
            for t in 1..n {
                s2.push((omega + alpha * residuals[t-1].powi(2) + b * s2[t-1]).max(1e-12));
            }
            s2.iter().zip(residuals.iter()).map(|(s, e)| -0.5 * (s.ln() + e.powi(2) / s)).sum()
        };
        let g_beta = (ll_beta(beta + delta) - ll_beta(beta - delta)) / (2.0 * delta);

        // Adaptive learning rate
        let lr = 1e-7 / (1.0 + iteration as f64 * 0.01);

        omega = (omega + lr * g_omega).max(1e-10);
        alpha = (alpha + lr * g_alpha).clamp(0.001, 0.5);
        beta = (beta + lr * g_beta).clamp(0.001, 0.998 - alpha);
    }

    // Final conditional variances
    let mut sigma2 = Vec::with_capacity(n);
    sigma2.push(sample_var);
    for t in 1..n {
        let s2 = omega + alpha * residuals[t - 1].powi(2) + beta * sigma2[t - 1];
        sigma2.push(s2.max(1e-12));
    }

    // Historical volatility series (annualized)
    let sqrt_252 = 252.0_f64.sqrt();
    // Only output every 5th day to keep series manageable
    let historical_volatility: Vec<DateValue> = sigma2.iter().enumerate()
        .filter(|(i, _)| i % 5 == 0 || *i == n - 1)
        .map(|(i, s2)| DateValue {
            date: prices[i + 1].0.clone(), // returns are offset by 1 from prices
            value: s2.sqrt() * sqrt_252 * 100.0,
        })
        .collect();

    // Forecast
    let persistence = alpha + beta;
    let long_run_var = if persistence < 1.0 { omega / (1.0 - persistence) } else { sample_var };
    let current_sigma2 = *sigma2.last().unwrap_or(&sample_var);
    let last_residual2 = residuals.last().map(|e| e.powi(2)).unwrap_or(sample_var);

    let mut forecast_volatility = Vec::with_capacity(forecast_days);
    let mut fc_sigma2 = omega + alpha * last_residual2 + beta * current_sigma2;

    for day in 1..=forecast_days {
        forecast_volatility.push(DateValue {
            date: format!("T+{}", day),
            value: fc_sigma2.sqrt() * sqrt_252 * 100.0,
        });
        // Forecast recursion: E[σ²_{t+k}] = ω + (α+β) · σ²_{t+k-1}
        // converges to long_run_var
        fc_sigma2 = omega + persistence * fc_sigma2;
    }

    let current_volatility = current_sigma2.sqrt() * sqrt_252 * 100.0;
    let long_run_volatility = long_run_var.sqrt() * sqrt_252 * 100.0;

    let ai_summary = format!(
        "GARCH(1,1)-Modell{}: ω={:.6}, α={:.3}, β={:.3}, Persistenz={:.3}. \
         Aktuelle bedingte Volatilität: {:.1}% (annualisiert). \
         Langfristige Volatilität: {:.1}%. {}",
        if converged { " (konvergiert)" } else { " (nicht konvergiert)" },
        omega, alpha, beta, persistence,
        current_volatility, long_run_volatility,
        if persistence > 0.99 {
            "Sehr hohe Persistenz — Volatilitätsschocks halten lange an."
        } else if persistence > 0.95 {
            "Hohe Persistenz — typisch für Aktienindizes."
        } else {
            "Moderate Persistenz — Volatilität normalisiert sich schnell."
        }
    );

    GarchResult {
        historical_volatility,
        forecast_volatility,
        omega, alpha, beta,
        persistence,
        long_run_volatility,
        current_volatility,
        converged,
        ai_summary,
    }
}
