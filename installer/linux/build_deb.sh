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
  # Keep this list in sync with build_mono.ps1. Do not build the standalone
  # MonoUIScript/UIScript demo entry points into EditorScript.dll.
  SOURCES=(
    scripts/RotationScript.cs scripts/UI.cs scripts/IconIds.cs
    scripts/UI.Theme.cs scripts/UI.Widgets.cs scripts/UI.ComplexWidgets.cs
    scripts/UI.NewWidgets.cs scripts/UI.Scene.cs scripts/UI.Pipeline.cs
    scripts/UI.AssetProject.cs scripts/DFX.cs scripts/EditorScript.cs
    scripts/EditorScript.State.cs scripts/EditorScript.View.cs
    scripts/EditorScript.Presenter.cs scripts/EditorScript.WidgetGallery.cs
    scripts/EditorScript.Dock.cs scripts/EditorScript.CommandPalette.cs
    scripts/EditorScript.Gizmo.cs scripts/EditorScript.FloatingPanel.cs
    scripts/EditorScript.TextureThumb.cs scripts/AssetProjectTest.cs
    scripts/UITestRunner.cs scripts/UITestRunner.Report.cs
    scripts/IntentTestRunner.cs scripts/TestTelemetry.cs
    scripts/ExposeAttribute.cs scripts/IScriptEntity.cs
    scripts/RotatingEntity.cs scripts/TestRunner.cs
  )
  mcs -target:library -out:scripts/bin/Mono/EditorScript.dll "${SOURCES[@]}"
else
  echo "warning: mcs not found; EditorScript.dll not refreshed" >&2
fi

python3 "$ROOT/installer/linux/build_deb.py" \
  --binary "$ROOT/engine/target/release/mono_editor_demo" \
  --version "$VERSION" \
  --out "$ROOT/dist"
