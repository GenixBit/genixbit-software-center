# Architecture

## Goals

GenixBit Software Center provides a trustworthy graphical control plane for software on GenixBit OS. The UI is unprivileged. Package database changes are owned by a small system service with a stable D-Bus contract.

## Components

```text
┌─────────────────────────────────────────────────────────────┐
│ GenixBit Software Center                                   │
│ Rust + GTK4 + Libadwaita                                   │
│ Dashboard · Discover · Installed · Updates · Security      │
└───────────────────────────┬─────────────────────────────────┘
                            │ typed D-Bus calls and signals
┌───────────────────────────▼─────────────────────────────────┐
│ genixpkgd                                                   │
│ authorization · transaction queue · validation · audit log │
└───────────────┬───────────────────────┬─────────────────────┘
                │                       │
       ┌────────▼────────┐     ┌────────▼────────┐
       │ APT/dpkg backend│     │ systemd backend │
       │ GenixBit OS v1  │     │ approved units │
       └─────────────────┘     └─────────────────┘
```

## Trust boundaries

### Desktop application

The desktop application may read public catalogue data and request typed operations. It must not:

- execute arbitrary commands;
- invoke `sudo`;
- write to `/var/lib/dpkg`, `/etc/apt` or trusted key directories;
- accept a shell fragment as an API argument;
- store privileged credentials.

### `genixpkgd`

The daemon owns privileged transactions. It must:

- validate all arguments again, regardless of UI validation;
- serialize package transactions;
- authorize each modifying operation through PolicyKit;
- use argument arrays rather than a shell;
- provide progress and completion signals;
- keep an append-only transaction journal;
- fail closed on repository, signature or metadata errors.

## Backend abstraction

The daemon will expose a package-backend trait so GenixBit can initially use APT/dpkg and later add GenixPkg without changing the UI or D-Bus API.

```rust
trait PackageBackend {
    async fn list_installed(&self) -> Result<Vec<Package>>;
    async fn search(&self, query: &str) -> Result<Vec<Package>>;
    async fn check_updates(&self) -> Result<Vec<Update>>;
    async fn install(&self, packages: &[PackageId]) -> Result<TransactionId>;
    async fn remove(&self, packages: &[PackageId]) -> Result<TransactionId>;
    async fn upgrade(&self, packages: &[PackageId]) -> Result<TransactionId>;
}
```

The trait above is a design contract; it is not yet part of the compiled public API.

## Package transaction lifecycle

1. UI requests an operation.
2. Daemon validates package identifiers and operation constraints.
3. Daemon requests the relevant PolicyKit authorization.
4. Daemon creates a transaction identifier and queues the work.
5. Backend refreshes and verifies package metadata where required.
6. Backend performs the operation without a shell.
7. Daemon emits structured progress signals.
8. Daemon records completion, errors and changed packages.
9. UI refreshes installed and update state.

## Data sources

The first implementation will use:

- APT repository metadata for system packages;
- AppStream metadata for application descriptions, icons and screenshots;
- dpkg status for installed package state;
- Ubuntu and GenixBit security advisories;
- OSV as optional enrichment;
- systemd D-Bus APIs for approved service operations.

## Packaging

Production artifacts are `.deb` packages integrated into the GenixBit OS image. Portable builds can be evaluated later, but they cannot replace the installed system daemon and PolicyKit integration.
