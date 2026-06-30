#!/usr/bin/env bash
set -euo pipefail

repo_url="${ATFLOW_REPO_URL:-https://github.com/zhangcongke/atflow.git}"
install_dir="${ATFLOW_INSTALL_DIR:-$HOME/.local/bin}"
install_dir="${install_dir%/}"
if [ -z "$install_dir" ]; then
  install_dir="/"
fi

die() {
  printf 'atflow install: %s\n' "$*" >&2
  exit 1
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "required command '$1' was not found in PATH"
  fi
}

if [ ! -t 0 ]; then
  die "interactive stdin is required because at init asks setup questions; run with bash <(curl ...) or run at init manually"
fi

if [ "$install_dir" = "/" ]; then
  installed_path="/at"
else
  installed_path="$install_dir/at"
fi

require_command git
require_command cargo

tmp_dir="$(mktemp -d)"
build_root="$tmp_dir/install"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

git clone --depth 1 "$repo_url" "$tmp_dir/atflow"
printf 'Building Atflow...\n'
cargo install --path "$tmp_dir/atflow" --root "$build_root" --locked --quiet
mkdir -p "$install_dir"
cp "$build_root/bin/at" "$installed_path"
chmod 755 "$installed_path"

printf 'Installed Atflow to %s\n' "$installed_path"
"$installed_path" init
