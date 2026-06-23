use std::{path::PathBuf, process::ExitCode, thread, time::Duration};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hzii_net::{
    config::Config,
    portal::{PortalClient, PortalError},
    state::{self, SessionState},
};

#[derive(Parser)]
#[command(author, version, about = "北航杭州创新研究院校园网无感认证客户端")]
struct Cli {
    #[arg(
        short,
        long,
        global = true,
        default_value = "config.toml",
        env = "HZII_NET_CONFIG"
    )]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 登录一次并保存会话信息
    Login,
    /// 常驻运行：登录、保活并在掉线后自动重连
    Watch,
    /// 使用已保存的会话检查在线状态（会触发一次保活）
    Status,
    /// 注销已保存的会话
    Logout,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("错误：{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;
    let client = PortalClient::new(&config.server, config.timeout_seconds);
    match cli.command {
        Command::Login => login_once(&config, &client),
        Command::Watch => watch(&config, &client),
        Command::Status => status(&config, &client),
        Command::Logout => logout(&config, &client),
    }
}

fn perform_login(config: &Config, client: &PortalClient) -> Result<SessionState> {
    let (username, password) = config.credentials()?;
    let info = client
        .login(
            &username,
            &password,
            &config.uplcyid,
            config.language,
            config.check_captcha,
        )
        .context("登录请求失败")?;
    if info.must_change_password {
        eprintln!("警告：服务器要求首次登录后修改密码，请使用浏览器处理。");
    }
    Ok(SessionState::from_login(info))
}

fn login_once(config: &Config, client: &PortalClient) -> Result<()> {
    let state = perform_login(config, client)?;
    save_or_warn(config, &state);
    println!(
        "登录成功：用户 {}，IP {}，建议 {} 秒后保活",
        state.username, state.login_ip, state.keepalive_interval
    );
    Ok(())
}

fn watch(config: &Config, client: &PortalClient) -> Result<()> {
    let state_path = config.state_path();
    let mut session = match state::load(&state_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("警告：忽略损坏或不可读的会话文件：{error:#}");
            None
        }
    };
    let mut delay = Duration::ZERO;
    let mut keepalive_failures = 0_u32;

    println!("watch 模式已启动；按 Ctrl+C 停止。");
    loop {
        if !delay.is_zero() {
            thread::sleep(delay);
        }

        if session.is_none() {
            match perform_login(config, client) {
                Ok(new_session) => {
                    println!(
                        "登录成功：用户 {}，IP {}",
                        new_session.username, new_session.login_ip
                    );
                    delay = keepalive_delay(config, new_session.keepalive_interval);
                    save_or_warn(config, &new_session);
                    session = Some(new_session);
                    keepalive_failures = 0;
                }
                Err(error) => {
                    eprintln!("登录失败：{error:#}；{} 秒后重试。", config.retry_seconds);
                    delay = Duration::from_secs(config.retry_seconds);
                }
            }
            continue;
        }

        let secret = session
            .as_ref()
            .expect("session checked above")
            .secret
            .clone();
        match client.keepalive(&secret) {
            Ok(info) => {
                let current = session.as_mut().expect("session checked above");
                current.update_from_keepalive(info);
                println!(
                    "保活成功：IP {}，在线 {} 秒，下次间隔 {} 秒",
                    current.login_ip, current.online_duration, current.keepalive_interval
                );
                delay = keepalive_delay(config, current.keepalive_interval);
                save_or_warn(config, current);
                keepalive_failures = 0;
            }
            Err(PortalError::Offline(message)) => {
                eprintln!("会话已离线：{message}；准备重新登录。");
                let _ = state::remove(&state_path);
                session = None;
                keepalive_failures = 0;
                delay = Duration::from_secs(config.retry_seconds);
            }
            Err(error) => {
                keepalive_failures += 1;
                eprintln!(
                    "保活失败（{}/{}）：{}",
                    keepalive_failures, config.max_keepalive_failures, error
                );
                if keepalive_failures >= config.max_keepalive_failures {
                    eprintln!("连续保活失败，丢弃旧会话并准备重新登录。");
                    let _ = state::remove(&state_path);
                    session = None;
                    keepalive_failures = 0;
                }
                delay = Duration::from_secs(config.retry_seconds);
            }
        }
    }
}

fn status(config: &Config, client: &PortalClient) -> Result<()> {
    let state_path = config.state_path();
    let Some(mut session) = state::load(&state_path)? else {
        println!("没有已保存的会话；当前状态未知。");
        return Ok(());
    };

    match client.keepalive(&session.secret) {
        Ok(info) => {
            session.update_from_keepalive(info);
            state::save(&state_path, &session)?;
            println!(
                "在线：用户 {}，IP {}，在线 {} 秒",
                session.username, session.login_ip, session.online_duration
            );
            Ok(())
        }
        Err(PortalError::Offline(message)) => {
            state::remove(&state_path)?;
            println!("离线：{message}");
            Ok(())
        }
        Err(error) => Err(error).context("无法确认在线状态"),
    }
}

fn logout(config: &Config, client: &PortalClient) -> Result<()> {
    let state_path = config.state_path();
    let session =
        state::load(&state_path)?.context("没有已保存的会话，无法取得注销所需的 secret")?;
    client
        .logout(&session.username, &session.secret, config.language)
        .context("注销请求失败")?;
    state::remove(&state_path)?;
    println!("注销成功：用户 {}", session.username);
    Ok(())
}

fn keepalive_delay(config: &Config, server_interval: u64) -> Duration {
    let interval = if server_interval == 0 {
        300
    } else {
        server_interval
    };
    Duration::from_secs(
        interval
            .saturating_sub(config.keepalive_margin_seconds)
            .max(5),
    )
}

fn save_or_warn(config: &Config, session: &SessionState) {
    if let Err(error) = state::save(&config.state_path(), session) {
        eprintln!("警告：会话已建立，但状态文件保存失败：{error:#}");
    }
}
