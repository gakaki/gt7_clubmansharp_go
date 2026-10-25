#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESOURCES="$ROOT/../../site/public"

if [[ ! -f "$RESOURCES/data/cars.json" ]]; then
  echo "Missing offline resource tree: $RESOURCES" >&2
  exit 1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS packaging must run on macOS." >&2
  exit 1
fi

cd "$ROOT"
./gradlew :composeApp:verifyOfflineSources
./gradlew :composeApp:createDistributable
./gradlew :composeApp:packageDmg
