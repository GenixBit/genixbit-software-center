# Repository signing validation

GenixBit OS image builds must validate APT repository trust before an image is published. The repository includes `scripts/validate-repository-signing.py`, a read-only audit that operates on an image root and a separately maintained keyring digest manifest.

## Required source policy

Every active `.sources` or `.list` entry must:

- use an `https://` repository URI;
- select `deb` or `deb-src` content only;
- declare exactly one `Signed-By` keyring path;
- keep the keyring below `/usr/share/keyrings/` with a `.gpg` suffix;
- avoid `Trusted: yes`, `Allow-Insecure: yes`, `Allow-Weak: yes` and `Check-Valid-Until: no` overrides;
- reference a non-empty regular keyring that is not group- or world-writable; and
- match a SHA-256 digest pinned by the image build.

Inline public keys and global legacy trust stores are intentionally rejected. Per-repository keyrings limit the authority of each trust anchor.

## Keyring manifest

The image pipeline supplies a JSON manifest outside the application source tree. It maps installed keyring paths to approved SHA-256 digests:

```json
{
  "version": 1,
  "keyrings": {
    "/usr/share/keyrings/genixbit-archive-keyring.gpg": "<64 lowercase hexadecimal characters>"
  }
}
```

Generate the digest from the exact packaged keyring artifact used by the image build. Do not copy a digest from an untrusted network response.

## Image-build command

Run the validator against the staged filesystem before creating or signing the OS image:

```bash
python3 scripts/validate-repository-signing.py \
  --root "$IMAGE_ROOT" \
  --manifest "$TRUSTED_KEYRING_MANIFEST"
```

The command fails when no active APT repository is present or when any entry violates the policy.

## Security boundary

This check does not contact a repository and does not claim that repository metadata has a valid signature. It validates the configured trust anchors and prevents common fail-open APT settings. During a distribution-managed repository refresh, APT and `gpgv` remain responsible for verifying signed `InRelease` or `Release.gpg` metadata against the approved keyring.

The GenixBit Software Center does not gain repository-refresh, install, remove or upgrade capability from this milestone. Those application paths remain fail-closed until the transaction and safe APT roadmaps are complete.
