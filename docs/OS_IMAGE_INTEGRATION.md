# GenixBit OS image integration

The desktop image installs the `genixbit-software-center` Debian binary package by default through `os-image/default-packages.list`.

The package is expected to provide:

- `genixbit-software-center` as the desktop application
- `genixpkgd` as the system D-Bus service executable
- the desktop and AppStream metadata under `data/`
- full-color and symbolic hicolor icons
- the packaged GTK stylesheet
- the public D-Bus interface contract
- the hardened `genixpkgd.service` systemd unit

Image builders should consume the seed as a package-selection input rather than copying build-tree binaries directly. This keeps ownership, upgrades, signatures and rollback within the distribution package manager.

CI runs `scripts/validate-os-image-install.py` to ensure the package remains selected exactly once and all source-controlled runtime assets remain present.

This integration does not enable package transactions in the application. Install, remove, upgrade, repository-refresh and service-control operations remain governed by the transaction roadmap and stay fail-closed.
