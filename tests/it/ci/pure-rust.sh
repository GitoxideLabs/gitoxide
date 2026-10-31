#!/usr/bin/env bash
set -euo pipefail

# This script runs only in the disposable Debian container started by `ci exec pure-rust`.
export CARGO_HOME=/cache/cargo RUSTUP_HOME=/cache/rustup CARGO_TARGET_DIR=/cache/build
export PATH="$CARGO_HOME/bin:$PATH"
apt-get update
apt-get install --no-install-recommends -y ca-certificates curl gcc libc-dev
for package in cmake g++ libssl-dev make pkgconf pkg-config; do
    if dpkg-query --status -- "$package"; then
        printf 'Unexpected development package: %s\n' "$package" >&2
        exit 1
    fi
done
for program in cmake g++ make pkgconf pkg-config; do
    if command -v -- "$program"; then
        printf 'Unexpected development tool: %s\n' "$program" >&2
        exit 1
    fi
done
if ! command -v rustup >/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
        sh -s -- -y --profile minimal --default-toolchain stable
fi
rustup toolchain install stable --profile minimal --no-self-update
export RUSTUP_TOOLCHAIN=stable
cargo tree --locked --no-default-features --features max-pure >/tmp/tree.txt
if grep -E -x -e '.*\b(-sys|cc|cmake|pkg-config|vcpkg)\b.*' /tmp/tree.txt; then
    printf 'warning: dependencies associated with C/C++ builds are present\n' >&2
fi
if grep -P '(?<!\b(linux-raw|aws-lc|inotify|dirs))-sys\b' /tmp/tree.txt; then
    printf 'Unrecognized system dependency in max-pure\n' >&2
    exit 1
fi
cat >/usr/local/bin/ci-cc1 <<'EOF'
#!/bin/sh
set -e
printf '%s\n' "$0 $*" | flock /tmp/ci-cc1.lock tee -a /tmp/ci-cc1.log >&2
exec "$0.orig" "$@"
EOF
chmod +x /usr/local/bin/ci-cc1
while IFS= read -r compiler; do
    mv -- "$compiler" "$compiler.orig"
    ln -s /usr/local/bin/ci-cc1 "$compiler"
done < <(find /usr/lib/gcc \( -name cc1 -o -name cc1plus \) -print)
cargo install --debug --locked --no-default-features --features max-pure --path . --root /cache/install
if [[ -e /tmp/ci-cc1.log ]]; then
    cat /tmp/ci-cc1.log
    printf 'warning: max-pure invoked a C/C++ compiler\n' >&2
fi
