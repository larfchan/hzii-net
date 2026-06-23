# hzii-net

北航杭州创新研究院校园网 Portal 的无界面登录与保活客户端。适用于 Windows、Linux、macOS，以及运行 OpenWrt/ImmortalWrt 的路由器。

> 本项目仅用于自动登录你有权使用的校园网账号。请遵守学校网络管理规定。

## 功能

- `login`：登录一次并保存会话令牌。
- `watch`：常驻运行，按服务器返回的间隔保活，掉线后自动重新登录。
- `status`：使用本地会话令牌检查状态。
- `logout`：注销当前会话。
- 自动处理 Portal 返回的图形验证码字符，不需要 OCR。
- 不依赖浏览器、Node.js、Python 或 OpenSSL。
- GitHub Actions 自动生成多平台 Release。

## 快速开始

从 [Releases](../../releases) 下载与你设备匹配的压缩包，然后创建配置：

```sh
cp config.example.toml config.toml
```

修改 `username` 和 `password` 后运行：

```sh
# 推荐：常驻登录、保活和断线重连
./hzii-net --config config.toml watch

# 其他命令
./hzii-net --config config.toml login
./hzii-net --config config.toml status
./hzii-net --config config.toml logout
```

执行 `logout` 前应先停止正在运行的 `watch` 或系统服务，否则常驻进程会把注销识别为掉线并自动重新登录。

也可以通过环境变量提供敏感字段，环境变量优先于配置文件：

```sh
export HZII_NET_USERNAME='学号'
export HZII_NET_PASSWORD='密码'
./hzii-net --config config.toml watch
```

PowerShell 对应写法：

```powershell
$env:HZII_NET_USERNAME = '学号'
$env:HZII_NET_PASSWORD = '密码'
.\hzii-net.exe --config .\config.toml watch
```

## 下载哪个版本

在 Linux/OpenWrt 上先执行：

```sh
uname -m
```

| `uname -m` 常见输出 | Release 目标 | 说明 |
|---|---|---|
| `x86_64` | `x86_64-unknown-linux-musl` | 64 位 Intel/AMD Linux、x86 软路由 |
| `i386`、`i486`、`i586`、`i686` | `i686-unknown-linux-musl` | 32 位 x86 Linux |
| `aarch64`、`arm64` | `aarch64-unknown-linux-musl` | 64 位 ARM 路由器/服务器 |
| `armv7l` | `armv7-unknown-linux-musleabihf` | 32 位 ARMv7，硬浮点 |
| `armv6l` 或较老的 `arm` | `arm-unknown-linux-musleabi(hf)` | 需根据固件软/硬浮点 ABI 选择 |

Windows 和 macOS 的包名同样包含目标三元组。AArch64 是 64 位 ARM；Release 名称中的 `arm` 通常指 32 位 ARM，两者不能混用。

## 后台运行

- Linux/systemd：[systemd 安装说明](docs/systemd.md)
- OpenWrt/ImmortalWrt：[路由器安装说明](docs/openwrt.md)
- Windows：[开机任务说明](docs/windows.md)

系统服务只负责开机启动和进程崩溃后重启。登录、451 秒左右的周期保活和掉线重连都由 `watch` 命令内部完成，不需要 cron 反复执行登录。

## 本地构建与 GitHub Actions

参见 [构建和发布指南](docs/build-and-release.md)。在 64 位 Windows 上可以生成 ARM/AArch64 程序；“构建机器架构”和“目标程序架构”不是一回事。Linux ARM 的链接环境用 WSL + `cross` 会更方便，但发布版本可以完全交给 GitHub Actions。

## 安全说明

Portal 使用 HTTP，并在浏览器端用公开的固定 AES 密钥处理密码。这只能隐藏肉眼可见的明文，不能抵抗抓包或请求重放。

- 不要公开 HAR 文件；HAR 应按明文密码处理。
- 不要把真实 `config.toml` 提交到 Git。
- Linux/OpenWrt 上建议执行 `chmod 600 config.toml`。
- 会话文件包含可用于保活和注销的 `secret`，默认写入系统临时目录。

更多说明见 [SECURITY.md](SECURITY.md)。

## 协议与兼容性

当前实现针对 `spos httpd/4.8` Portal 的以下接口：

- `/user_auth_verify.cgi`
- `/portal.cgi`
- `/keepalive.cgi`
- `/logout.cgi`

它不是深澜 `/cgi-bin/srun_portal` 协议。其他学校即使页面相似，也应先核对请求和响应格式。协议记录见 [docs/protocol.md](docs/protocol.md)。

## 开源许可

[MIT](LICENSE)
