//! Sync through the Google Drive API. Notes are `<id>.json` files in the app's private
//! `appDataFolder`, which only Pinboard can see. Each file carries its content hash in
//! `appProperties`, so listing does not need to download every note.

use super::oauth;
use super::{RemoteMeta, Result, SyncBackend, SyncError};
use crate::model::Note;
use crate::store::note_id_from_file;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";

pub struct DriveBackend {
    client: Client,
    account: String,
    client_id: String,
    client_secret: String,
    refresh_token: String,
    access: Option<(String, Instant)>,
    /// Note id to Drive file id, filled by `list`.
    files: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    next_page_token: Option<String>,
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    name: String,
    #[serde(default)]
    app_properties: HashMap<String, String>,
}

pub fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Pinboard/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("http client")
}

impl DriveBackend {
    pub fn new(account: String, client_id: String, client_secret: String, refresh_token: String) -> Self {
        DriveBackend {
            client: http_client(),
            account,
            client_id,
            client_secret,
            refresh_token,
            access: None,
            files: HashMap::new(),
        }
    }

    fn token(&mut self) -> Result<String> {
        if let Some((token, expires)) = &self.access {
            if Instant::now() + Duration::from_secs(60) < *expires {
                return Ok(token.clone());
            }
        }
        let t = oauth::refresh(&self.client, &self.client_id, &self.client_secret, &self.refresh_token)?;
        self.access = Some((t.access_token.clone(), t.expires_at));
        Ok(t.access_token)
    }

    /// Sends a request, refreshing the access token once if Google rejects it.
    fn send(&mut self, build: impl Fn(&Client) -> RequestBuilder) -> Result<Response> {
        for attempt in 0..2 {
            let token = self.token()?;
            let resp = build(&self.client)
                .bearer_auth(token)
                .send()
                .map_err(|e| SyncError::Remote(tr!("Google Drive에 연결하지 못했습니다: {}", "Could not connect to Google Drive: {}"; "无法连接 Google Drive：{}", "Google Drive に接続できませんでした：{}"; "Impossible de se connecter à Google Drive : {}", "No se pudo conectar con Google Drive: {}", e.without_url())))?;
            if resp.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                self.access = None;
                continue;
            }
            return Ok(resp);
        }
        unreachable!()
    }

    fn check(resp: Response) -> Result<Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let body = resp.text().unwrap_or_default();
        let reason = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or_default();
        Err(SyncError::Remote(tr!("Google Drive 오류 (HTTP {}): {reason}", "Google Drive error (HTTP {}): {reason}"; "Google Drive 错误（HTTP {}）：{reason}", "Google Drive エラー（HTTP {}）：{reason}"; "Erreur Google Drive (HTTP {}) : {reason}", "Error de Google Drive (HTTP {}): {reason}", status.as_u16())))
    }

    fn multipart(meta: &serde_json::Value, note: &Note) -> (String, Vec<u8>) {
        let boundary = format!("pinboard-{}", uuid::Uuid::new_v4().simple());
        let media = serde_json::to_string(note).expect("serializable");
        let body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n\
             --{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{media}\r\n--{boundary}--"
        );
        (format!("multipart/related; boundary={boundary}"), body.into_bytes())
    }
}

impl SyncBackend for DriveBackend {
    fn key(&self) -> String {
        format!("gdrive:{}", self.account)
    }

    fn list(&mut self) -> Result<Vec<RemoteMeta>> {
        let mut page: Option<String> = None;
        let mut found: Vec<DriveFile> = Vec::new();
        loop {
            let token = page.clone();
            let resp = self.send(|c| {
                let mut q = vec![
                    ("spaces", "appDataFolder".to_string()),
                    ("fields", "nextPageToken,files(id,name,appProperties)".to_string()),
                    ("pageSize", "1000".to_string()),
                ];
                if let Some(t) = &token {
                    q.push(("pageToken", t.clone()));
                }
                c.get(format!("{API}/files")).query(&q)
            })?;
            let list: FileList = Self::check(resp)?
                .json()
                .map_err(|_| SyncError::Remote(tr!("Google Drive 목록을 해석하지 못했습니다.", "Could not read the Google Drive file list."; "无法解析 Google Drive 文件列表。", "Google Drive のファイル一覧を読み取れませんでした。"; "Impossible de lire la liste des fichiers Google Drive.", "No se pudo leer la lista de archivos de Google Drive.")))?;
            found.extend(list.files);
            match list.next_page_token {
                Some(t) => page = Some(t),
                None => break,
            }
        }

        self.files.clear();
        let mut metas = Vec::new();
        for f in found {
            let Some(id) = note_id_from_file(Path::new(&f.name)) else { continue };
            if self.files.contains_key(&id) {
                // Two uploads raced and made duplicates; keep the first one.
                continue;
            }
            self.files.insert(id.clone(), f.id.clone());
            let hash = match f.app_properties.get("hash") {
                Some(h) => h.clone(),
                None => self.get(&id)?.content_hash(),
            };
            metas.push(RemoteMeta { id, hash });
        }
        Ok(metas)
    }

    fn get(&mut self, id: &str) -> Result<Note> {
        let file = self
            .files
            .get(id)
            .cloned()
            .ok_or_else(|| SyncError::Remote(tr!("Google Drive에 메모 {id}가 없습니다.", "Note {id} was not found in Google Drive."; "Google Drive 中未找到便签 {id}。", "Google Drive にメモ {id} が見つかりません。"; "La note {id} est introuvable dans Google Drive.", "No se encontró la nota {id} en Google Drive.")))?;
        let resp = self.send(|c| c.get(format!("{API}/files/{file}")).query(&[("alt", "media")]))?;
        let note = crate::validation::json(Self::check(resp)?, crate::validation::MAX_NOTE_JSON)?;
        crate::validation::matching_note(&note, id)?;
        Ok(note)
    }

    fn put(&mut self, note: &Note) -> Result<()> {
        crate::validation::note(note)?;
        let props = serde_json::json!({
            "hash": note.content_hash(),
            "updatedAt": note.updated_at.to_string(),
        });
        match self.files.get(&note.id).cloned() {
            Some(file) => {
                let (ctype, body) = Self::multipart(&serde_json::json!({ "appProperties": props }), note);
                let resp = self.send(|c| {
                    c.patch(format!("{UPLOAD}/files/{file}"))
                        .query(&[("uploadType", "multipart")])
                        .header("Content-Type", &ctype)
                        .body(body.clone())
                })?;
                Self::check(resp)?;
            }
            None => {
                let meta = serde_json::json!({
                    "name": format!("{}.json", note.id),
                    "parents": ["appDataFolder"],
                    "appProperties": props,
                });
                let (ctype, body) = Self::multipart(&meta, note);
                let resp = self.send(|c| {
                    c.post(format!("{UPLOAD}/files"))
                        .query(&[("uploadType", "multipart"), ("fields", "id")])
                        .header("Content-Type", &ctype)
                        .body(body.clone())
                })?;
                let created: serde_json::Value = Self::check(resp)?
                    .json()
                    .map_err(|_| SyncError::Remote(tr!("Google Drive 업로드 응답을 해석하지 못했습니다.", "Could not read the Google Drive upload response."; "无法解析 Google Drive 上传响应。", "Google Drive のアップロード応答を読み取れませんでした。"; "Impossible de lire la réponse d'envoi de Google Drive.", "No se pudo leer la respuesta de carga de Google Drive.")))?;
                if let Some(file) = created["id"].as_str() {
                    self.files.insert(note.id.clone(), file.to_string());
                }
            }
        }
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<()> {
        let Some(file) = self.files.get(id).cloned() else { return Ok(()) };
        let resp = self.send(|c| c.delete(format!("{API}/files/{file}")))?;
        if resp.status() != StatusCode::NOT_FOUND {
            Self::check(resp)?;
        }
        self.files.remove(id);
        Ok(())
    }
}
