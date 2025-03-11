#!/bin/bash
set -euxC

target="$1"

# Arrange for the indirect `tzdata` dependency to be installed and configured
# without prompting for the time zone. (Passing `-y` is not enough.)
export DEBIAN_FRONTEND=noninteractive TZ=UTC

# Install tools for setting up APT repositores. Install `apt-utils` before the
# others, so the installation of `gnupg` can use it for debconf.
apt-get update
apt-get install --no-install-recommends -y apt-utils
apt-get install --no-install-recommends -y apt-transport-https dpkg-dev gnupg
type dpkg-architecture  # Make sure we really have this.

# Decide what architecture to use for `git`, shared libraries for gitoxide when
# attempting to build `max`, and shared libraries used by `git` itself.
apt_suffix=
if target_arch="$(dpkg-architecture --host-type "$target" --query DEB_HOST_ARCH)"; then
    dpkg --add-architecture "$target_arch"
    apt_suffix=":$target_arch"
fi

# Add the git-core PPA manually. (Faster than installing `add-apt-repository`.)
release="$(sed -n 's/^VERSION_CODENAME=//p' /etc/os-release)"
echo "deb https://ppa.launchpadcontent.net/git-core/ppa/ubuntu $release main" \
    >/etc/apt/sources.list.d/git-core-ubuntu-ppa.list
apt-key adv --keyserver keyserver.ubuntu.com \
    --recv-keys F911AB184317630C59970973E363C90F8F1B6217
apt-get update

# Remove the old `git` and associated packages.
apt-get purge --autoremove -y git

# Git dependencies. These are for the desired architecture, except `git-man` is
# the same package for all architectures, and we can't always install `perl` or
# `liberror-perl` for the desired architecture (at least in s390x).
# TODO(maint): Resolve these dynamically to support future `cross` base images.
git_deps=(
    git-man
    "libc6$apt_suffix"
    "libcurl3-gnutls$apt_suffix"
    "libexpat1$apt_suffix"
    liberror-perl
    "libpcre2-8-0$apt_suffix"
    "zlib1g$apt_suffix"
    perl
)

# Other dependencies for running the gitoxide test suite and fixture scripts,
# and for building and testing gitoxide for feature sets beyond `max-pure`.
gix_test_deps=(
    ca-certificates
    cmake
    "curl$apt_suffix"
    jq
    "libc-dev$apt_suffix"
    "libssl-dev$apt_suffix"
    patch
    pkgconf
)

# Install everything we need except `git` (and what we already have). We can't
# necessarily install `git` this way, because it insists on `perl` and
# `liberror-perl` dependencies of the same architecture as it. These may not be
# possible to install in a mixed environment, where most packages are a
# different architecture, and where `perl` is a dependency of other important
# packages. So we will install everything else first (then manually add `git`).
apt-get install --no-install-recommends -y \
    "${git_deps[@]}" "${gix_test_deps[@]}" file

# Add `git` by manually downloading it and installing it with `dpkg`, forcing
# installation to proceed even if its `perl` and `liberror-perl` dependencies,
# as declared by `git`, are absent. (We have already installed them, but in a
# possibly different architecture. `git` can still use them, because its use is
# to run scripts, rather than to link to a shared library they provide.) It is
# preferred to let `apt-get download` drop privileges to the `_apt` user during
# download, so we download it inside `/tmp`. But we create a subdirectory so it
# is safe to make assumptions about what files globs can expand to, even if
# `/tmp` is mounted to an outside share temp dir on a multi-user system.
mkdir /tmp/dl  # Don't use `-p`; if it exists already, we cannot trust it.
chown _apt /tmp/dl  # Use owner, as the container may not have an `_apt` group.
(cd /tmp/dl && apt-get download "git$apt_suffix")
dpkg --ignore-depends="perl$apt_suffix,liberror-perl$apt_suffix" \
    -i /tmp/dl/git[-_]*.deb

# Show information about the newly installed `git` (and ensure it can run).
git version --build-options
git="$(command -v git)"
file -- "$git"

# Clean up files related to package management that we won't need anymore.
apt-get clean
rm -rf /tmp/dl /var/lib/apt/lists/*

# If this is an Android-related image or otherwise has a runner script `cross`
# uses for Android, then patch the script to add the ability to suppress its
# customization of `LD_PRELOAD`. This runner script sets `LD_PRELOAD` to the
# path of `libc++_shared.so` in the vendored Android NDK. But this causes a
# problem for us because, when a non-Android (i.e. a host-architecture) program
# is run, `ld.so` shows a message about the "wrong ELF class". Such programs
# can still run, but when we make an assertion about, parse, or otherwise rely
# on their output to standard error, we get test failures. (That especially
# affects fixtures.) This change lets us pass `NO_PRELOAD_CXX=1` to avoid that.
if test -f /android-runner; then
    sed -i.orig 's/^export LD_PRELOAD=/test "${NO_PRELOAD_CXX:-0}" != 0 || &/'
        /android-runner
fi

# Ensure a nonempty Git `system` scope (for the `installation_config` tests).
git config --system gitoxide.imaginary.arbitraryVariable arbitraryValue
