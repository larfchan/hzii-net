# systemd installation

Example paths:

```sh
sudo install -m 0755 hzii-net /usr/local/bin/hzii-net
sudo useradd --system --home /nonexistent --shell /usr/sbin/nologin hzii-net
sudo install -d -o hzii-net -g hzii-net -m 0700 /etc/hzii-net
sudo install -o hzii-net -g hzii-net -m 0600 config.toml /etc/hzii-net/config.toml
sudo install -m 0644 packaging/systemd/hzii-net.service /etc/systemd/system/hzii-net.service
sudo systemctl daemon-reload
sudo systemctl enable --now hzii-net
```

Set this in `/etc/hzii-net/config.toml`:

```toml
state_file = "/run/hzii-net/session.json"
```

Inspect logs and status:

```sh
systemctl status hzii-net
journalctl -u hzii-net -f
```

如需注销，先停止常驻服务，再以服务账号执行命令：

```sh
sudo systemctl stop hzii-net
sudo -u hzii-net /usr/local/bin/hzii-net --config /etc/hzii-net/config.toml logout
```
