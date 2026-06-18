#!/usr/bin/env sh
set -eu

required_paths="
Cargo.toml
source/Cargo.toml
fuzz/Cargo.toml
package.json
tsconfig.json
ui/console/package.json
docs/Architecture.md
deploy/helm/oxisentinel-gateway-controller/Chart.yaml
tests/rust/workspace_structure.rs
"

for path in $required_paths; do
  if [ ! -e "$path" ]; then
    echo "missing required workspace path: $path" >&2
    exit 1
  fi
done
