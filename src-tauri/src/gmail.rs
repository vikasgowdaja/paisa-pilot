use base64::Engine;
use chrono::{DateTime, NaiveDate, Utc};
#[cfg(windows)]
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use regex::Regex;
use reqwest::blocking::{Client, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::OnceLock;
#[cfg(windows)]
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
#[cfg(windows)]
use url::Url;

#[cfg(windows)]
const GMAIL_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
#[cfg(windows)]
const CREDENTIAL_SERVICE: &str = "PaisaPilot Gmail";
#[cfg(windows)]
const CREDENTIAL_USER: &str = "oauth-tokens";
#[cfg(windows)]
const CLIENT_ID: Option<&str> = option_env!("PAISAPILOT_GOOGLE_CLIENT_ID");

#[cfg(windows)]
#[derive(Debug, Deserialize, Serialize)]
struct OAuthTokens {
    access_token: String,
    refresh_token: String,
    expires_at_ms: i64,
    email: String,
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub connected: bool,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct GmailMessageList {
    #[serde(default)]
    messages: Vec<GmailMessageId>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailMessageId {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GmailMessageResponse {
    #[serde(rename = "internalDate")]
    internal_date: String,
    payload: Option<GmailPayload>,
}

#[derive(Debug, Deserialize)]
struct GmailPayload {
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    headers: Option<Vec<GmailHeader>>,
    body: Option<GmailBody>,
    parts: Option<Vec<GmailPayload>>,
}

#[derive(Debug, Deserialize)]
struct GmailHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct GmailBody {
    data: Option<String>,
}

#[derive(Debug)]
pub struct GmailMessage {
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub received_at_ms: i64,
    pub extracted: ExtractedTransaction,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExtractedTransaction {
    pub date: String,
    pub amount: Option<String>,
    pub currency: String,
    pub event_type: String,
    pub kind: String,
    pub merchant: String,
    pub account_hint: String,
    pub reference: String,
    pub description: String,
}

fn gmail_http_client() -> Result<Client, String> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Could not prepare the Gmail connection: {e}"))
}

#[cfg(windows)]
fn client_id(id: &str) -> Result<ClientId, String> {
    let id = id.trim();
    if id.is_empty() || id.len() > 300 || !id.ends_with(".apps.googleusercontent.com") {
        return Err("The Google OAuth client ID configured for this build is invalid.".into());
    }
    Ok(ClientId::new(id.to_string()))
}

#[cfg(windows)]
pub fn configured_client_id() -> Result<&'static str, String> {
    CLIENT_ID
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            "Google sign-in is not configured in this PaisaPilot build. The app publisher must configure its OAuth client ID.".into()
        })
}

#[cfg(windows)]
fn credential_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_USER)
        .map_err(|e| format!("Could not access Windows Credential Manager: {e}"))
}

#[cfg(windows)]
fn store_tokens(tokens: &OAuthTokens) -> Result<(), String> {
    let json =
        serde_json::to_string(tokens).map_err(|e| format!("Could not encode OAuth tokens: {e}"))?;
    credential_entry()?.set_password(&json).map_err(|e| {
        format!("Could not securely save the Gmail token in Windows Credential Manager: {e}")
    })
}

#[cfg(windows)]
fn load_tokens() -> Result<Option<OAuthTokens>, String> {
    let entry = credential_entry()?;
    let secret = match entry.get_password() {
        Ok(secret) => secret,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Could not read the Gmail token from Windows Credential Manager: {e}"
            ))
        }
    };
    let tokens = serde_json::from_str(&secret)
        .map_err(|e| format!("Saved Gmail token data is invalid; disconnect and reconnect: {e}"))?;
    Ok(Some(tokens))
}

#[cfg(windows)]
pub fn auth_status() -> Result<AuthStatus, String> {
    Ok(match load_tokens()? {
        Some(tokens) => AuthStatus {
            connected: true,
            email: tokens.email,
        },
        None => AuthStatus {
            connected: false,
            email: String::new(),
        },
    })
}

#[cfg(windows)]
pub fn disconnect() -> Result<(), String> {
    match credential_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!(
            "Could not remove the Gmail token from Windows Credential Manager: {e}"
        )),
    }
}

#[cfg(windows)]
fn oauth_client(id: &str) -> Result<BasicClient, String> {
    Ok(BasicClient::new(
        client_id(id)?,
        None,
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into())
            .map_err(|e| format!("Invalid Google authorization endpoint: {e}"))?,
        Some(
            TokenUrl::new("https://oauth2.googleapis.com/token".into())
                .map_err(|e| format!("Invalid Google token endpoint: {e}"))?,
        ),
    ))
}

#[cfg(windows)]
fn callback(listener: &TcpListener, expected_state: &str) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Could not prepare the local OAuth callback: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| format!("Could not read the OAuth callback: {e}"))?;
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .map_err(|e| format!("Could not read the OAuth callback: {e}"))?;
                let target = line
                    .split_whitespace()
                    .nth(1)
                    .ok_or("Google returned an invalid OAuth callback.")?;
                let callback_url = Url::parse(&format!("http://127.0.0.1{target}"))
                    .map_err(|e| format!("Google returned an invalid OAuth callback: {e}"))?;
                let state = callback_url
                    .query_pairs()
                    .find(|(key, _)| key == "state")
                    .map(|(_, value)| value.into_owned())
                    .unwrap_or_default();
                let error = callback_url
                    .query_pairs()
                    .find(|(key, _)| key == "error")
                    .map(|(_, value)| value.into_owned());
                let code = callback_url
                    .query_pairs()
                    .find(|(key, _)| key == "code")
                    .map(|(_, value)| value.into_owned());
                let (status, body, result) = if let Some(error) = error {
                    (
                        "400 Bad Request",
                        "Google access was not granted. You can close this window.",
                        Err(format!("Google authorization failed: {error}")),
                    )
                } else if state != expected_state {
                    (
                        "400 Bad Request",
                        "The authorization response could not be verified.",
                        Err("Google OAuth state did not match. Retry the connection.".to_string()),
                    )
                } else if let Some(code) = code {
                    (
                        "200 OK",
                        "PaisaPilot is connected to Gmail. You can close this window.",
                        Ok(code),
                    )
                } else {
                    (
                        "400 Bad Request",
                        "The authorization response did not contain a code.",
                        Err("Google did not return an authorization code.".to_string()),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Could not finish the OAuth callback: {e}"))?;
                return result;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(
                        "Google authorization timed out. Start the connection again.".into(),
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                return Err(format!(
                    "Could not receive the Google OAuth callback: {error}"
                ))
            }
        }
    }
}

#[cfg(windows)]
pub fn connect(id: &str) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Could not start the local OAuth callback: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Could not start the local OAuth callback: {e}"))?
        .port();
    let redirect = RedirectUrl::new(format!("http://127.0.0.1:{port}/"))
        .map_err(|e| format!("Could not prepare the OAuth redirect: {e}"))?;
    let client = oauth_client(id)?.set_redirect_uri(redirect);
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(GMAIL_READONLY_SCOPE.to_string()))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(challenge)
        .url();
    webbrowser::open(auth_url.as_str())
        .map_err(|e| format!("Could not open the system browser for Google sign-in: {e}"))?;
    let code = callback(&listener, csrf.secret())?;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(verifier)
        .request(oauth2::ureq::http_client)
        .map_err(|e| format!("Google token exchange failed: {e}"))?;
    let access_token = token.access_token().secret().to_string();
    let refresh_token = token.refresh_token().map(|value| value.secret().to_string())
        .ok_or("Google did not return a refresh token. Revoke PaisaPilot access in Google Account settings and try again.")?;
    let email = profile_email(&access_token)?;
    let expires_in = token
        .expires_in()
        .unwrap_or(Duration::from_secs(3600))
        .as_millis() as i64;
    store_tokens(&OAuthTokens {
        access_token,
        refresh_token,
        expires_at_ms: Utc::now().timestamp_millis().saturating_add(expires_in),
        email: email.clone(),
    })?;
    Ok(email)
}

#[cfg(windows)]
fn valid_access_token(id: &str) -> Result<String, String> {
    let mut tokens = load_tokens()?.ok_or("Connect a Gmail account in Settings first.")?;
    if tokens.expires_at_ms <= Utc::now().timestamp_millis().saturating_add(60_000) {
        let client = oauth_client(id)?;
        let refresh = oauth2::RefreshToken::new(tokens.refresh_token.clone());
        let response = client
            .exchange_refresh_token(&refresh)
            .request(oauth2::ureq::http_client)
            .map_err(|e| {
                format!("Could not refresh the Gmail authorization. Reconnect in Settings: {e}")
            })?;
        tokens.access_token = response.access_token().secret().to_string();
        if let Some(refresh) = response.refresh_token() {
            tokens.refresh_token = refresh.secret().to_string();
        }
        let expires_in = response
            .expires_in()
            .unwrap_or(Duration::from_secs(3600))
            .as_millis() as i64;
        tokens.expires_at_ms = Utc::now().timestamp_millis().saturating_add(expires_in);
        store_tokens(&tokens)?;
    }
    Ok(tokens.access_token)
}

fn json_response<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    response
        .error_for_status()
        .map_err(|e| format!("Gmail API request failed: {e}"))?
        .json::<T>()
        .map_err(|e| format!("Gmail returned an invalid response: {e}"))
}

pub fn profile_email(access_token: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Profile {
        email_address: String,
    }
    let response = gmail_http_client()?
        .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
        .bearer_auth(access_token)
        .send()
        .map_err(|e| format!("Could not verify the Gmail account: {e}"))?;
    let profile: Profile = json_response(response)?;
    if profile.email_address.trim().is_empty() {
        return Err("Google did not return the Gmail account address.".into());
    }
    Ok(profile.email_address)
}

pub fn search_after_ms(last_sync_ms: Option<i64>) -> String {
    let since_ms = last_sync_ms
        .map(|value| value.saturating_sub(2 * 24 * 60 * 60 * 1000))
        .unwrap_or_else(|| (Utc::now() - chrono::Duration::days(365)).timestamp_millis());
    let date = DateTime::<Utc>::from_timestamp_millis(since_ms)
        .unwrap_or_else(Utc::now)
        .format("%Y/%m/%d");
    format!(
        "after:{date} {{transaction payment debited credited UPI UTR INR Rs receipt refund bill}}"
    )
}

#[cfg(windows)]
pub fn list_message_ids(
    id: &str,
    query: &str,
    page_token: Option<&str>,
) -> Result<(Vec<String>, Option<String>), String> {
    let access_token = valid_access_token(id)?;
    list_message_ids_with_token(&access_token, query, page_token)
}

pub fn list_message_ids_with_token(
    access_token: &str,
    query: &str,
    page_token: Option<&str>,
) -> Result<(Vec<String>, Option<String>), String> {
    let mut request = gmail_http_client()?
        .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
        .bearer_auth(access_token)
        .query(&[("q", query), ("maxResults", "100")]);
    if let Some(page_token) = page_token {
        request = request.query(&[("pageToken", page_token)]);
    }
    let page: GmailMessageList = json_response(
        request
            .send()
            .map_err(|e| format!("Could not search Gmail: {e}"))?,
    )?;
    Ok((
        page.messages
            .into_iter()
            .map(|message| message.id)
            .collect(),
        page.next_page_token,
    ))
}

fn decode_body(data: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(data))
        .map_err(|e| format!("Gmail returned an invalid message body: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn collect_text(
    payload: &GmailPayload,
    plain: &mut Vec<String>,
    html: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(data) = payload.body.as_ref().and_then(|body| body.data.as_deref()) {
        let text = decode_body(data)?;
        match payload.mime_type.as_deref() {
            Some("text/plain") => plain.push(text),
            Some("text/html") => html.push(text),
            _ => {}
        }
    }
    if let Some(parts) = &payload.parts {
        for part in parts {
            collect_text(part, plain, html)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn fetch_message(id: &str, message_id: &str) -> Result<Option<GmailMessage>, String> {
    let access_token = valid_access_token(id)?;
    fetch_message_with_token(&access_token, message_id)
}

pub fn fetch_message_with_token(
    access_token: &str,
    message_id: &str,
) -> Result<Option<GmailMessage>, String> {
    let response = gmail_http_client()?
        .get(format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
            url::form_urlencoded::byte_serialize(message_id.as_bytes()).collect::<String>()
        ))
        .bearer_auth(access_token)
        .query(&[("format", "full")])
        .send()
        .map_err(|e| format!("Could not fetch a Gmail message: {e}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let message: GmailMessageResponse = json_response(response)?;
    let payload = message
        .payload
        .ok_or("Gmail returned a message without its content.")?;
    let header = |name: &str| {
        payload
            .headers
            .as_ref()
            .and_then(|headers| {
                headers
                    .iter()
                    .find(|header| header.name.eq_ignore_ascii_case(name))
            })
            .map(|header| header.value.clone())
            .unwrap_or_default()
    };
    let subject = header("Subject");
    let sender = header("From");
    let received_at_ms = message
        .internal_date
        .parse::<i64>()
        .map_err(|e| format!("Gmail returned an invalid message date: {e}"))?;
    let mut plain = Vec::new();
    let mut html = Vec::new();
    collect_text(&payload, &mut plain, &mut html)?;
    let body = if plain.is_empty() {
        static HTML_TAGS: OnceLock<Regex> = OnceLock::new();
        html.iter()
            .map(|part| regex(&HTML_TAGS, r"(?s)<[^>]*>").replace_all(part, " "))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        plain.join("\n")
    };
    Ok(Some(GmailMessage {
        message_id: message_id.to_string(),
        extracted: extract_transaction(&subject, &sender, &body, received_at_ms),
        subject,
        sender,
        received_at_ms,
    }))
}
fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("Gmail parser regex is valid"))
}

fn clean(value: &str) -> String {
    value
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '-' | '.' | ',' | ';'))
        .chars()
        .take(100)
        .collect::<String>()
        .trim()
        .to_string()
}

fn transaction_date(body: &str, received_at_ms: i64) -> String {
    static ISO: OnceLock<Regex> = OnceLock::new();
    static LOCAL: OnceLock<Regex> = OnceLock::new();
    let formats = [
        (
            regex(
                &ISO,
                r"\b(20\d{2})[-/.](0?[1-9]|1[0-2])[-/.](0?[1-9]|[12]\d|3[01])\b",
            ),
            true,
        ),
        (
            regex(
                &LOCAL,
                r"\b(0?[1-9]|[12]\d|3[01])[-/.](0?[1-9]|1[0-2])[-/.](20\d{2})\b",
            ),
            false,
        ),
    ];
    for (pattern, iso) in formats {
        if let Some(captures) = pattern.captures(body) {
            let date = if iso {
                format!("{}-{:0>2}-{:0>2}", &captures[1], &captures[2], &captures[3])
            } else {
                format!("{}-{:0>2}-{:0>2}", &captures[3], &captures[2], &captures[1])
            };
            if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_ok() {
                return date;
            }
        }
    }
    DateTime::<Utc>::from_timestamp_millis(received_at_ms)
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string())
}

pub fn extract_transaction(
    subject: &str,
    sender: &str,
    body: &str,
    received_at_ms: i64,
) -> ExtractedTransaction {
    static AMOUNT: OnceLock<Regex> = OnceLock::new();
    static REFERENCE: OnceLock<Regex> = OnceLock::new();
    static ACCOUNT: OnceLock<Regex> = OnceLock::new();
    static MERCHANT: OnceLock<Regex> = OnceLock::new();

    let text = format!("{subject}\n{body}");
    let lower = text.to_lowercase();
    let amount = regex(
        &AMOUNT,
        r"(?i)(?:INR|Rs\.?|₹)\s*([0-9][0-9,]*(?:\.[0-9]{1,2})?)|([0-9][0-9,]*(?:\.[0-9]{1,2})?)\s*(?:INR|Rs\.?|₹)",
    )
    .captures(&text)
    .and_then(|captures| captures.get(1).or_else(|| captures.get(2)))
    .map(|value| value.as_str().replace(',', ""));

    let reference = regex(
        &REFERENCE,
        r"(?i)(?:UTR|UPI\s*(?:ref(?:erence)?|ID)|RRN|reference\s*(?:no|number|id)?|txn(?:\s*id)?|transaction\s*id)\s*[:#-]?\s*([A-Z0-9-]{6,40})",
    )
    .captures(&text)
    .and_then(|captures| captures.get(1))
    .map(|value| clean(value.as_str()))
    .unwrap_or_default();

    let account_hint = regex(
        &ACCOUNT,
        r"(?i)(?:a/c|account|card)\s*(?:ending|xx|x|no\.?)?\s*[:#-]?\s*([Xx*0-9]{4,16})",
    )
    .captures(&text)
    .and_then(|captures| captures.get(1))
    .map(|value| clean(value.as_str()))
    .unwrap_or_default();

    let merchant = regex(
        &MERCHANT,
        r"(?im)^\s*(?:merchant|paid to|received from|credited by|debited to|at)\s*[:\-]?\s*([^\r\n]{2,80})",
    )
    .captures(body)
    .and_then(|captures| captures.get(1))
    .map(|value| clean(value.as_str()))
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| clean(sender.split('<').next().unwrap_or(sender)));

    let event_type = if ["refund", "reversal", "reversed", "chargeback"]
        .iter()
        .any(|term| lower.contains(term))
    {
        if lower.contains("refund") || lower.contains("chargeback") {
            "refund"
        } else {
            "reversal"
        }
    } else if ["credit card payment", "card payment", "card bill"]
        .iter()
        .any(|term| lower.contains(term))
    {
        "card_payment"
    } else if ["transfer", "self transfer", "fund transfer"]
        .iter()
        .any(|term| lower.contains(term))
    {
        "transfer"
    } else if ["credited", "received", "salary", "deposit"]
        .iter()
        .any(|term| lower.contains(term))
    {
        "income"
    } else if [
        "debited",
        "paid",
        "purchase",
        "spent",
        "withdrawn",
        "charged",
    ]
    .iter()
    .any(|term| lower.contains(term))
    {
        "expense"
    } else {
        "unknown"
    };
    let kind = match event_type {
        "income" | "refund" => "income",
        "expense" => "expense",
        _ => "unknown",
    };
    let currency = if text.contains('₹')
        || lower.contains("inr")
        || lower.contains("rs.")
        || lower.contains("rs ")
    {
        "INR"
    } else {
        "UNKNOWN"
    };
    let description = if merchant.is_empty() {
        clean(subject)
    } else {
        merchant.clone()
    };

    ExtractedTransaction {
        date: transaction_date(body, received_at_ms),
        amount,
        currency: currency.to_string(),
        event_type: event_type.to_string(),
        kind: kind.to_string(),
        merchant,
        account_hint,
        reference,
        description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_upi_debit_fields_and_normalizes_amount() {
        let extracted = extract_transaction(
            "UPI payment debited",
            "Bank Alerts <alerts@example.test>",
            "Amount: INR 1,234.50\nPaid to: Local Market\nA/c XX1234\nUPI Ref: ABCD123456\nDate: 27/10/2026",
            1_793_088_000_000,
        );
        assert_eq!(extracted.date, "2026-10-27");
        assert_eq!(extracted.amount.as_deref(), Some("1234.50"));
        assert_eq!(extracted.currency, "INR");
        assert_eq!(extracted.event_type, "expense");
        assert_eq!(extracted.kind, "expense");
        assert_eq!(extracted.merchant, "Local Market");
        assert_eq!(extracted.account_hint, "1234");
        assert_eq!(extracted.reference, "ABCD123456");
    }

    #[test]
    fn flags_refunds_transfers_and_card_payments_without_importable_kinds() {
        let refund = extract_transaction("Refund received", "", "INR 250 credited", 0);
        let transfer = extract_transaction("Own account transfer", "", "INR 250 transfer", 0);
        let card = extract_transaction("Credit card bill payment", "", "Paid INR 250", 0);
        assert_eq!(
            (refund.event_type.as_str(), refund.kind.as_str()),
            ("refund", "income")
        );
        assert_eq!(
            (transfer.event_type.as_str(), transfer.kind.as_str()),
            ("transfer", "unknown")
        );
        assert_eq!(
            (card.event_type.as_str(), card.kind.as_str()),
            ("card_payment", "unknown")
        );
    }

    #[test]
    fn uses_received_date_and_marks_unrecognized_currency() {
        let extracted = extract_transaction(
            "Debit alert",
            "Bank",
            "Amount 250 debited",
            1_793_088_000_000,
        );
        assert_eq!(extracted.date, "2026-10-27");
        assert_eq!(extracted.currency, "UNKNOWN");
        assert_eq!(extracted.kind, "expense");
    }
}
