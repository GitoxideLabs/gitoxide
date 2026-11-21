#!/usr/bin/env bash
set -eu

# A relative askpass executable reads a sibling file, requiring the caller's directory.
mkdir 'working directory'
cd 'working directory'
git init -q
printf '%s\n' 'operation-secret' > response.txt
cat > askpass <<'EOF'
#!/bin/sh
read -r response < response.txt
printf '%s\n' "$response"
EOF
chmod +x askpass
