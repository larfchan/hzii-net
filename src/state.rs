use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::portal::{KeepaliveInfo, LoginInfo};

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub username: String,
    pub secret: String,
    pub login_ip: String,
    pub login_time: String,
    pub keepalive_interval: u64,
    pub online_duration: u64,
}

impl SessionState {
    pub fn from_login(info: LoginInfo) -> Self {
        Self {
            username: info.username,
            secret: info.secret,
            login_ip: info.login_ip,
            login_time: info.login_time,
            keepalive_interval: info.keepalive_interval,
            online_duration: 0,
        }
    }

    pub fn update_from_keepalive(&mut self, info: KeepaliveInfo) {
        self.username = info.username;
        self.login_ip = info.login_ip;
        self.login_time = info.login_time;
        self.keepalive_interval = info.next_interval;
        self.online_duration = info.online_duration;
    }
}

pub fn load(path: &Path) -> Result<Option<SessionState>> {
    match fs::read_to_string(path) {
        Ok(content) => {
            let state = serde_json::from_str(&content)
                .with_context(|| format!("无法解析会话文件 {}", path.display()))?;
            Ok(Some(state))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("无法读取会话文件 {}", path.display())),
    }
}

pub fn save(path: &Path, state: &SessionState) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("无法创建会话目录 {}", parent.display()))?;
    }

    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("无法写入会话文件 {}", path.display()))?;
    let content = serde_json::to_vec_pretty(state).context("无法序列化会话状态")?;
    file.write_all(&content)
        .with_context(|| format!("无法写入会话文件 {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("无法同步会话文件 {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("无法设置会话文件权限 {}", path.display()))?;
    }
    Ok(())
}

pub fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("无法删除会话文件 {}", path.display())),
    }
}
