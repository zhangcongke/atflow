#!/usr/bin/env bash
set -euo pipefail

repo_url="${ATFLOW_REPO_URL:-https://github.com/zhangcongke/atflow.git}"
install_dir="${ATFLOW_INSTALL_DIR:-$HOME/.local/bin}"
install_dir="${install_dir%/}"

die() {
  printf 'atflow install: %s\n' "$*" >&2
  exit 1
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "required command '$1' was not found in PATH"
  fi
}

case "$install_dir" in
  */bin)
    install_root="${install_dir%/bin}"
    ;;
  bin)
    install_root="."
    ;;
  *)
    die "ATFLOW_INSTALL_DIR must end in /bin because cargo install --root writes to <root>/bin"
    ;;
esac

if [ -z "$install_root" ]; then
  install_root="/"
fi

if [ "$install_root" = "/" ]; then
  installed_path="/bin/at"
else
  installed_path="$install_root/bin/at"
fi

require_command git
require_command cargo

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

git clone --depth 1 "$repo_url" "$tmp_dir/atflow"
cargo install --path "$tmp_dir/atflow" --root "$install_root" --locked

printf 'Installed Atflow to %s\n' "$installed_path"
"$installed_path" init
