#!/bin/sh
set -eu

# Common Git installation layouts are discoverable without running Git, even when
# only external helper programs were installed. Keep each layout independent.
mkdir -p unix-lib/prefix/bin unix-lib/prefix/lib/git-core
mkdir -p unix-libexec/prefix/bin unix-libexec/prefix/libexec/git-core
mkdir -p windows-ucrt/prefix/cmd windows-ucrt/prefix/ucrt64/libexec/git-core
mkdir -p windows-mingw/prefix/cmd windows-mingw/prefix/mingw64/libexec/git-core
mkdir -p custom-core real/bin real/libexec/git-core symlink/bin
printf '#!/bin/sh\nexit 91\n' >real/bin/git
chmod +x real/bin/git
ln -s ../../real/bin/git symlink/bin/git

# Configuration lives outside helpers, including a package manager's stable prefix.
for prefix in unix-lib/prefix unix-libexec/prefix windows-ucrt/prefix windows-mingw/prefix symlink; do
    mkdir -p "$prefix/etc"
    printf '[core]\n    autocrlf = false\n' >"$prefix/etc/gitconfig"
done
mkdir -p apple/Developer/usr/bin apple/Developer/usr/libexec/git-core apple/Developer/usr/share/git-core program-data/Git
printf '[credential]\n    helper = osxkeychain\n' >apple/Developer/usr/share/git-core/gitconfig
printf '[core]\n    autocrlf = true\n' >program-data/Git/config
