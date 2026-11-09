#!/usr/bin/env bash
set -eu

# Every fake Git reports a different path from the inferred one, so finding the
# Homebrew config proves discovery did not launch it. Queries do not mutate fixtures.
for bin in homebrew/Cellar/git/2.56.0/bin incomplete/Cellar/git/2.56.0/bin custom/bin unknown non-executable; do
    mkdir -p "$bin"
    cat >"$bin/git" <<'EOF'
#!/bin/sh
printf 'unknown\000file:/queried/gitconfig\000core.test\000'
EOF
    chmod +x "$bin/git"
done

# Apple Git can read a different installation config when invoked indirectly.
mkdir -p apple-link real-apple-link
ln -s ../Xcode.app/Contents/Developer/usr/bin/git apple-link/git
ln -s /Library/Developer/CommandLineTools/usr/bin/git real-apple-link/git
ln -s /Library/Developer/CommandLineTools real-developer-link
chmod -x non-executable/git
mkdir -p broken
ln -s missing-git broken/git

# Homebrew links its public bin through opt into a versioned Cellar installation,
# but keeps the configuration under the unversioned prefix.
mkdir -p homebrew/bin homebrew/opt homebrew/etc
ln -s ../Cellar/git/2.56.0 homebrew/opt/git
ln -s ../opt/git/bin/git homebrew/bin/git
printf '[core]\n\tfilemode = true\n' >homebrew/etc/gitconfig

# A mode can grant execution to others but deny it to the file's owner.
mkdir -p restricted/Cellar/git/2.56.0/bin restricted/etc
cp homebrew/Cellar/git/2.56.0/bin/git restricted/Cellar/git/2.56.0/bin/git
chmod 645 restricted/Cellar/git/2.56.0/bin/git
cp homebrew/etc/gitconfig restricted/etc/gitconfig

# An arbitrary bin/etc layout does not establish Git's compile-time config path.
mkdir -p custom/etc
printf '[core]\n\tfilemode = false\n' >custom/etc/gitconfig

# Apple Git reads installation config below the selected developer directory.
# Empty executable files cannot run, making a successful lookup proof of inference.
for developer in Xcode.app/Contents/Developer Library/Developer/CommandLineTools; do
    mkdir -p "$developer/usr/bin" "$developer/usr/share/git-core"
    touch "$developer/usr/bin/git"
    chmod +x "$developer/usr/bin/git"
    printf '[credential]\n\thelper = osxkeychain\n' >"$developer/usr/share/git-core/gitconfig"
done
