#!/bin/bash
set -e
cd "$(dirname "$0")"
bin="target/release/liquidglass0-mcp"
if [[ ! -f "$bin" ]] || [[ -n "$(find liquidglass0-mcp/src liquidglass0-render/src liquidglass0-core/src -newer "$bin" 2>/dev/null)" ]]; then
    cargo build -p liquidglass0-mcp --release
fi
exec "$bin"
