use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::Mutex;
use tauri::command;

use crate::security;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStartResult {
    pub auth_url: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterUser {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaUploadResponse {
    pub media_id_string: String,
}

/// v2 media upload response: { data: { id, media_key, ... } }
#[derive(Debug, Deserialize)]
struct MediaUploadV2Data {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MediaUploadV2Response {
    data: MediaUploadV2Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TweetResult {
    pub tweet_id: String,
    pub tweet_url: String,
}

// ============================================================================
// OAuth State (shared between start_auth and await_callback)
// ============================================================================

struct OAuthState {
    code_verifier: String,
    state: String,
    port: u16,
    listener: Option<TcpListener>,
}

// Manual Debug impl because TcpListener doesn't implement Debug nicely
impl std::fmt::Debug for OAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthState")
            .field("code_verifier", &self.code_verifier)
            .field("state", &self.state)
            .field("port", &self.port)
            .field("listener", &self.listener.as_ref().map(|_| "TcpListener(...)"))
            .finish()
    }
}

static OAUTH_STATE: Mutex<Option<OAuthState>> = Mutex::new(None);

// ============================================================================
// Helper: Generate PKCE code verifier (43-128 chars, unreserved URL chars)
// ============================================================================

fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    (0..128)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    BASE64
        .encode(hash)
        .replace('+', "-")
        .replace('/', "_")
        .replace('=', "")
}

fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

// ============================================================================
// Validation
// ============================================================================

/// Validate that a Twitter/X Client ID is well-formed (non-empty, max 256 chars).
/// X OAuth 2.0 Client IDs can be alphanumeric and may contain special characters.
fn validate_twitter_client_id(client_id: &str) -> Result<(), String> {
    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        return Err("Twitter Client ID darf nicht leer sein.".to_string());
    }
    if trimmed.len() > 256 {
        return Err(format!(
            "Twitter Client ID darf maximal 256 Zeichen lang sein (aktuell: {}).",
            trimmed.len()
        ));
    }
    Ok(())
}

// ============================================================================
// Commands
// ============================================================================

/// Fixed port for OAuth callback — must match redirect URI registered in X Developer Console
const OAUTH_CALLBACK_PORT: u16 = 17548;

/// Start OAuth 2.0 PKCE flow: generate verifier/challenge, bind local server, return auth URL
#[command]
pub async fn twitter_start_auth(client_id: String) -> Result<AuthStartResult, String> {
    let client_id = client_id.trim().to_string();
    validate_twitter_client_id(&client_id)?;

    let port = OAUTH_CALLBACK_PORT;

    // Bind listener and keep it alive until callback is received (prevents race condition)
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|e| format!("OAuth callback port {} is already in use: {}", port, e))?;

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();

    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "https://twitter.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("tweet.read tweet.write users.read offline.access"),
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge),
    );

    // Store state + listener for callback
    let mut oauth_state = OAUTH_STATE.lock().map_err(|e| format!("Lock error: {}", e))?;
    *oauth_state = Some(OAuthState {
        code_verifier,
        state: state.clone(),
        port,
        listener: Some(listener),
    });

    log::info!("Twitter OAuth started on port {}", port);

    Ok(AuthStartResult { auth_url, port })
}

/// Wait for the OAuth callback on the local server (120s timeout)
#[command]
pub async fn twitter_await_callback() -> Result<CallbackResult, String> {
    let (listener, expected_state) = {
        let mut oauth_state = OAUTH_STATE.lock().map_err(|e| format!("Lock error: {}", e))?;
        let state = oauth_state
            .as_mut()
            .ok_or("No OAuth flow in progress. Call twitter_start_auth first.")?;
        let listener = state.listener.take()
            .ok_or("OAuth listener already consumed. Call twitter_start_auth again.")?;
        (listener, state.state.clone())
    };

    listener
        .set_nonblocking(false)
        .map_err(|e| format!("Failed to set blocking: {}", e))?;

    // Set a timeout using tokio
    let result = tokio::time::timeout(std::time::Duration::from_secs(120), tokio::task::spawn_blocking(move || {
        let (mut stream, _) = listener.accept().map_err(|e| format!("Accept failed: {}", e))?;

        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).map_err(|e| format!("Read failed: {}", e))?;

        // Parse GET /callback?code=xxx&state=yyy HTTP/1.1
        let path = request_line
            .split_whitespace()
            .nth(1)
            .ok_or("Invalid HTTP request")?
            .to_string();

        // Extract query params
        let query = path
            .split('?')
            .nth(1)
            .ok_or("No query parameters in callback")?;

        let mut code = None;
        let mut state = None;

        for param in query.split('&') {
            let mut parts = param.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            match key {
                "code" => code = Some(urlencoding::decode(value).unwrap_or_default().to_string()),
                "state" => state = Some(urlencoding::decode(value).unwrap_or_default().to_string()),
                _ => {}
            }
        }

        let code = code.ok_or("Missing 'code' parameter in callback")?;
        let state = state.ok_or("Missing 'state' parameter in callback")?;

        // Send success HTML response
        let html = r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Portfolio Now</title><style>body{font-family:-apple-system,BlinkMacSystemFont,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#0a0a0a;color:#fff}div{text-align:center;padding:2rem}h1{color:#1d9bf0;margin-bottom:1rem}p{color:#71767b}</style></head><body><div><h1>Verbunden!</h1><p>Du kannst dieses Fenster jetzt schließen und zu Portfolio Now zurückkehren.</p></div></body></html>"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(),
            html
        );

        use std::io::Write as _;
        stream.write_all(response.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;
        stream.flush().ok();

        Ok::<(String, String), String>((code, state))
    }))
    .await
    .map_err(|_| "OAuth callback timeout (120s). Please try again.".to_string())?
    .map_err(|e| format!("Callback task failed: {}", e))?;

    let (code, state) = result?;

    // Verify state matches
    if state != expected_state {
        return Err("OAuth state mismatch. Possible CSRF attack.".to_string());
    }

    log::info!("Twitter OAuth callback received successfully");

    Ok(CallbackResult { code, state })
}

/// Exchange authorization code for access/refresh tokens
#[command]
pub async fn twitter_exchange_token(
    client_id: String,
    code: String,
) -> Result<TokenResponse, String> {
    let (code_verifier, port) = {
        let mut oauth_state = OAUTH_STATE.lock().map_err(|e| format!("Lock error: {}", e))?;
        let state = oauth_state
            .take()
            .ok_or("No OAuth flow in progress")?;
        (state.code_verifier, state.port)
    };

    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.x.com/2/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
            urlencoding::encode(&code),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&client_id),
            urlencoding::encode(&code_verifier),
        ))
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("Twitter token exchange failed: {} - {}", status, body);
        return Err(format!("Token exchange failed ({}): {}", status, body));
    }

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    log::info!("Twitter token exchange successful");
    Ok(token)
}

/// Refresh access token using refresh token
#[command]
pub async fn twitter_refresh_token(
    client_id: String,
    refresh_token: String,
) -> Result<TokenResponse, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.x.com/2/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&refresh_token={}&client_id={}",
            urlencoding::encode(&refresh_token),
            urlencoding::encode(&client_id),
        ))
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("Twitter token refresh failed: {} - {}", status, body);
        return Err(format!("Token refresh failed ({}): {}", status, body));
    }

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    log::info!("Twitter token refresh successful");
    Ok(token)
}

/// Get authenticated user info
#[command]
pub async fn twitter_get_user_info(access_token: String) -> Result<TwitterUser, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.x.com/2/users/me")
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("User info request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Get user info failed ({}): {}", status, body));
    }

    #[derive(Deserialize)]
    struct UserResponse {
        data: TwitterUser,
    }

    let user_response: UserResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user response: {}", e))?;

    log::info!("Twitter user info: @{}", user_response.data.username);
    Ok(user_response.data)
}

/// Upload media (image) to Twitter
#[command]
pub async fn twitter_upload_media(
    access_token: String,
    image_base64: String,
) -> Result<MediaUploadResponse, String> {
    security::check_rate_limit("twitter_upload", &security::limits::twitter_upload())
        .map_err(|e| e.to_string())?;

    let image_bytes = BASE64
        .decode(&image_base64)
        .map_err(|e| format!("Invalid base64 image: {}", e))?;

    // X/Twitter media upload limit is 5MB
    if image_bytes.len() > 5 * 1024 * 1024 {
        return Err("Image too large. Maximum size is 5MB.".to_string());
    }

    let part = multipart::Part::bytes(image_bytes)
        .file_name("chart.jpg")
        .mime_str("image/jpeg")
        .map_err(|e| format!("Failed to create multipart: {}", e))?;

    let form = multipart::Form::new()
        .text("media_category", "tweet_image")
        .part("media", part);

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.x.com/2/media/upload")
        .bearer_auth(&access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Media upload request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("X media upload failed: {} - {}", status, body);
        return Err(format!("Media upload failed ({}): {}", status, body));
    }

    let v2_response: MediaUploadV2Response = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;

    let media_id = v2_response.data.id;
    log::info!("X media uploaded: {}", media_id);

    Ok(MediaUploadResponse {
        media_id_string: media_id,
    })
}

/// Post a tweet (with optional media and reply)
#[command]
pub async fn twitter_post_tweet(
    access_token: String,
    text: String,
    media_ids: Option<Vec<String>>,
    reply_to_tweet_id: Option<String>,
) -> Result<TweetResult, String> {
    security::check_rate_limit("twitter_post", &security::limits::twitter_post())
        .map_err(|e| e.to_string())?;

    if text.is_empty() {
        return Err("Tweet text cannot be empty".to_string());
    }

    // Build tweet payload
    let mut payload = serde_json::json!({
        "text": text
    });

    if let Some(ids) = media_ids {
        if !ids.is_empty() {
            payload["media"] = serde_json::json!({
                "media_ids": ids
            });
        }
    }

    if let Some(reply_id) = reply_to_tweet_id {
        payload["reply"] = serde_json::json!({
            "in_reply_to_tweet_id": reply_id
        });
    }

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.x.com/2/tweets")
        .bearer_auth(&access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Tweet post request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("Twitter post failed: {} - {}", status, body);
        return Err(format!("Tweet post failed ({}): {}", status, body));
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct TweetData {
        id: String,
        text: String,
    }

    #[derive(Deserialize)]
    struct TweetApiResponse {
        data: TweetData,
    }

    let tweet_response: TweetApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse tweet response: {}", e))?;

    let tweet_url = format!(
        "https://x.com/i/web/status/{}",
        tweet_response.data.id
    );

    log::info!("Tweet posted: {}", tweet_url);

    Ok(TweetResult {
        tweet_id: tweet_response.data.id,
        tweet_url,
    })
}
