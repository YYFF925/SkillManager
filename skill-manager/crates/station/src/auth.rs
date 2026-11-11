//! 唯一管理员：密码 + TOTP / 一次性恢复码。无公网注册。

use std::io::{self, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::RngCore;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::json;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::db::StationDb;

const COOKIE: &str = "sm_station";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminRow {
    pub username: String,
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub recovery_code_hashes: Vec<String>,
    pub session_ttl_seconds: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupShown {
    pub username: String,
    pub totp_secret: String,
    pub otpauth: String,
    pub qr_unicode: String,
    pub recovery_codes: Vec<String>,
}

pub fn cookie_name() -> &'static str {
    COOKIE
}

pub fn has_admin(db: &StationDb) -> Result<bool, String> {
    let n: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM admin", [], |row| row.get(0))
        .map_err(|err| err.to_string())?;
    Ok(n > 0)
}

pub fn load_admin(db: &StationDb) -> Result<Option<AdminRow>, String> {
    db.conn()
        .query_row(
            "SELECT username, password_hash, totp_secret, recovery_code_hashes, session_ttl_seconds FROM admin LIMIT 1",
            [],
            |row| {
                let hashes: String = row.get(3)?;
                Ok(AdminRow {
                    username: row.get(0)?,
                    password_hash: row.get(1)?,
                    totp_secret: row.get(2)?,
                    recovery_code_hashes: serde_json::from_str(&hashes).unwrap_or_default(),
                    session_ttl_seconds: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|err| err.to_string())
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|err| err.to_string())
}

fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

fn hash_code(code: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(code.as_bytes()))
}

fn random_codes(n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            let mut bytes = [0u8; 5];
            rand::thread_rng().fill_bytes(&mut bytes);
            hex::encode(bytes)
        })
        .collect()
}

fn totp_from_secret(secret: &str, username: &str) -> Result<TOTP, String> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|err| err.to_string())?,
        Some("Skill Manager".to_string()),
        username.to_string(),
    )
    .map_err(|err| err.to_string())
}

fn qr_unicode(otpauth: &str) -> String {
    match qrcode::QrCode::new(otpauth.as_bytes()) {
        Ok(code) => code
            .render::<qrcode::render::unicode::Dense1x2>()
            .quiet_zone(false)
            .build(),
        Err(_) => String::new(),
    }
}

/// 首次创建管理员；恢复码与 TOTP 密钥只在返回值里出现一次。
pub fn create_admin(db: &StationDb, username: &str, password: &str) -> Result<SetupShown, String> {
    if has_admin(db)? {
        return Err("管理员已存在，不能再注册".to_string());
    }
    if username.trim().is_empty() || password.len() < 8 {
        return Err("用户名不能为空，密码至少 8 位".to_string());
    }
    let password_hash = hash_password(password)?;
    let secret = Secret::generate_secret();
    let totp_secret = secret.to_encoded().to_string();
    let totp = totp_from_secret(&totp_secret, username)?;
    let otpauth = totp.get_url();
    let recovery_codes = random_codes(8);
    let recovery_hashes: Vec<String> = recovery_codes.iter().map(|c| hash_code(c)).collect();
    db.conn()
        .execute(
            "INSERT INTO admin (id, username, password_hash, totp_secret, recovery_code_hashes, session_ttl_seconds)
             VALUES (1, ?1, ?2, ?3, ?4, 604800)",
            rusqlite::params![
                username,
                password_hash,
                totp_secret,
                serde_json::to_string(&recovery_hashes).unwrap_or_else(|_| "[]".into()),
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(SetupShown {
        username: username.to_string(),
        totp_secret: totp_secret.clone(),
        otpauth: otpauth.clone(),
        qr_unicode: qr_unicode(&otpauth),
        recovery_codes,
    })
}

pub fn reset_admin(db: &StationDb) -> Result<(), String> {
    db.conn()
        .execute_batch("DELETE FROM admin; DELETE FROM sessions;")
        .map_err(|err| err.to_string())
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn expiry(ttl: i64) -> String {
    let t = SystemTime::now() + Duration::from_secs(ttl.max(60) as u64);
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

pub fn create_session(db: &StationDb, ttl: i64) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    db.conn()
        .execute(
            "INSERT INTO sessions (session_id, created_at, expires_at, last_device_id) VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params![id, now_rfc3339(), expiry(ttl)],
        )
        .map_err(|err| err.to_string())?;
    Ok(id)
}

pub fn session_valid(db: &StationDb, session_id: &str) -> Result<bool, String> {
    let now = now_rfc3339();
    let found: Option<String> = db
        .conn()
        .query_row(
            "SELECT session_id FROM sessions WHERE session_id = ?1 AND expires_at > ?2",
            rusqlite::params![session_id, now],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    Ok(found.is_some())
}

pub fn session_device(db: &StationDb, session_id: &str) -> Result<Option<String>, String> {
    let row: Option<Option<String>> = db
        .conn()
        .query_row(
            "SELECT last_device_id FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    Ok(row.flatten())
}

pub fn set_session_device(
    db: &StationDb,
    session_id: &str,
    device_id: Option<&str>,
) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE sessions SET last_device_id = ?1 WHERE session_id = ?2",
            rusqlite::params![device_id, session_id],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn delete_session(db: &StationDb, session_id: &str) -> Result<(), String> {
    db.conn()
        .execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn set_ttl(db: &StationDb, seconds: i64) -> Result<i64, String> {
    if seconds < 60 {
        return Err("登录保持时长至少 60 秒".to_string());
    }
    db.conn()
        .execute("UPDATE admin SET session_ttl_seconds = ?1", [seconds])
        .map_err(|err| err.to_string())?;
    Ok(seconds)
}

#[derive(Clone, Debug)]
pub struct LoginResult {
    pub session_id: String,
    pub must_rebind_totp: bool,
    pub rebind: Option<SetupShown>,
}

fn mint_totp(username: &str) -> Result<SetupShown, String> {
    let secret = Secret::generate_secret();
    let totp_secret = secret.to_encoded().to_string();
    let totp = totp_from_secret(&totp_secret, username)?;
    let otpauth = totp.get_url();
    let recovery_codes = random_codes(8);
    Ok(SetupShown {
        username: username.to_string(),
        totp_secret,
        otpauth: otpauth.clone(),
        qr_unicode: qr_unicode(&otpauth),
        recovery_codes,
    })
}

/// 登录：用户名 + 密码 + TOTP 或恢复码。恢复码登录必须能重新绑定 TOTP。
pub fn login(
    db: &StationDb,
    username: &str,
    password: &str,
    otp: &str,
) -> Result<LoginResult, String> {
    let admin = load_admin(db)?.ok_or_else(|| "尚未创建管理员".to_string())?;
    if admin.username != username || !verify_password(&admin.password_hash, password) {
        return Err("用户名或密码不正确".to_string());
    }
    let totp_ok = admin
        .totp_secret
        .as_deref()
        .and_then(|secret| totp_from_secret(secret, &admin.username).ok())
        .and_then(|totp| totp.check_current(otp).ok())
        .unwrap_or(false);
    let recovery_hash = hash_code(otp);
    let recovery_ok = admin
        .recovery_code_hashes
        .iter()
        .any(|h| h == &recovery_hash);
    if !totp_ok && !recovery_ok {
        return Err("一次性密码或恢复码不正确".to_string());
    }
    let mut rebind = None;
    if recovery_ok {
        let shown = mint_totp(&admin.username)?;
        let recovery_hashes: Vec<String> =
            shown.recovery_codes.iter().map(|c| hash_code(c)).collect();
        db.conn()
            .execute(
                "UPDATE admin SET totp_secret = ?1, recovery_code_hashes = ?2",
                rusqlite::params![
                    shown.totp_secret,
                    serde_json::to_string(&recovery_hashes).unwrap_or_else(|_| "[]".into()),
                ],
            )
            .map_err(|err| err.to_string())?;
        rebind = Some(shown);
    }
    let session_id = create_session(db, admin.session_ttl_seconds)?;
    Ok(LoginResult {
        session_id,
        must_rebind_totp: rebind.is_some(),
        rebind,
    })
}

/// 恢复码登录后确认新的 TOTP 已写入验证器。
pub fn confirm_totp_rebind(db: &StationDb, otp: &str) -> Result<serde_json::Value, String> {
    let admin = load_admin(db)?.ok_or_else(|| "尚未创建管理员".to_string())?;
    let secret = admin
        .totp_secret
        .ok_or_else(|| "尚未生成新的一次性密码，请重新用恢复码登录".to_string())?;
    let totp = totp_from_secret(&secret, &admin.username)?;
    if totp.check_current(otp).unwrap_or(false) {
        Ok(json!({ "ok": true, "bound": true }))
    } else {
        Err("一次性密码不正确，请确认验证器已添加新密钥".to_string())
    }
}

pub fn ttl(db: &StationDb) -> Result<i64, String> {
    Ok(load_admin(db)?
        .ok_or_else(|| "尚未创建管理员".to_string())?
        .session_ttl_seconds)
}

/// 本机交互式创建管理员（仅 TTY）。
pub fn interactive_setup(db: &StationDb) -> Result<SetupShown, String> {
    let mut username = String::new();
    print!("尚未创建管理员。用户名：");
    let _ = io::stdout().flush();
    io::stdin()
        .read_line(&mut username)
        .map_err(|err| err.to_string())?;
    let username = username.trim().to_string();
    print!("密码（至少 8 位）：");
    let _ = io::stdout().flush();
    let mut password = String::new();
    io::stdin()
        .read_line(&mut password)
        .map_err(|err| err.to_string())?;
    let shown = create_admin(db, &username, password.trim())?;
    eprintln!("请用验证器扫描或手动输入密钥：{}", shown.totp_secret);
    eprintln!("{}", shown.qr_unicode);
    eprintln!("一次性恢复码（只显示一次）：");
    for code in &shown.recovery_codes {
        eprintln!("  {code}");
    }
    Ok(shown)
}

pub fn verify_totp(db: &StationDb, otp: &str) -> Result<serde_json::Value, String> {
    let admin = load_admin(db)?.ok_or_else(|| "尚未创建管理员".to_string())?;
    let secret = admin
        .totp_secret
        .ok_or_else(|| "尚未绑定一次性密码".to_string())?;
    let totp = totp_from_secret(&secret, &admin.username)?;
    if totp.check_current(otp).unwrap_or(false) {
        Ok(json!({ "ok": true }))
    } else {
        Err("一次性密码不正确".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::StationDb;

    #[test]
    fn recovery_login_issues_new_totp_instead_of_only_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let db = StationDb::open(dir.path()).unwrap();
        let shown = create_admin(&db, "admin", "password1").unwrap();
        let code = shown.recovery_codes[0].clone();
        let result = login(&db, "admin", "password1", &code).unwrap();
        assert!(result.must_rebind_totp);
        let rebind = result.rebind.expect("recovery login must return new TOTP");
        assert!(!rebind.totp_secret.is_empty());
        assert!(!rebind.recovery_codes.is_empty());
        assert_ne!(rebind.totp_secret, shown.totp_secret);
        let admin = load_admin(&db).unwrap().unwrap();
        assert_eq!(
            admin.totp_secret.as_deref(),
            Some(rebind.totp_secret.as_str())
        );
        let reused = login(&db, "admin", "password1", &code);
        assert!(
            reused.is_err(),
            "consumed recovery code must not work again"
        );
    }
}
