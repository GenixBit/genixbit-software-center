# GenixBit Software Center

A native Linux software, update, security, service and system-profile manager for **GenixBit OS**.

The project is a clean Rust/GTK4 implementation inspired by the product scope of Brew Browser, but it is not a web browser and it does not use Tauri, Svelte, JavaScript or a WebView.

## Foundation architecture

- **Desktop application:** Rust + GTK4 + Libadwaita
- **Privileged service:** `genixpkgd`, written in Rust
- **IPC:** typed D-Bus interface at `com.genixbit.PackageManager1`
- **Authorization:** PolicyKit rules for privileged package transactions
- **Initial backend:** APT/dpkg on the Ubuntu-based GenixBit OS image
- **Future backend:** native GenixPkg implementation behind the same D-Bus contract
- **Service management:** systemd
- **Primary package format:** `.deb`

The graphical application never launches `sudo`, never accepts arbitrary shell commands and never directly changes the package database. All privileged operations must cross the narrow D-Bus and PolicyKit boundary exposed by `genixpkgd`.

## Planned sections

1. Dashboard
2. Discover
3. Installed
4. Updates
5. Software Stacks
6. Security
7. Services
8. System Profiles
9. Activity
10. Settings

## Current status

The `0.1.0` foundation contains:

- Cargo workspace
- Native GTK4/Libadwaita application shell
- Initial navigation and page structure
- D-Bus service skeleton
- Strict Debian package-name validation
- PolicyKit, systemd, desktop and AppStream metadata
- Architecture and phased implementation roadmap
- GitHub Actions checks

Package transactions are deliberately disabled until the APT transaction layer, PolicyKit authorization checks, progress signals and rollback behaviour are implemented and tested.

## Build requirements

On Ubuntu or GenixBit OS:

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config libgtk-4-dev libadwaita-1-dev \
  libdbus-1-dev
```

Install a current stable Rust toolchain, then run:

```bash
cargo run -p genixbit-software-center
```

For safe D-Bus development on the user session bus:

```bash
GENIXPKGD_BUS=session cargo run -p genixpkgd
```

The production service uses the system bus and must be installed with the supplied systemd, D-Bus and PolicyKit definitions.

## Repository layout

```text
crates/software-center/   Native GTK4 desktop application
crates/genixpkgd/         Privileged package-management service
dbus/                     Public D-Bus interface contract
polkit/                   Authorization policy
systemd/                  System service unit
data/                     Desktop and AppStream metadata
docs/                     Architecture, security model and roadmap
```

## Security principles

- Typed operations only; no general shell execution endpoint
- Package identifiers validated at the daemon boundary
- PolicyKit authorization for every modifying action
- Read-only operations separated from privileged transactions
- Signed repositories and package verification required
- Transaction logs and user-visible progress
- Fail closed when metadata, signatures or authorization are invalid

## License and attribution

GenixBit Software Center is licensed under the MIT License. See `LICENSE` and `ATTRIBUTION.md`.
