# Build and release

## Native build

Install stable Rust, then run:

```sh
cargo test --locked
cargo build --locked --release
```

The executable is under `target/release/`.

`Cargo.lock` is committed because this is an executable application; keep it in version control so local and GitHub release builds resolve the same dependencies.

## Cross-compiling Linux from WSL

WSL is useful but not mandatory. Install a Linux distribution, Docker, Rust and `cross`, then:

```sh
cargo install cross --locked
cross build --locked --release --target x86_64-unknown-linux-musl
cross build --locked --release --target i686-unknown-linux-musl
cross build --locked --release --target aarch64-unknown-linux-musl
cross build --locked --release --target armv7-unknown-linux-musleabihf
cross build --locked --release --target arm-unknown-linux-musleabi
cross build --locked --release --target arm-unknown-linux-musleabihf
```

`cross` runs target toolchains in containers. A 64-bit Windows or x86-64 Linux host can produce AArch64 output; it simply cannot execute that output locally without emulation or matching hardware.

## Windows targets

With Visual Studio C++ build tools installed:

```powershell
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc aarch64-pc-windows-msvc
cargo build --locked --release --target x86_64-pc-windows-msvc
cargo build --locked --release --target i686-pc-windows-msvc
cargo build --locked --release --target aarch64-pc-windows-msvc
```

The ARM64 Windows target is unrelated to AArch64 OpenWrt; each uses a different operating-system ABI.

## macOS targets

Apple SDK linking must run on macOS, which is one reason to use GitHub Actions:

```sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --locked --release --target x86_64-apple-darwin
cargo build --locked --release --target aarch64-apple-darwin
```

## GitHub Actions

The repository contains two workflows:

- `ci.yml`: formatting, Clippy and tests on pushes and pull requests.
- `release.yml`: a manual run builds every target without publishing; a `v*` tag additionally creates the GitHub Release.

Create a repository using the contents of the `hzii-net` directory, then update the `repository` URL in `Cargo.toml`:

```sh
git init
git add .
git commit -m "Initial release"
git branch -M main
git remote add origin https://github.com/YOUR_NAME/hzii-net.git
git push -u origin main
```

Open `Actions` → `Release` → `Run workflow` once before the first tag. This exercises all 11 target builds and keeps the packages as workflow artifacts without creating a public Release.

Before releasing, make the version in these files agree:

- `Cargo.toml`
- `CHANGELOG.md`

Then push a tag:

```sh
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

In repository settings, ensure GitHub Actions is allowed to create releases (`Settings` → `Actions` → `General` → `Workflow permissions` → read and write). The workflow also declares `contents: write` for its release job.

The workflow uses GitHub-hosted Windows and macOS runners plus Linux containers through `cross`; no WSL or local cross toolchain is required for official releases.

## Why there is no TLS feature

The observed Portal is HTTP-only. `ureq` is built with `default-features = false`, so release builds do not include a TLS backend or OpenSSL. If the institute migrates to HTTPS, TLS support and certificate behavior must be designed and tested instead of merely changing the URL.
