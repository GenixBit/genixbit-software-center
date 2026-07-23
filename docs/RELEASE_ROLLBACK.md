# Release and rollback testing

GenixBit Software Center uses `release/release-policy.json` as the source-controlled compatibility contract for application releases and supported rollbacks.

The contract records:

- the current semantic version;
- the stable D-Bus API generation;
- transaction-journal, event-journal, settings and System Profile format generations;
- explicitly supported rollback targets; and
- runtime assets that must remain present in a release source tree.

CI runs `scripts/validate-release-rollback.py` and its unit tests. The validator confirms that the current version matches the Cargo workspace and AppStream metadata, every rollback target is older than the current release, the D-Bus API generation is unchanged, and each target can read every persistent format produced by the current release.

A rollback target must be removed from the policy before the current release introduces a persistent format that target cannot read. The image or package release pipeline must then reject that downgrade rather than risk corrupting transaction history, settings, or exported profiles.

Run the source-tree audit with:

```bash
python3 scripts/validate-release-rollback.py
```

This milestone validates release metadata and compatibility only. It does not install, downgrade, remove, refresh, launch, or stop software. Distribution package tooling remains responsible for building signed artifacts, creating snapshots, and performing an operator-approved rollback. Package-changing application paths and service controls remain fail-closed.
