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

Phase 1 is feature-complete and keeps all package-changing operations disabled.

- [x] Shared package, update and application domain models
- [x] APT available-update reader
- [x] dpkg installed-state reader
- [x] Local AppStream catalogue search
- [x] Installed packages page
- [x] Updates page
- [x] AppStream search page
- [x] System health dashboard
- [x] Package detail window
- [x] Installed-package search and Debian-section filtering
- [x] AppStream category metadata and result filtering
- [x] Package origin, candidate version and update status
- [x] Featured AppStream collection metadata and D-Bus API
- [x] Bounded paginated AppStream service and client API
- [x] Featured collection browser in GTK
- [x] Pagination or virtualized lists in GTK for large result sets

## Phase 2 — Transaction framework

The foundation slice is in progress. Package execution remains disabled and protected transaction controls fail closed until caller-aware PolicyKit verification is connected.

Foundation completed in the current slice:

- [x] Typed transaction preview, change, record and queue-snapshot models
- [x] Fail-closed authorization boundary with an explicit session-test override
- [x] Serialized pending queue with deterministic ordering
- [x] Append-only transaction journal with persistence tests
- [x] D-Bus preview, queue inspection, journal inspection and cancellation APIs
- [x] Install, remove and upgrade metadata previews without package execution

Remaining Phase 2 completion criteria:

- [ ] Caller-aware PolicyKit authorization helper
- [ ] Active transaction runner on the serialized queue
- [ ] D-Bus progress, log and completion signals
- [ ] Cancellation rules for active package-manager subprocesses
- [ ] APT dependency, download-size and disk-space simulation
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

- [ ] Full dashboard
- [ ] Curated Discover catalogue
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
