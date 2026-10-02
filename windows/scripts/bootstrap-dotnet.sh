#!/usr/bin/env bash
set -euo pipefail
INSTALL_DIR="${DOTNET_INSTALL_DIR:-$HOME/.dotnet}"
CHANNEL="${DOTNET_CHANNEL:-9.0}"
curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh | bash -s -- --channel "$CHANNEL" --install-dir "$INSTALL_DIR"
export PATH="$INSTALL_DIR:$PATH"
echo "Installed: $(dotnet --version)"
echo "Add to shell: export PATH=\"$INSTALL_DIR:\$PATH\""
