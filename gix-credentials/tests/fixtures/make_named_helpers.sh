#!/bin/sh
set -eu

# Helpers remain usable without installing the Git executable. The same program
# is discoverable directly in PATH, in a conventional layout, or via GIT_EXEC_PATH.
mkdir -p 'installation with spaces/bin' 'installation with spaces/libexec/git-core' standalone
cat >'installation with spaces/libexec/git-core/git-credential-sample' <<'EOF'
#!/bin/sh
printf 'username=%s\npassword=%s:%s\n' "$GIX_HELPER_USER" "$1" "$2"
EOF
chmod +x 'installation with spaces/libexec/git-core/git-credential-sample'
cp 'installation with spaces/libexec/git-core/git-credential-sample' standalone/git-credential-sample
