#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
SKIP_BUILD=0
if [[ "${1:-}" == "--skip-build" ]]; then SKIP_BUILD=1; fi

cd "$ROOT/engine"
if [[ "$SKIP_BUILD" -eq 0 ]]; then
  cargo build -j 1 --release --features mono
fi

# Compile editor C# scripts (mcs from mono-devel)
if command -v mcs >/dev/null 2>&1; then
  mkdir -p scripts/bin/Mono
  mapfile -t SOURCES < <(find scripts -maxdepth 1 -name '*.cs' | sort)
  mcs -target:library -out:scripts/bin/Mono/EditorScript.dll "${SOURCES[@]}"
else
  echo "warning: mcs not found; EditorScript.dll not refreshed" >&2
fi

python3 "$ROOT/installer/linux/build_deb.py" \
  --binary "$ROOT/engine/target/release/mono_editor_demo" \
  --version "$VERSION" \
  --out "$ROOT/dist"
