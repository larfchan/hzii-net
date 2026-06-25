#!/bin/sh
#
# Interactive installer for OpenWrt / ImmortalWrt.
#
# Usage on router:
#   mkdir -p /tmp/hzii-net-install
#   # Upload these two files into /tmp/hzii-net-install:
#   #   1. hzii-net        - the router-matching executable
#   #   2. hzii-net.toml   - your edited config, or config.toml
#   sh /tmp/hzii-net-install/install-immortalwrt.sh
#
# Optional environment variables:
#   STAGING_DIR=/tmp/hzii-net-install
#   SRC_BIN=/tmp/hzii-net-install/hzii-net
#   SRC_CONFIG=/tmp/hzii-net-install/hzii-net.toml

set -eu

APP_NAME="hzii-net"

STAGING_DIR="${STAGING_DIR:-/tmp/hzii-net-install}"
SRC_BIN="${SRC_BIN:-$STAGING_DIR/$APP_NAME}"
SRC_CONFIG_INPUT="${SRC_CONFIG:-}"
SRC_CONFIG="$SRC_CONFIG_INPUT"

DEST_BIN="/usr/bin/$APP_NAME"
DEST_CONFIG="/etc/$APP_NAME.toml"
DEST_INIT="/etc/init.d/$APP_NAME"
STATE_FILE="/tmp/$APP_NAME-session.json"

TMP_CONFIG="/tmp/$APP_NAME-config.$$"

cleanup() {
    rm -f "$TMP_CONFIG"
}
trap cleanup EXIT

say() {
    printf '%s\n' "$*"
}

hr() {
    say "------------------------------------------------------------"
}

fail() {
    say "错误：$*" >&2
    exit 1
}

need_root() {
    if [ "$(id -u)" != "0" ]; then
        fail "请用 root 用户在路由器上运行本脚本。"
    fi
}

pick_config() {
    if [ -n "$SRC_CONFIG_INPUT" ]; then
        SRC_CONFIG="$SRC_CONFIG_INPUT"
        return
    fi

    if [ -f "$STAGING_DIR/$APP_NAME.toml" ]; then
        SRC_CONFIG="$STAGING_DIR/$APP_NAME.toml"
    elif [ -f "$STAGING_DIR/config.toml" ]; then
        SRC_CONFIG="$STAGING_DIR/config.toml"
    else
        SRC_CONFIG="$STAGING_DIR/$APP_NAME.toml"
    fi
}

print_banner() {
    hr
    say "hzii-net ImmortalWrt / OpenWrt 交互式安装脚本"
    hr
    say ""
    say "这个脚本会做这些事："
    say "  1) 从临时目录读取你上传的可执行文件和配置文件；"
    say "  2) 自动设置上传文件权限：程序 755，配置 600；"
    say "  3) 安装可执行文件到：$DEST_BIN"
    say "  4) 安装配置文件到：$DEST_CONFIG"
    say "  5) 自动把会话状态写到内存文件：$STATE_FILE"
    say "  6) 写入 procd 启动脚本：$DEST_INIT"
    say "  7) 启用开机自启，并立刻启动 watch 模式。"
    say ""
    say "注意："
    say "  - /tmp 只是中转目录，重启会消失；"
    say "  - 程序和配置会放到 /usr/bin、/etc，重启后仍在；"
    say "  - session 状态放在 /tmp，避免 keepalive 频繁写路由器闪存；"
    say "  - config 里会保存校园网密码，请确保 /etc/$APP_NAME.toml 权限为 600。"
    say ""
}

print_prepare_steps() {
    pick_config

    say "请先确认你已经把必要文件上传到了路由器："
    say ""
    say "  目录：$STAGING_DIR"
    say "  可执行文件：$SRC_BIN"
    say "  配置文件：$SRC_CONFIG"
    say ""
    say "配置文件至少要改好："
    say '  username = "你的学号"'
    say '  password = "你的密码"'
    say ""
    say "如果你用的是 64 位 ARM 路由器，通常下载："
    say "  hzii-net-*-aarch64-unknown-linux-musl.tar.xz"
    say ""
    say "如果不确定架构，可以在路由器上执行："
    say "  uname -m"
    say ""
}

wait_for_files() {
    while :; do
        pick_config

        missing=0
        [ -f "$SRC_BIN" ] || missing=1
        [ -f "$SRC_CONFIG" ] || missing=1

        if [ "$missing" = "0" ]; then
            say "已找到："
            ls -l "$SRC_BIN" "$SRC_CONFIG"
            say ""
            return
        fi

        say "还没有找到完整文件。当前检测结果："
        if [ -f "$SRC_BIN" ]; then
            say "  [OK]   $SRC_BIN"
        else
            say "  [缺少] $SRC_BIN"
        fi

        if [ -f "$SRC_CONFIG" ]; then
            say "  [OK]   $SRC_CONFIG"
        else
            say "  [缺少] $SRC_CONFIG"
            say "         也可以命名为：$STAGING_DIR/config.toml"
        fi

        say ""
        say "请通过 WinSCP / scp / SFTP 把文件放好。"
        printf "放好后按 Enter 继续检查；输入 q 回车退出："
        IFS= read -r answer || exit 1
        [ "$answer" = "q" ] && exit 0
        say ""
    done
}

normalize_source_permissions() {
    say "正在设置上传文件权限："
    say "  chmod 755 $SRC_BIN"
    say "  chmod 600 $SRC_CONFIG"
    chmod 755 "$SRC_BIN"
    chmod 600 "$SRC_CONFIG"
    say ""
}

prepare_config() {
    tr -d '\r' < "$SRC_CONFIG" > "$TMP_CONFIG"

    if grep -Eq '^[[:space:]]*username[[:space:]]*=[[:space:]]*"YOUR_STUDENT_ID"' "$TMP_CONFIG"; then
        fail "配置文件 username 还是示例值，请先改成你的学号。"
    fi

    if grep -Eq '^[[:space:]]*password[[:space:]]*=[[:space:]]*"YOUR_PASSWORD"' "$TMP_CONFIG"; then
        fail "配置文件 password 还是示例值，请先改成你的校园网密码。"
    fi

    if ! grep -Eq '^[[:space:]]*state_file[[:space:]]*=' "$TMP_CONFIG"; then
        {
            say ""
            say "# OpenWrt / ImmortalWrt: keep session state in RAM, not flash."
            say "state_file = \"$STATE_FILE\""
        } >> "$TMP_CONFIG"
    fi
}

write_init_script() {
    cat > "$DEST_INIT" <<'EOF'
#!/bin/sh /etc/rc.common

USE_PROCD=1
START=95
STOP=10

start_service() {
    procd_open_instance
    procd_set_param command /usr/bin/hzii-net --config /etc/hzii-net.toml watch
    procd_set_param respawn 3600 5 5
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_close_instance
}

service_triggers() {
    procd_add_reload_trigger network
}
EOF
    chmod 755 "$DEST_INIT"
}

confirm_install() {
    hr
    say "即将正式安装："
    say "  $SRC_BIN    -> $DEST_BIN"
    say "  $SRC_CONFIG -> $DEST_CONFIG"
    say "  写入启动项：$DEST_INIT"
    say ""
    say "安装后会立即执行："
    say "  $DEST_INIT enable"
    say "  $DEST_INIT start"
    say ""
    printf "确认继续请输入 YES："
    IFS= read -r confirm || exit 1
    [ "$confirm" = "YES" ] || fail "用户取消安装。"
}

install_files() {
    if [ -x "$DEST_INIT" ]; then
        "$DEST_INIT" stop >/dev/null 2>&1 || true
    fi

    mkdir -p /usr/bin /etc

    cp "$SRC_BIN" "$DEST_BIN"
    chmod 755 "$DEST_BIN"

    if ! "$DEST_BIN" --version >/dev/null 2>&1; then
        say ""
        say "无法运行 $DEST_BIN --version。"
        say "这通常说明你下载的架构不匹配，或者文件没有正确解压。"
        say "当前路由器架构：$(uname -m)"
        fail "请换成匹配当前路由器架构的 release 包后重试。"
    fi

    cp "$TMP_CONFIG" "$DEST_CONFIG"
    chmod 600 "$DEST_CONFIG"

    write_init_script
}

start_service() {
    "$DEST_INIT" enable
    "$DEST_INIT" start
}

print_result() {
    say ""
    hr
    say "安装命令已执行完。下面是几个检查命令的输出："
    hr
    say ""

    say "程序版本："
    "$DEST_BIN" --version || true
    say ""

    say "启动脚本状态："
    "$DEST_INIT" status || true
    say ""

    say "进程："
    ps | grep '[h]zii-net' || true
    say ""

    say "最近日志："
    if command -v logread >/dev/null 2>&1; then
        logread -e "$APP_NAME" | tail -n 40 || true
    else
        say "当前系统没有 logread 命令。"
    fi

    say ""
    hr
    say "常用命令："
    say "  查看日志：logread -e $APP_NAME"
    say "  查看状态：$DEST_INIT status"
    say "  重启服务：$DEST_INIT restart"
    say "  停止服务：$DEST_INIT stop"
    say "  禁用自启：$DEST_INIT disable"
    say "  手动状态：$DEST_BIN --config $DEST_CONFIG status"
    say "  手动注销：$DEST_BIN --config $DEST_CONFIG logout"
    hr
}

main() {
    need_root
    print_banner
    print_prepare_steps
    wait_for_files
    normalize_source_permissions
    prepare_config
    confirm_install
    install_files
    start_service
    sleep 2
    print_result
}

main "$@"
