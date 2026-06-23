# Changelog

## 0.1.1

- Treat captcha responses without `verify` (for example `{"result":"fail"}`) as "no captcha required", matching the browser implementation.

## 0.1.0

- Initial login, keepalive, status, logout, and automatic reconnect support.
- Linux musl, Windows, and macOS release matrix.
- systemd, OpenWrt/ImmortalWrt, and Windows startup examples.
