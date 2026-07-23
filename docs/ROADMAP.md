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

The protected transaction foundation is active. Caller-aware PolicyKit verification is connected, but real package execution remains disabled until the subprocess runner and active-cancellation rules are completed and tested.

Foundation completed:

- [x] Typed transaction preview, change, record, event and queue-snapshot models
- [x] Fail-closed authorization boundary with an explicit session-test override
- [x] Caller-aware PolicyKit authorization using the authenticated D-Bus sender
- [x] Serialized pending queue with deterministic ordering
- [x] Append-only transaction journal with persistence tests
- [x] D-Bus preview, queue inspection, event history, journal inspection and cancellation APIs
- [x] Ordered D-Bus lifecycle signal for preview, queue and cancellation changes
- [x] Bounded cursor-based transaction event history
- [x] Install, remove and upgrade metadata previews without package execution
- [x] APT dependency, download-size and disk-space simulation
- [x] Simulation-only active runner on the serialized queue
- [x] Simulated running, progress and completion event emission
- [x] Integration tests in a disposable Ubuntu container

Remaining Phase 2 completion criteria:

- [ ] Real package transaction runner on the serialized queue
- [ ] Active package-manager subprocess progress and log parsing
- [ ] Cancellation rules for active package-manager subprocesses

## Phase 3 — Safe APT operations

- [ ] Install packages
- [ ] Remove packages
- [ ] Upgrade selected packages
- [ ] Upgrade all packages
- [ ] Repository refresh
- [x] Dependency and disk-space preview
- [ ] Conffile and reboot-required handling
- [ ] Recovery guidance for interrupted dpkg state

## Phase 4 — Product features

- [x] Full dashboard
- [x] Curated Discover catalogue
- [x] Software Stacks
- [ ] Security advisories
- [x] Approved systemd services
- [ ] System Profiles export, comparison and restore
- [x] Read-only transaction Activity history page
- [ ] Settings and offline controls

Software Stacks currently reports curated package roles and installed progress from the local snapshot only; installation controls remain disabled.

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
