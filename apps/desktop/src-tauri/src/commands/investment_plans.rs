//! Investment Plans (Sparpläne) Commands
//!
//! Manage recurring investment plans for automated purchases.

use crate::db;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use tauri::command;

/// Plan interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum PlanInterval {
    Weekly,
    Biweekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[allow(dead_code)]
impl PlanInterval {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Weekly => "WEEKLY",
            Self::Biweekly => "BIWEEKLY",
            Self::Monthly => "MONTHLY",
            Self::Quarterly => "QUARTERLY",
            Self::Yearly => "YEARLY",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "WEEKLY" => Some(Self::Weekly),
            "BIWEEKLY" => Some(Self::Biweekly),
            "MONTHLY" => Some(Self::Monthly),
            "QUARTERLY" => Some(Self::Quarterly),
            "YEARLY" => Some(Self::Yearly),
            _ => None,
        }
    }
}

/// Investment plan data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentPlanData {
    pub id: i64,
    pub name: String,
    pub security_id: i64,
    pub security_name: String,
    pub account_id: i64,
    pub account_name: String,
    pub portfolio_id: i64,
    pub portfolio_name: String,
    pub interval: String,
    pub amount: i64,  // cents
    pub currency: String,
    pub day_of_month: i32,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_active: bool,
    pub last_execution: Option<String>,
    pub next_execution: Option<String>,
    pub total_invested: i64,  // cents
    pub execution_count: i32,
}

/// Create plan request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvestmentPlanRequest {
    pub name: String,
    pub security_id: i64,
    pub account_id: i64,
    pub portfolio_id: i64,
    pub interval: String,
    pub amount: i64,  // cents
    pub day_of_month: i32,
    pub start_date: String,
    pub end_date: Option<String>,
}

/// Plan execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentPlanExecution {
    pub id: i64,
    pub plan_id: i64,
    pub date: String,
    pub shares: i64,  // scaled
    pub price: i64,   // scaled
    pub amount: i64,  // cents
    pub fees: i64,    // cents
    pub transaction_id: i64,
}

/// Ensure investment_plans table exists
fn ensure_tables(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pp_investment_plan (
            id INTEGER PRIMARY KEY,
            import_id INTEGER,
            name TEXT NOT NULL,
            security_id INTEGER NOT NULL,
            account_id INTEGER NOT NULL,
            portfolio_id INTEGER NOT NULL,
            interval TEXT NOT NULL,
            amount INTEGER NOT NULL,
            day_of_month INTEGER NOT NULL DEFAULT 1,
            start_date TEXT NOT NULL,
            end_date TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (security_id) REFERENCES pp_security(id),
            FOREIGN KEY (account_id) REFERENCES pp_account(id),
            FOREIGN KEY (portfolio_id) REFERENCES pp_portfolio(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pp_plan_execution (
            id INTEGER PRIMARY KEY,
            plan_id INTEGER NOT NULL,
            date TEXT NOT NULL,
            shares INTEGER NOT NULL,
            price INTEGER NOT NULL,
            amount INTEGER NOT NULL,
            fees INTEGER NOT NULL DEFAULT 0,
            transaction_id INTEGER,
            FOREIGN KEY (plan_id) REFERENCES pp_investment_plan(id),
            FOREIGN KEY (transaction_id) REFERENCES pp_txn(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all investment plans
#[command]
pub fn get_investment_plans() -> Result<Vec<InvestmentPlanData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    let mut stmt = conn.prepare(
        "SELECT
            p.id, p.name, p.security_id, s.name, p.account_id, a.name,
            p.portfolio_id, pf.name, p.interval, p.amount, a.currency,
            p.day_of_month, p.start_date, p.end_date, p.is_active,
            (SELECT MAX(date) FROM pp_plan_execution WHERE plan_id = p.id) as last_exec,
            (SELECT COALESCE(SUM(amount), 0) FROM pp_plan_execution WHERE plan_id = p.id) as total,
            (SELECT COUNT(*) FROM pp_plan_execution WHERE plan_id = p.id) as exec_count
         FROM pp_investment_plan p
         JOIN pp_security s ON s.id = p.security_id
         JOIN pp_account a ON a.id = p.account_id
         JOIN pp_portfolio pf ON pf.id = p.portfolio_id
         ORDER BY p.name"
    ).map_err(|e| e.to_string())?;

    let plans = stmt.query_map([], |row| {
        let start_date: String = row.get(12)?;
        let end_date: Option<String> = row.get(13)?;
        let is_active: bool = row.get::<_, i32>(14)? != 0;
        let last_execution: Option<String> = row.get(15)?;
        let interval: String = row.get(8)?;
        let day_of_month: i32 = row.get(11)?;

        // Calculate next execution
        let next_execution = if is_active {
            calculate_next_execution(&interval, day_of_month, last_execution.as_deref(), end_date.as_deref())
        } else {
            None
        };

        Ok(InvestmentPlanData {
            id: row.get(0)?,
            name: row.get(1)?,
            security_id: row.get(2)?,
            security_name: row.get(3)?,
            account_id: row.get(4)?,
            account_name: row.get(5)?,
            portfolio_id: row.get(6)?,
            portfolio_name: row.get(7)?,
            interval,
            amount: row.get(9)?,
            currency: row.get(10)?,
            day_of_month,
            start_date,
            end_date,
            is_active,
            last_execution,
            next_execution,
            total_invested: row.get(16)?,
            execution_count: row.get(17)?,
        })
    }).map_err(|e| e.to_string())?;

    plans.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get a single investment plan
#[command]
pub fn get_investment_plan(id: i64) -> Result<InvestmentPlanData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    conn.query_row(
        "SELECT
            p.id, p.name, p.security_id, s.name, p.account_id, a.name,
            p.portfolio_id, pf.name, p.interval, p.amount, a.currency,
            p.day_of_month, p.start_date, p.end_date, p.is_active,
            (SELECT MAX(date) FROM pp_plan_execution WHERE plan_id = p.id) as last_exec,
            (SELECT COALESCE(SUM(amount), 0) FROM pp_plan_execution WHERE plan_id = p.id) as total,
            (SELECT COUNT(*) FROM pp_plan_execution WHERE plan_id = p.id) as exec_count
         FROM pp_investment_plan p
         JOIN pp_security s ON s.id = p.security_id
         JOIN pp_account a ON a.id = p.account_id
         JOIN pp_portfolio pf ON pf.id = p.portfolio_id
         WHERE p.id = ?1",
        [id],
        |row| {
            let start_date: String = row.get(12)?;
            let end_date: Option<String> = row.get(13)?;
            let is_active: bool = row.get::<_, i32>(14)? != 0;
            let last_execution: Option<String> = row.get(15)?;
            let interval: String = row.get(8)?;
            let day_of_month: i32 = row.get(11)?;

            let next_execution = if is_active {
                calculate_next_execution(&interval, day_of_month, last_execution.as_deref(), end_date.as_deref())
            } else {
                None
            };

            Ok(InvestmentPlanData {
                id: row.get(0)?,
                name: row.get(1)?,
                security_id: row.get(2)?,
                security_name: row.get(3)?,
                account_id: row.get(4)?,
                account_name: row.get(5)?,
                portfolio_id: row.get(6)?,
                portfolio_name: row.get(7)?,
                interval,
                amount: row.get(9)?,
                currency: row.get(10)?,
                day_of_month,
                start_date,
                end_date,
                is_active,
                last_execution,
                next_execution,
                total_invested: row.get(16)?,
                execution_count: row.get(17)?,
            })
        }
    ).map_err(|e| e.to_string())
}

/// Create a new investment plan
#[command]
pub fn create_investment_plan(data: CreateInvestmentPlanRequest) -> Result<InvestmentPlanData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    // Get import_id from security
    let import_id: i64 = conn.query_row(
        "SELECT import_id FROM pp_security WHERE id = ?1",
        [data.security_id],
        |row| row.get(0)
    ).map_err(|e| format!("Security not found: {}", e))?;

    conn.execute(
        "INSERT INTO pp_investment_plan (import_id, name, security_id, account_id, portfolio_id, interval, amount, day_of_month, start_date, end_date, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1)",
        rusqlite::params![
            import_id,
            data.name,
            data.security_id,
            data.account_id,
            data.portfolio_id,
            data.interval,
            data.amount,
            data.day_of_month,
            data.start_date,
            data.end_date,
        ],
    ).map_err(|e| e.to_string())?;

    let plan_id = conn.last_insert_rowid();
    get_investment_plan(plan_id)
}

/// Update an investment plan
#[command]
pub fn update_investment_plan(
    id: i64,
    name: Option<String>,
    amount: Option<i64>,
    day_of_month: Option<i32>,
    end_date: Option<String>,
    is_active: Option<bool>,
) -> Result<InvestmentPlanData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(n) = name {
        updates.push("name = ?");
        params.push(Box::new(n));
    }
    if let Some(a) = amount {
        updates.push("amount = ?");
        params.push(Box::new(a));
    }
    if let Some(d) = day_of_month {
        updates.push("day_of_month = ?");
        params.push(Box::new(d));
    }
    if let Some(e) = end_date {
        updates.push("end_date = ?");
        params.push(Box::new(e));
    }
    if let Some(a) = is_active {
        updates.push("is_active = ?");
        params.push(Box::new(if a { 1i32 } else { 0i32 }));
    }

    if updates.is_empty() {
        return get_investment_plan(id);
    }

    params.push(Box::new(id));
    let sql = format!(
        "UPDATE pp_investment_plan SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice()).map_err(|e| e.to_string())?;

    get_investment_plan(id)
}

/// Delete an investment plan
#[command]
pub fn delete_investment_plan(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Delete executions first
    conn.execute("DELETE FROM pp_plan_execution WHERE plan_id = ?1", [id])
        .map_err(|e| e.to_string())?;

    // Delete plan
    conn.execute("DELETE FROM pp_investment_plan WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get executions for a plan
#[command]
pub fn get_investment_plan_executions(plan_id: i64) -> Result<Vec<InvestmentPlanExecution>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    let mut stmt = conn.prepare(
        "SELECT id, plan_id, date, shares, price, amount, fees, transaction_id
         FROM pp_plan_execution
         WHERE plan_id = ?1
         ORDER BY date DESC"
    ).map_err(|e| e.to_string())?;

    let executions = stmt.query_map([plan_id], |row| {
        Ok(InvestmentPlanExecution {
            id: row.get(0)?,
            plan_id: row.get(1)?,
            date: row.get(2)?,
            shares: row.get(3)?,
            price: row.get(4)?,
            amount: row.get(5)?,
            fees: row.get(6)?,
            transaction_id: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    executions.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Execute an investment plan manually
#[command]
pub fn execute_investment_plan(
    plan_id: i64,
    date: String,
    price: Option<i64>,
) -> Result<InvestmentPlanExecution, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get plan details
    let plan = get_investment_plan(plan_id)?;

    // Get price (from parameter or latest price)
    let exec_price = match price {
        Some(p) => p,
        None => {
            conn.query_row(
                "SELECT value FROM pp_latest_price WHERE security_id = ?1",
                [plan.security_id],
                |row| row.get::<_, i64>(0)
            ).map_err(|_| "No price available for security".to_string())?
        }
    };

    // Calculate shares: amount / price (both scaled)
    // amount is in cents (×10²), price is ×10⁸
    // shares should be ×10⁸
    let shares = if exec_price > 0 {
        (plan.amount as i128 * 100_000_000 / (exec_price as i128 / 100)) as i64
    } else {
        return Err("Price cannot be zero".to_string());
    };

    // Get import_id
    let import_id: i64 = conn.query_row(
        "SELECT import_id FROM pp_investment_plan WHERE id = ?1",
        [plan_id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Create portfolio transaction
    let txn_uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
         VALUES (?1, ?2, 'portfolio', ?3, ?4, 'BUY', ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            import_id,
            txn_uuid,
            plan.portfolio_id,
            plan.security_id,
            date,
            plan.amount,
            plan.currency,
            shares,
            format!("Sparplan: {}", plan.name),
        ],
    ).map_err(|e| e.to_string())?;

    let portfolio_txn_id = conn.last_insert_rowid();

    // Create account transaction
    let account_uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
         VALUES (?1, ?2, 'account', ?3, ?4, 'BUY', ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            import_id,
            account_uuid,
            plan.account_id,
            plan.security_id,
            date,
            plan.amount,
            plan.currency,
            shares,
            format!("Sparplan: {}", plan.name),
        ],
    ).map_err(|e| e.to_string())?;

    let account_txn_id = conn.last_insert_rowid();

    // Create cross entry
    conn.execute(
        "INSERT INTO pp_cross_entry (entry_type, portfolio_txn_id, account_txn_id)
         VALUES ('BUY_SELL', ?1, ?2)",
        [portfolio_txn_id, account_txn_id],
    ).map_err(|e| e.to_string())?;

    // Record execution
    conn.execute(
        "INSERT INTO pp_plan_execution (plan_id, date, shares, price, amount, fees, transaction_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        rusqlite::params![plan_id, date, shares, exec_price, plan.amount, portfolio_txn_id],
    ).map_err(|e| e.to_string())?;

    let exec_id = conn.last_insert_rowid();

    Ok(InvestmentPlanExecution {
        id: exec_id,
        plan_id,
        date,
        shares,
        price: exec_price,
        amount: plan.amount,
        fees: 0,
        transaction_id: portfolio_txn_id,
    })
}

/// Get plans due for execution on a date
#[command]
pub fn get_plans_due_for_execution(date: String) -> Result<Vec<InvestmentPlanData>, String> {
    let plans = get_investment_plans()?;

    let target_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    Ok(plans
        .into_iter()
        .filter(|p| {
            if !p.is_active {
                return false;
            }

            if let Some(next) = &p.next_execution {
                if let Ok(next_date) = NaiveDate::parse_from_str(next, "%Y-%m-%d") {
                    return next_date <= target_date;
                }
            }
            false
        })
        .collect())
}

/// Calculate next execution date
fn calculate_next_execution(
    interval: &str,
    day_of_month: i32,
    last_execution: Option<&str>,
    end_date: Option<&str>,
) -> Option<String> {
    let today = chrono::Local::now().date_naive();

    let base_date = match last_execution {
        Some(last) => NaiveDate::parse_from_str(last, "%Y-%m-%d").ok()?,
        None => today,
    };

    let next = match interval {
        "WEEKLY" => base_date + chrono::Duration::days(7),
        "BIWEEKLY" => base_date + chrono::Duration::days(14),
        "MONTHLY" => {
            let mut year = base_date.year();
            let mut month = base_date.month() + 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
            let day = day_of_month.min(days_in_month(year, month) as i32) as u32;
            NaiveDate::from_ymd_opt(year, month, day)?
        }
        "QUARTERLY" => {
            let mut year = base_date.year();
            let mut month = base_date.month() + 3;
            if month > 12 {
                month -= 12;
                year += 1;
            }
            let day = day_of_month.min(days_in_month(year, month) as i32) as u32;
            NaiveDate::from_ymd_opt(year, month, day)?
        }
        "YEARLY" => {
            let year = base_date.year() + 1;
            let month = base_date.month();
            let day = day_of_month.min(days_in_month(year, month) as i32) as u32;
            NaiveDate::from_ymd_opt(year, month, day)?
        }
        _ => return None,
    };

    // Check end date
    if let Some(end) = end_date {
        if let Ok(end_d) = NaiveDate::parse_from_str(end, "%Y-%m-%d") {
            if next > end_d {
                return None;
            }
        }
    }

    Some(next.format("%Y-%m-%d").to_string())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
