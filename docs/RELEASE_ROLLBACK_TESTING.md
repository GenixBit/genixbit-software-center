# Release and rollback testing

GenixBit OS release pipelines must build the full Rust workspace in release mode and validate a retained current/previous package pair before publication.

The release manifest names exactly two artifacts: the current release and one older rollback release. Both must use the `genixbit-software-center` package identity, the same architecture, semantic `x.y.z` versions, and pinned SHA-256 digests. The rollback version must be strictly older than the current version.

Run:

```bash
python3 scripts/validate-release-rollback.py --root "$ARTIFACT_DIR" --manifest "$RELEASE_MANIFEST"
```

The policy retains exactly one previous release and forbids automatic rollback. A distribution operator may later perform an authenticated package-manager rollback after separate integration testing. This repository check never installs, removes, upgrades, downgrades, refreshes repositories, or controls services. Application package-changing paths remain fail-closed.
