# Roadmap

## Phase 0 — Native foundation

- [x] Cargo workspace
- [x] GTK4/Libadwaita application shell
- [x] Navigation structure
- [x] `genixpkgd` D-Bus skeleton
- [x] Package-name validation tests
- [x] Desktop, AppStream, systemd and PolicyKit metadata
- [x] CI foundation

## Phase 1 — Read-only package model

- [ ] Package and application domain models
- [ ] APT cache reader
- [ ] dpkg installed-state reader
- [ ] AppStream catalogue ingestion
- [ ] Installed packages page
- [ ] Package detail page
- [ ] Search and category filtering
- [ ] System health summary

## Phase 2 — Transaction framework

- [ ] PolicyKit authorization helper
- [ ] Serialized transaction queue
- [ ] D-Bus progress, log and completion signals
- [ ] Cancellation rules
- [ ] Transaction journal
- [ ] Install, remove and upgrade previews
- [ ] Integration tests in disposable containers or virtual machines

## Phase 3 — Safe APT operations

- [ ] Install packages
- [ ] Remove packages
- [ ] Upgrade selected packages
- [ ] Upgrade all packages
- [ ] Repository refresh
- [ ] Dependency and disk-space preview
- [ ] Conffile and reboot-required handling
- [ ] Recovery guidance for interrupted dpkg state

## Phase 4 — Product features

- [ ] Dashboard
- [ ] Discover catalogue
- [ ] Updates
- [ ] Software Stacks
- [ ] Security advisories
- [ ] Approved systemd services
- [ ] System Profiles export, comparison and restore
- [ ] Activity history
- [ ] Settings and offline controls

## Phase 5 — GenixBit OS integration

- [ ] Branded icon and design tokens
- [ ] Default installation in the GenixBit OS image
- [ ] Repository signing validation
- [ ] Release and rollback testing
- [ ] Accessibility and keyboard-navigation audit
- [ ] Translations
- [ ] Stable D-Bus API versioning

## Phase 6 — Future GenixPkg backend

- [ ] Define native GenixPkg package metadata
- [ ] Implement backend behind the existing service contract
- [ ] Migration and coexistence strategy
- [ ] Atomic transaction and rollback support
