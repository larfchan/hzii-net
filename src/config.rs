use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub uplcyid: String,
    pub language: u8,
    pub timeout_seconds: u64,
    pub check_captcha: bool,
    pub retry_seconds: u64,
    pub keepalive_margin_seconds: u64,
    pub max_keepalive_failures: u32,
    pub state_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: "http://10.10.0.10:8008".to_owned(),
            username: None,
            password: None,
            uplcyid: "null".to_owned(),
            language: 0,
            timeout_seconds: 30,
            check_captcha: true,
            retry_seconds: 15,
            keepalive_margin_seconds: 10,
            max_keepalive_failures: 3,
            state_file: None,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件 {}", path.display()))?;
        let mut config: Self = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件 {}", path.display()))?;

        if let Ok(server) = env::var("HZII_NET_SERVER") {
            config.server = server;
        }
        if let Ok(username) = env::var("HZII_NET_USERNAME") {
            config.username = Some(username);
        }
        if let Ok(password) = env::var("HZII_NET_PASSWORD") {
            config.password = Some(password);
        }

        config.server = config.server.trim_end_matches('/').to_owned();
        config.validate()?;
        Ok(config)
    }

    pub fn credentials(&self) -> Result<(String, String)> {
        let username = self
            .username
            .as_deref()
            .filter(|value| !value.is_empty())
            .context("未配置用户名；请设置 username 或 HZII_NET_USERNAME")?;
        let password = self
            .password
            .as_deref()
            .filter(|value| !value.is_empty())
            .context("未配置密码；请设置 password 或 HZII_NET_PASSWORD")?;
        Ok((username.to_owned(), password.to_owned()))
    }

    pub fn state_path(&self) -> PathBuf {
        self.state_file
            .clone()
            .unwrap_or_else(|| env::temp_dir().join("hzii-net-session.json"))
    }

    fn validate(&self) -> Result<()> {
        if !self.server.starts_with("http://") {
            bail!("本项目仅支持 HTTP Portal，server 必须以 http:// 开头");
        }
        if self.language > 1 {
            bail!("language 只能为 0（中文）或 1（英文）");
        }
        if self.timeout_seconds == 0 || self.retry_seconds == 0 {
            bail!("timeout_seconds 和 retry_seconds 必须大于 0");
        }
        if self.max_keepalive_failures == 0 {
            bail!("max_keepalive_failures 必须大于 0");
        }
        Ok(())
    }
}
