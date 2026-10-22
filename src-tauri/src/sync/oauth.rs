//! Google OAuth for installed apps: loopback redirect with PKCE.
//! Tokens are never logged; errors carry only Google's error code and description.

use super::{Result, SyncError};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &str = "https://www.googleapis.com/auth/drive.appdata openid email";

pub struct Tokens {
    pub access_token: String,
    pub expires_at: Instant,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

fn token_request(client: &Client, form: &[(&str, &str)]) -> Result<Tokens> {
    let resp = client
        .post(TOKEN_URL)
        .form(form)
        .send()
        .map_err(|e| SyncError::Remote(tr!("Google에 연결하지 못했습니다: {}", "Could not connect to Google: {}"; "无法连接 Google：{}", "Google に接続できませんでした：{}"; "Impossible de se connecter à Google : {}", "No se pudo conectar con Google: {}", e.without_url())))?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        let err: Option<TokenError> = serde_json::from_str(&text).ok();
        return Err(match err {
            Some(e) if e.error == "invalid_grant" => SyncError::NotSignedIn,
            Some(e) => SyncError::Remote(tr!(
                "Google 인증 오류: {} {}", "Google authentication error: {} {}"; "Google 身份验证错误：{} {}", "Google 認証エラー：{} {}"; "Erreur d'authentification Google : {} {}", "Error de autenticación de Google: {} {}",
                e.error,
                e.error_description.unwrap_or_default()
            )),
            None => SyncError::Remote(tr!("Google 인증 오류: HTTP {status}", "Google authentication error: HTTP {status}"; "Google 身份验证错误：HTTP {status}", "Google 認証エラー：HTTP {status}"; "Erreur d'authentification Google : HTTP {status}", "Error de autenticación de Google: HTTP {status}")),
        });
    }
    let t: TokenResponse = serde_json::from_str(&text)
        .map_err(|_| SyncError::Remote(tr!("Google 인증 응답을 해석하지 못했습니다.", "Could not read the Google authentication response."; "无法解析 Google 身份验证响应。", "Google の認証応答を読み取れませんでした。"; "Impossible de lire la réponse d'authentification de Google.", "No se pudo leer la respuesta de autenticación de Google.")))?;
    Ok(Tokens {
        access_token: t.access_token,
        expires_at: Instant::now() + Duration::from_secs(t.expires_in.unwrap_or(3600)),
        refresh_token: t.refresh_token,
        email: t.id_token.as_deref().and_then(email_from_id_token),
    })
}

/// Reads the email claim for display. The token came straight from Google over TLS,
/// and nothing here relies on it for authorization, so the signature is not checked.
fn email_from_id_token(jwt: &str) -> Option<String> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("email")?.as_str().map(str::to_string)
}

pub fn refresh(client: &Client, client_id: &str, client_secret: &str, refresh_token: &str) -> Result<Tokens> {
    token_request(
        client,
        &[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ],
    )
}

/// Runs the browser sign-in. `open` shows the consent page; this waits up to `timeout`
/// for Google to redirect back to the loopback listener.
pub fn sign_in(
    client: &Client,
    client_id: &str,
    client_secret: &str,
    open: impl FnOnce(&str) -> std::result::Result<(), String>,
    timeout: Duration,
) -> Result<Tokens> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = uuid::Uuid::new_v4().simple().to_string();
    let url = url::Url::parse_with_params(
        AUTH_URL,
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", state.as_str()),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .expect("valid auth url");
    open(url.as_str()).map_err(SyncError::Config)?;

    let code = wait_for_code(&listener, &state, timeout)?;
    token_request(
        client,
        &[
            ("code", code.as_str()),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
        ],
    )
}

fn wait_for_code(listener: &TcpListener, state: &str, timeout: Duration) -> Result<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return Err(SyncError::Remote("Google sign-in timed out. Try again.".into()));
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(result) = handle_redirect(stream, state, deadline)? {
                    return result;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(SyncError::Remote(tr!("Google 로그인 시간이 초과되었습니다. 다시 시도하세요.", "Google sign-in timed out. Try again."; "Google 登录超时。请重试。", "Google へのログインがタイムアウトしました。再試行してください。"; "La connexion à Google a expiré. Réessayez.", "Se agotó el tiempo para iniciar sesión en Google. Inténtalo de nuevo.")));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Returns `None` for unrelated requests (like a favicon) so the caller keeps waiting.
fn handle_redirect(mut stream: TcpStream, state: &str, deadline: Instant) -> Result<Option<Result<String>>> {
    stream.set_nonblocking(false)?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    let read_deadline = deadline.min(Instant::now() + Duration::from_secs(2));
    let mut buf = vec![0u8; 8192];
    let mut len = 0;
    while len < buf.len() {
        let remaining = read_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() { return Ok(None); }
        stream.set_read_timeout(Some(remaining))?;
        let n = stream.read(&mut buf[len..]).unwrap_or(0);
        if n == 0 {
            break;
        }
        len += n;
        if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    if !buf[..len].windows(4).any(|w| w == b"\r\n\r\n") { return Ok(None); }
    let request = String::from_utf8_lossy(&buf[..len]);
    let mut line = request.lines().next().unwrap_or("").split_whitespace();
    let (method, target, version) = (line.next(), line.next().unwrap_or(""), line.next());
    if method != Some("GET") || !matches!(version, Some("HTTP/1.0" | "HTTP/1.1"))
        || line.next().is_some() || !(target == "/" || target.starts_with("/?"))
        || target.contains('#') || target.contains('\\')
    {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Ok(None);
    }
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}")) else { return Ok(None) };
    let param = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());

    // Authenticate error responses too. Stray requests must not cancel a real login.
    if param("state").as_deref() != Some(state)
        || ["state", "code", "error"].iter().any(|name| url.query_pairs().filter(|(key, _)| key.as_ref() == *name).count() > 1)
    {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Ok(None);
    }

    let (code, error) = (param("code"), param("error"));
    if (code.is_none() && error.is_none()) || (code.is_some() && error.is_some())
        || code.as_ref().is_some_and(|value| value.is_empty()) {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Ok(None);
    }
    let result = match (code, error) {
        (_, Some(err)) => Err(SyncError::Remote(tr!("Google 로그인이 취소되었습니다 ({err}).", "Google sign-in was cancelled ({err})."; "Google 登录已取消（{err}）。", "Google へのログインがキャンセルされました（{err}）。"; "La connexion à Google a été annulée ({err}).", "Se canceló el inicio de sesión en Google ({err}).")) ),
        (Some(code), None) => Ok(code),
        (None, None) => unreachable!(),
    };
    let message = if result.is_ok() {
        tr!("Pinboard에 Google 계정을 연결했습니다. 이 탭을 닫아도 됩니다.", "Google sign-in received. Return to Pinboard to finish connecting. You can close this tab."; "已收到 Google 登录信息。请返回 Pinboard 完成连接。可以关闭此标签页。", "Google のログイン情報を受信しました。Pinboard に戻って接続を完了してください。このタブは閉じても構いません。"; "Connexion Google reçue. Revenez à Pinboard pour terminer la connexion. Vous pouvez fermer cet onglet.", "Se recibió el inicio de sesión de Google. Vuelve a Pinboard para terminar la conexión. Puedes cerrar esta pestaña.")
    } else {
        tr!("Google 계정을 연결하지 못했습니다. Pinboard 설정에서 다시 시도하세요.", "Could not connect your Google account. Try again in Pinboard settings."; "无法连接 Google 账号。请在 Pinboard 设置中重试。", "Google アカウントを接続できませんでした。Pinboard の設定から再試行してください。"; "Impossible de connecter votre compte Google. Réessayez dans les paramètres de Pinboard.", "No se pudo conectar tu cuenta de Google. Inténtalo de nuevo en la configuración de Pinboard.")
    };
    let lang = crate::i18n::get().tag();
    let body = format!(
        "<!doctype html><html lang={lang}><meta charset=utf-8><title>Pinboard</title>\
         <body style=\"font-family:system-ui,sans-serif;padding:48px;color:#1F252B\">{message}</body></html>"
    );
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_unauthenticated_callbacks_do_not_cancel_sign_in() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            for target in [":invalid-port", "/?error=access_denied", "/?error=access_denied&state=wrong",
                "/?code=bad&state=wrong", "/?code=bad&state=st&state=other", "/?code=one&code=two&state=st",
                "/?code=&state=st", "/?code=good&state=st"] {
                let mut stream = TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                stream.write_all(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                if target.contains("code=good") { assert!(response.contains("200 OK")); }
                else { assert!(!response.contains("200 OK")); }
            }
        });
        assert_eq!(wait_for_code(&listener, "st", Duration::from_secs(5)).unwrap(), "good");
        client.join().unwrap();
    }

    #[test]
    fn an_idle_callback_connection_cannot_extend_the_login_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let _idle = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let start = Instant::now();
        assert!(wait_for_code(&listener, "st", Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn reads_email_claim() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"email":"someone@example.com","sub":"1"}"#);
        assert_eq!(
            email_from_id_token(&format!("h.{payload}.s")).as_deref(),
            Some("someone@example.com")
        );
        assert_eq!(email_from_id_token("garbage"), None);
    }

    #[test]
    fn loopback_accepts_code_and_checks_state() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let send = |path: &str| {
                let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
                s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).unwrap();
                let mut out = String::new();
                let _ = s.read_to_string(&mut out);
                out
            };
            send("/favicon.ico");
            send("/?code=abc&state=st")
        });
        let code = wait_for_code(&listener, "st", Duration::from_secs(5)).unwrap();
        assert_eq!(code, "abc");
        assert!(client.join().unwrap().contains("200 OK"));
    }
}
