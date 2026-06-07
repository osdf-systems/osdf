#!/usr/bin/env bash
# Install or refresh the osdf CLI on ~/.local/bin (Unix).
#
# Usage:
#   ./scripts/install-cli.sh
#   ./scripts/install-cli.sh --copy-only --binary target/release/osdf
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_dir="${OSDF_INSTALL_DIR:-$HOME/.local/bin}"
copy_only=0
binary_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --copy-only)
      copy_only=1
      shift
      ;;
    --binary)
      binary_path="${2:-}"
      shift 2
      ;;
    --skip-path-hint)
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

mkdir -p "$install_dir"

if [[ "$copy_only" -eq 0 ]]; then
  echo "Building osdf-cli (release)..."
  (cd "$repo_root" && cargo build --release -p osdf-cli)
  binary_path="$repo_root/target/release/osdf"
fi

if [[ -z "$binary_path" ]]; then
  echo "--binary is required when --copy-only is set." >&2
  exit 1
fi

if [[ ! -f "$binary_path" ]]; then
  echo "Built CLI not found at $binary_path." >&2
  exit 1
fi

install_path="$install_dir/osdf"
cp "$binary_path" "$install_path"
chmod +x "$install_path"
echo "Installed $install_path"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    echo
    echo "Add to PATH (shell startup file, e.g. ~/.zshrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo "Open a new terminal after updating PATH."
    ;;
esac

"$install_path" --version
echo
"$install_path" verify --help
