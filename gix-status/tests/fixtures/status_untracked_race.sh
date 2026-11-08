#!/usr/bin/env bash
set -eu -o pipefail

# A rename candidate whose path is removed or replaced after status opens the file.
# The process filter changes the path before completing its handshake and offers no
# clean capability, so status hashes the original open file without filtering it.
git init -q
echo 'renamed filter=race -text' >.gitattributes
echo original >source
git add .gitattributes source
git commit -q -m 'A file to rename'
mv source renamed

cat >.git/change-untracked.sh <<'EOF'
#!/usr/bin/env bash
set -eu -o pipefail
case "$1" in
    remove) rm renamed ;;
    replace)
        echo 'a replacement with a different length' >.git/replacement
        mv .git/replacement renamed
        ;;
    *) exit 1 ;;
esac
# Packet-line greeting, version, flush, and an empty set of filter capabilities.
printf '0016git-filter-server\n000eversion=2\n00000000'
cat >/dev/null
EOF
