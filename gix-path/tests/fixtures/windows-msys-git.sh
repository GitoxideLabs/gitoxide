#!/usr/bin/env bash
set -eu

# These empty Git executables cannot run. A complete pacman registration identifies the
# known MSYS2 package, whose compiled paths still need conversion by its own runtime.
for root in standard missing-config mounted unregistered wrong-package incomplete duplicate no-core; do
    mkdir -p "$root/usr/bin" "$root/usr/lib/git-core" "$root/etc"
    touch "$root/usr/bin/git.exe" "$root/usr/lib/git-core/git.exe"
    package="$root/var/lib/pacman/local/git-2.56.0-1"
    if test "$root" != unregistered; then
        mkdir -p "$package"
        cat >"$package/desc" <<-'EOF'
	%NAME%
	git

	%VERSION%
	2.56.0-1
	EOF
        cat >"$package/files" <<-'EOF'
	%FILES%
	usr/bin/git.exe
	usr/lib/git-core/git.exe
	EOF
    fi
done

printf '%%NAME%%\nother\n' >wrong-package/var/lib/pacman/local/git-2.56.0-1/desc
printf '%%FILES%%\nusr/bin/git.exe\n' >incomplete/var/lib/pacman/local/git-2.56.0-1/files
cp -R duplicate/var/lib/pacman/local/git-2.56.0-1 duplicate/var/lib/pacman/local/git-2.55.0-1
rm no-core/usr/lib/git-core/git.exe

# Git for Windows supplies the same MSYS runtime tools. Relocating them makes conversion
# independent of installed MSYS2 distributions without downloading or launching Git.
for root in standard missing-config mounted unregistered; do
    cp /usr/bin/cygpath.exe /usr/bin/msys-2.0.dll "$root/usr/bin/"
done
printf '[validation]\norigin = msys-package\n' >standard/etc/gitconfig

# Mount /usr/lib and /etc separately: deriving configuration from the native core path,
# or returning the physical package path instead of asking cygpath, must fail this fixture.
mkdir -p mounted-lib/git-core mounted-etc
touch mounted-lib/git-core/git.exe
printf '[validation]\norigin = mounted-msys-package\n' >mounted-etc/gitconfig
mounted_lib=$(cygpath -am "$PWD/mounted-lib")
mounted_etc=$(cygpath -am "$PWD/mounted-etc")
printf '%s /usr/lib ntfs binary 0 0\n%s /etc ntfs binary 0 0\n' \
    "${mounted_lib// /\\040}" "${mounted_etc// /\\040}" >mounted/etc/fstab
