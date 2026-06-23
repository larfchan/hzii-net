# OpenWrt / ImmortalWrt installation

## 1. Choose the architecture

```sh
uname -m
opkg print-architecture 2>/dev/null || true
```

Typical mapping:

- `aarch64` / `arm64`: `aarch64-unknown-linux-musl`
- `armv7l`: `armv7-unknown-linux-musleabihf`
- `x86_64`: `x86_64-unknown-linux-musl`
- old 32-bit ARM: determine whether the firmware uses soft-float (`musleabi`) or hard-float (`musleabihf`)

OpenWrt's package architecture name (for example `aarch64_cortex-a53`) can be more specific than Rust's target name. A statically linked musl executable normally only needs the CPU family and kernel ABI to match.

## 2. Install files

Copy the unpacked executable and configuration to the router:

```sh
scp hzii-net root@192.168.1.1:/usr/bin/hzii-net
scp config.toml root@192.168.1.1:/etc/hzii-net.toml
scp packaging/openwrt/hzii-net.init root@192.168.1.1:/etc/init.d/hzii-net
```

On the router:

```sh
chmod 0755 /usr/bin/hzii-net /etc/init.d/hzii-net
chmod 0600 /etc/hzii-net.toml
/etc/init.d/hzii-net enable
/etc/init.d/hzii-net start
logread -e hzii-net
```

如需注销，先停止服务，避免 `watch` 自动重新登录：

```sh
/etc/init.d/hzii-net stop
/usr/bin/hzii-net --config /etc/hzii-net.toml logout
```

Use a RAM-backed state file to protect flash storage:

```toml
state_file = "/tmp/hzii-net-session.json"
```

The authentication request must leave through the campus-facing interface. On ordinary single-WAN routers the routing table already ensures this. Multi-WAN users must add an appropriate policy route; authenticating from a LAN server generally authorizes that server's source IP, not the router's campus-facing IP.
