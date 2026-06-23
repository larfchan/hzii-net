use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::crypto::{self, CryptoError};

#[derive(Debug, Error)]
pub enum PortalError {
    #[error("HTTP 请求失败：{0}")]
    Transport(String),
    #[error("Portal 返回 HTTP {status}：{body}")]
    HttpStatus { status: u16, body: String },
    #[error("认证失败：{0}")]
    Authentication(String),
    #[error("当前会话已离线：{0}")]
    Offline(String),
    #[error("服务器要求浏览器继续认证：{0}")]
    SecondaryAuthentication(String),
    #[error("Portal 响应格式异常：{0}")]
    InvalidResponse(String),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

pub struct PortalClient {
    agent: ureq::Agent,
    server: String,
    origin: String,
    referer: String,
}

pub struct LoginInfo {
    pub username: String,
    pub login_time: String,
    pub login_ip: String,
    pub secret: String,
    pub keepalive_interval: u64,
    pub unix_login_time: String,
    pub must_change_password: bool,
    pub can_change_password: bool,
}

pub struct KeepaliveInfo {
    pub next_interval: u64,
    pub online_duration: u64,
    pub login_time: String,
    pub username: String,
    pub login_ip: String,
    pub must_change_password: bool,
    pub can_change_password: bool,
}

impl PortalClient {
    pub fn new(server: &str, timeout_seconds: u64) -> Self {
        let server = server.trim_end_matches('/').to_owned();
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(timeout_seconds))
            .timeout_read(Duration::from_secs(timeout_seconds))
            .timeout_write(Duration::from_secs(timeout_seconds))
            .build();
        Self {
            origin: server.clone(),
            referer: format!("{server}/portal/local/index.html"),
            server,
            agent,
        }
    }

    pub fn login(
        &self,
        username: &str,
        password: &str,
        uplcyid: &str,
        language: u8,
        check_captcha: bool,
    ) -> Result<LoginInfo, PortalError> {
        let code = if check_captcha {
            self.captcha_code()?.unwrap_or_default()
        } else {
            String::new()
        };
        let encrypted_username = crypto::encrypt(username)?;
        let encrypted_password = crypto::encrypt(password)?;
        let language = language.to_string();
        let form = [
            ("username", encrypted_username.as_str()),
            ("password", encrypted_password.as_str()),
            ("uplcyid", uplcyid),
            ("language", language.as_str()),
            ("code", code.as_str()),
            ("submit", "submit"),
        ];
        let body = self.post_form("/portal.cgi", &form)?;
        parse_login_response(&body, username)
    }

    pub fn keepalive(&self, secret: &str) -> Result<KeepaliveInfo, PortalError> {
        let body = self.post_form(
            "/keepalive.cgi",
            &[("secret", secret), ("submit", "submit")],
        )?;
        parse_keepalive_response(&body)
    }

    pub fn logout(&self, username: &str, secret: &str, language: u8) -> Result<(), PortalError> {
        let language = language.to_string();
        let body = self.post_form(
            "/logout.cgi",
            &[
                ("username", username),
                ("secret", secret),
                ("language", language.as_str()),
                ("submit", "submit"),
            ],
        )?;
        if body.starts_with("0#") {
            return Err(PortalError::Authentication(message_after_hash(&body)));
        }
        Ok(())
    }

    fn captcha_code(&self) -> Result<Option<String>, PortalError> {
        let body = self.post_form("/user_auth_verify.cgi", &[("submit", "submit")])?;
        parse_captcha_response(&body)
    }

    fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<String, PortalError> {
        let url = format!("{}{path}", self.server);
        let result = self
            .agent
            .post(&url)
            .set("Accept", "*/*")
            .set("Origin", &self.origin)
            .set("Referer", &self.referer)
            .set("HTTP_X_REQUESTED_WITH", "xmlhttprequest")
            .set("X-Requested-With", "XMLHttpRequest")
            .set(
                "User-Agent",
                concat!("hzii-net/", env!("CARGO_PKG_VERSION")),
            )
            .send_form(form);

        match result {
            Ok(response) => response
                .into_string()
                .map_err(|error| PortalError::Transport(error.to_string())),
            Err(ureq::Error::Status(status, response)) => {
                let body = response.into_string().unwrap_or_default();
                Err(PortalError::HttpStatus {
                    status,
                    body: safe_excerpt(&body),
                })
            }
            Err(ureq::Error::Transport(error)) => Err(PortalError::Transport(error.to_string())),
        }
    }
}

fn parse_captcha_response(body: &str) -> Result<Option<String>, PortalError> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        PortalError::InvalidResponse(format!("验证码接口不是有效 JSON：{error}"))
    })?;
    let verify = value.get("verify").and_then(|item| {
        item.as_i64()
            .or_else(|| item.as_str().and_then(|text| text.parse().ok()))
            .or_else(|| item.as_bool().map(i64::from))
    });

    // The browser only asks for a code when `data.verify == 1`. Responses such
    // as {"result":"fail"} therefore mean "continue without a captcha".
    if verify != Some(1) {
        return Ok(None);
    }

    let code = value
        .get("code")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| PortalError::InvalidResponse("验证码响应缺少 code".to_owned()))?;
    Ok(Some(code.to_owned()))
}

fn parse_login_response(body: &str, expected_username: &str) -> Result<LoginInfo, PortalError> {
    if body.starts_with("0#") {
        return Err(PortalError::Authentication(message_after_hash(body)));
    }
    if body.starts_with("1#") {
        return Err(PortalError::SecondaryAuthentication(message_after_hash(
            body,
        )));
    }

    let fields: Vec<&str> = body.trim().split('&').collect();
    if fields.len() < 8 {
        return Err(PortalError::InvalidResponse(format!(
            "登录响应应至少有 8 个字段，实际为 {}",
            fields.len()
        )));
    }
    let username = crypto::decrypt(fields[0])?;
    if username != expected_username {
        return Err(PortalError::InvalidResponse(
            "登录响应中的用户名与请求不一致".to_owned(),
        ));
    }
    if fields[3].is_empty() {
        return Err(PortalError::InvalidResponse(
            "登录响应缺少 secret".to_owned(),
        ));
    }

    Ok(LoginInfo {
        username,
        login_time: fields[1].to_owned(),
        login_ip: fields[2].to_owned(),
        secret: fields[3].to_owned(),
        keepalive_interval: parse_number(fields[4], "登录保活间隔")?,
        unix_login_time: fields[5].to_owned(),
        must_change_password: fields[6] == "1",
        can_change_password: fields[7] != "0",
    })
}

fn parse_keepalive_response(body: &str) -> Result<KeepaliveInfo, PortalError> {
    if body.starts_with("0#") {
        return Err(PortalError::Offline(message_after_hash(body)));
    }
    let fields: Vec<&str> = body.trim().split('&').collect();
    if fields.len() < 8 {
        return Err(PortalError::InvalidResponse(format!(
            "保活响应应至少有 8 个字段，实际为 {}",
            fields.len()
        )));
    }
    Ok(KeepaliveInfo {
        next_interval: parse_number(fields[0], "下次保活间隔")?,
        online_duration: parse_number(fields[1], "在线时长")?,
        login_time: fields[3].to_owned(),
        username: fields[4].to_owned(),
        login_ip: fields[5].to_owned(),
        must_change_password: fields[6] == "1",
        can_change_password: fields[7] != "0",
    })
}

fn parse_number(value: &str, field: &str) -> Result<u64, PortalError> {
    value
        .parse()
        .map_err(|_| PortalError::InvalidResponse(format!("{field} 不是有效的非负整数")))
}

fn message_after_hash(body: &str) -> String {
    safe_excerpt(body.split_once('#').map_or(body, |(_, message)| message))
}

fn safe_excerpt(body: &str) -> String {
    let text: String = body
        .trim()
        .chars()
        .take(200)
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    if text.is_empty() {
        "服务器未返回错误详情".to_owned()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::encrypt_with_iv;

    #[test]
    fn parses_login_response() {
        let username = "test-user";
        let encrypted = encrypt_with_iv(username, "0123456789ABCDEF").unwrap();
        let response =
            format!("{encrypted}&2026-06-23 15:28:55&172.19.61.130&session-token&451&123456&0&1");
        let info = parse_login_response(&response, username).unwrap();
        assert_eq!(info.username, username);
        assert_eq!(info.keepalive_interval, 451);
        assert_eq!(info.secret, "session-token");
    }

    #[test]
    fn parses_keepalive_response() {
        let response = "451&1095&1&2026-06-23 15:18:21&test-user&172.19.61.130&0&1";
        let info = parse_keepalive_response(response).unwrap();
        assert_eq!(info.next_interval, 451);
        assert_eq!(info.online_duration, 1095);
        assert_eq!(info.username, "test-user");
    }

    #[test]
    fn classifies_offline_response() {
        assert!(matches!(
            parse_keepalive_response("0#offline"),
            Err(PortalError::Offline(_))
        ));
    }

    #[test]
    fn accepts_captcha_result_without_verify() {
        assert_eq!(parse_captcha_response(r#"{"result":"fail"}"#).unwrap(), None);
        assert_eq!(
            parse_captcha_response(r#"{"verify":1,"code":"a1b2"}"#).unwrap(),
            Some("a1b2".to_owned())
        );
    }
}
