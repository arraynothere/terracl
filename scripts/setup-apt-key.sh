#!/usr/bin/env bash
set -euo pipefail

name="TerraCL APT Repository"
email="arraynothere@users.noreply.github.com"
output="terracl-apt-private.asc"

export GNUPGHOME="$(mktemp -d)"
trap 'rm -rf "$GNUPGHOME"' EXIT

gpg --batch --pinentry-mode loopback --passphrase '' \
    --quick-generate-key "$name <$email>" rsa4096 sign never

fingerprint=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')
gpg --batch --pinentry-mode loopback --passphrase '' --armor --export-secret-keys "$fingerprint" > "$output"

echo "Key fingerprint: $fingerprint"
echo "Private key written to $output"
echo
echo "1. Open GitHub: Settings > Secrets and variables > Actions > New repository secret"
echo "2. Name: APT_GPG_PRIVATE_KEY"
echo "3. Value: the full content of $output"
echo "4. Delete $output afterwards and never commit it"
