# GenixBit Software Center

A native Linux software, update, security, service and system-profile manager for **GenixBit OS**.

The project is a clean Rust/GTK4 implementation inspired by the product scope of Brew Browser, but it is not a web browser and it does not use Tauri, Svelte, JavaScript or a WebView.

## Architecture

- **Desktop application:** Rust + GTK4 + Libadwaita
- **System service:** `genixpkgd`, written in Rust
- **IPC:** typed D-Bus interface at `com.genixbit.PackageManager1`
- **Authorization:** PolicyKit for future privileged transactions
- **Initial backend:** APT/dpkg on the Ubuntu-based GenixBit OS image
- **Application metadata:** local AppStream component pool
- **Future backend:** native GenixPkg implementation behind the same D-Bus contract
- **Service management:** systemd
- **Primary package format:** `.deb`

The graphical application never launches `sudo`, never accepts arbitrary shell commands and never directly changes the package database. All package operations cross the narrow D-Bus boundary exposed by `genixpkgd`.

## Current status — 0.2.0 read-only milestone

Implemented:

- Native GTK4/Libadwaita application shell and navigation
- Shared, typed package/application D-Bus models
- Installed package discovery from `/var/lib/dpkg/status`
- Available-update discovery through read-only APT output
- Local AppStream catalogue search
- Dashboard counts for installed packages, updates and security updates
- Functional Installed, Updates and Discover pages
- Parser and input-validation unit tests
- PolicyKit, systemd, desktop and AppStream metadata
- GitHub Actions formatting, checks, tests and Clippy

Not implemented yet:

- Install, remove or upgrade transactions
- Repository refresh
- Package detail pages and dependency previews
- Transaction authorization, progress, journaling or rollback
- Security advisories, services, software stacks and system profiles

All modifying D-Bus methods deliberately return `NotSupported` until the transaction framework is designed and tested.

## Build requirements

On Ubuntu or GenixBit OS:

```bash
sudo apt update
sudo apt install -y \
  appstream build-essential pkg-config \
  libgtk-4-dev libadwaita-1-dev libdbus-1-dev
```

Install a current stable Rust toolchain.

### Safe development mode

Run the daemon on the user session bus:

```bash
GENIXPKGD_BUS=session cargo run -p genixpkgd
```

In another terminal, launch the application against the same bus:

```bash
GENIXPKGD_BUS=session cargo run -p genixbit-software-center
```

The production service uses the system bus and must be installed with the supplied systemd, D-Bus and PolicyKit definitions.

## Read-only data sources

- `/var/lib/dpkg/status` for installed package state
- `apt list --upgradable` for available update metadata
- `appstreamcli search` for applications in the local AppStream pool

Commands are executed directly with fixed argument arrays. The project never constructs shell command strings from user input.

## Repository layout

```text
crates/package-model/     Shared D-Bus data types
crates/software-center/   Native GTK4 desktop application
crates/genixpkgd/         Package-management system service
dbus/                     Public D-Bus interface contract
polkit/                   Authorization policy
systemd/                  System service unit
data/                     Desktop and AppStream metadata
docs/                     Architecture, security model and roadmap
```

## Security principles

- Typed operations only; no general shell execution endpoint
- Package identifiers validated at the daemon boundary
- PolicyKit authorization required before future modifying actions
- Read-only operations separated from privileged transactions
- Signed repositories and package verification required before transactions
- Transaction logs and user-visible progress
- Fail closed when metadata, signatures or authorization are invalid

## License and attribution

GenixBit Software Center is licensed under the MIT License. See `LICENSE` and `ATTRIBUTION.md`.
