#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image_name="genixbit-transaction-integration:${GITHUB_SHA:-local}"

if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required to run the disposable transaction integration harness" >&2
    exit 1
fi

cd "${repository_root}"

docker build \
    --file tests/container/transaction.Dockerfile \
    --tag "${image_name}" \
    --progress plain \
    .

echo "Transaction integration container completed successfully: ${image_name}"
