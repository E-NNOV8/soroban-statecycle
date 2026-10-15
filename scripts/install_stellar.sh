#!/usr/bin/env bash
# Pinned official Linux x86_64 release; no unverified install script execution.
set -euo pipefail
case "$(uname -sm)" in
  'Linux x86_64') ;;
  *) echo 'Use the official Stellar CLI installation instructions for this platform.' >&2; exit 2 ;;
esac
destination=${1:-.local/bin}
mkdir -p "$destination"
task_download=$(mktemp -d)
trap 'rm -r -- "$task_download"' EXIT
curl --fail --location --retry 3 --output "$task_download/stellar.tar.gz" \
  'https://github.com/stellar/stellar-cli/releases/download/v28.1.0/stellar-cli-28.1.0-x86_64-unknown-linux-gnu.tar.gz'
printf '%s  %s\n' c1680deee94301d33ada7a17f98411e642a4248c727afbd2e43050d345746462 "$task_download/stellar.tar.gz" | sha256sum --check --status
tar -xzf "$task_download/stellar.tar.gz" -C "$task_download"
install -m 755 "$task_download/stellar" "$destination/stellar"
"$destination/stellar" --version
