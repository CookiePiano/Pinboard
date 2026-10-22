//! Credentials kept in the OS keyring (Windows Credential Manager, Secret Service on Linux).

const SERVICE: &str = "dev.pinboard.notes";

pub const GDRIVE_REFRESH_TOKEN: &str = "gdrive-refresh-token";
pub const GDRIVE_CLIENT_SECRET: &str = "gdrive-client-secret";

fn entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|e| tr!("키링을 열지 못했습니다: {e}", "Could not open the keyring: {e}"; "无法打开凭据存储：{e}", "キーリングを開けませんでした：{e}"; "Impossible d'ouvrir le trousseau : {e}", "No se pudo abrir el llavero: {e}"))
}

pub fn get(name: &str) -> Option<String> {
    entry(name).ok()?.get_password().ok().filter(|s| !s.is_empty())
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    entry(name)?
        .set_password(value)
        .map_err(|e| tr!("키링에 저장하지 못했습니다: {e}", "Could not save to the keyring: {e}"; "无法保存到凭据存储：{e}", "キーリングに保存できませんでした：{e}"; "Impossible d'enregistrer dans le trousseau : {e}", "No se pudo guardar en el llavero: {e}"))
}

pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(tr!("키링에서 지우지 못했습니다: {e}", "Could not remove from the keyring: {e}"; "无法从凭据存储中删除：{e}", "キーリングから削除できませんでした：{e}"; "Impossible de supprimer du trousseau : {e}", "No se pudo eliminar del llavero: {e}")),
    }
}
