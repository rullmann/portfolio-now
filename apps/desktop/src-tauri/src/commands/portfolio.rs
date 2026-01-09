use crate::db;
use crate::models::Portfolio;
use chrono::Utc;
use tauri::command;

#[command]
pub fn get_portfolios() -> Result<Vec<Portfolio>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare("SELECT id, name, base_currency, created_at, updated_at, note FROM portfolios")
        .map_err(|e| e.to_string())?;

    let portfolios = stmt
        .query_map([], |row| {
            Ok(Portfolio {
                id: row.get(0)?,
                name: row.get(1)?,
                base_currency: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                note: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(portfolios)
}

#[command]
pub fn create_portfolio(name: String, base_currency: String) -> Result<Portfolio, String> {
    let portfolio = Portfolio::new(name, base_currency);

    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute(
        "INSERT INTO portfolios (id, name, base_currency, created_at, updated_at, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            &portfolio.id,
            &portfolio.name,
            &portfolio.base_currency,
            &portfolio.created_at,
            &portfolio.updated_at,
            &portfolio.note,
        ),
    )
    .map_err(|e| e.to_string())?;

    Ok(portfolio)
}

#[command]
pub fn update_portfolio(
    id: String,
    name: String,
    base_currency: String,
    note: Option<String>,
) -> Result<Portfolio, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let updated_at = Utc::now();

    conn.execute(
        "UPDATE portfolios SET name = ?1, base_currency = ?2, note = ?3, updated_at = ?4 WHERE id = ?5",
        (&name, &base_currency, &note, &updated_at, &id),
    )
    .map_err(|e| e.to_string())?;

    // Fetch the updated portfolio
    let mut stmt = conn
        .prepare("SELECT id, name, base_currency, created_at, updated_at, note FROM portfolios WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let portfolio = stmt
        .query_row([&id], |row| {
            Ok(Portfolio {
                id: row.get(0)?,
                name: row.get(1)?,
                base_currency: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                note: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(portfolio)
}

#[command]
pub fn delete_portfolio(id: String) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute("DELETE FROM portfolios WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}
