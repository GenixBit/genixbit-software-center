# Release and rollback testing

GenixBit OS release pipelines must build the full Rust workspace in release mode and validate both the proposed release bundle and at least one retained rollback bundle before publication.

The repository contract is stored in `release/release-contract.json`. It pins the package, application and service identities, current semantic version, supported architectures, required source metadata, required installed paths and rollback policy.

## Source release gate

Run this check from the repository root:

```bash
python3 scripts/validate-release-rollback.py source \
  --source-root . \
  --contract release/release-contract.json
```

The source gate verifies:

- `workspace.package.version` and every crate's inherited version;
- the current AppStream release and desktop identity;
- the default OS-image package seed;
- all source-controlled runtime assets required by the release contract; and
- a rollback policy that retains at least one previous release, remains within the current major version and forbids automatic rollback.

CI also runs `cargo build --workspace --release --locked`, so the exact locked dependency graph must produce release-mode binaries.

## Staged bundle manifests

Stage each Debian package payload into its own filesystem root. Keep the manifest outside the staged root, then generate an exact SHA-256 inventory:

```bash
python3 scripts/validate-release-rollback.py manifest \
  --contract release/release-contract.json \
  --bundle-root "$CURRENT_ROOT" \
  --version 0.2.0 \
  --architecture amd64 \
  --output "$CURRENT_MANIFEST"
```

Repeat this step when producing a release manifest in the trusted packaging pipeline. Published manifests must be stored with their release artifacts. The validator rejects missing required paths, symbolic links, unsupported architectures and empty bundles.

## Current and rollback pair

Retrieve the previous manifest and staged payload from the trusted release archive, then validate both bundles together:

```bash
python3 scripts/validate-release-rollback.py pair \
  --contract release/release-contract.json \
  --current-root "$CURRENT_ROOT" \
  --current-manifest "$CURRENT_MANIFEST" \
  --rollback-root "$ROLLBACK_ROOT" \
  --rollback-manifest "$ROLLBACK_MANIFEST"
```

The pair gate checks exact file inventories and SHA-256 digests, rejects unmanifested or writable files, requires stable package/application/service identities, requires matching architectures, and requires the rollback version to be older while remaining within the current major version.

## Security boundary

This milestone validates release construction and rollback readiness. It does not install, remove, upgrade, downgrade or refresh packages and it never controls services. A rollback remains a distribution-operator action performed through the authenticated GenixBit OS package pipeline after separate image-level testing. The Software Center's package-changing operations remain fail-closed.
