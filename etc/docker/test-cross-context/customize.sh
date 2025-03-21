#!/bin/bash
set -euxC

target="$1"
test -n "$target"

# Arrange for the indirect `tzdata` dependency to be installed and configured
# without prompting for the time zone. (Passing `-y` is not enough.)
export DEBIAN_FRONTEND=noninteractive TZ=UTC

# Install tools for setting up APT repositores. Install `apt-utils` before the
# others, so the installation of `gnupg` can use it for debconf.
apt-get update
apt-get install --no-install-recommends -y apt-utils
apt-get install --no-install-recommends -y apt-transport-https dpkg-dev gnupg
type dpkg-architecture  # Make sure we really have this.

# Decide what architecture to use for `git`, shared libraries `git` links to,
# and shared libraries gitoxide links to when building `max`. Instead of this
# custom logic, we could use `$CROSS_DEB_ARCH`, which `cross` tries to provide
# (https://github.com/cross-rs/cross/blob/v0.2.5/src/lib.rs#L268), and which is
# available for roughly the same architectures where this logic gets a nonempty
# value. But using `$CROSS_DEB_ARCH` may make it harder to build and test the
# image manually. In particular, if it is not passed, we would conclude that we
# should install the versions of those packages with the host's architecture.
apt_suffix=
if target_arch="$(dpkg-architecture --host-type "$target" --query DEB_HOST_ARCH)"
then
    dpkg --add-architecture "$target_arch"
    apt_suffix=":$target_arch"
    printf 'INFO: Using target architecture for `git` and libs in container.\n'
    printf 'INFO: This architecture is %s.\n' "$target_arch"
else
    apt_suffix=''
    printf 'WARNING: Using HOST architecture for `git` and libs in container.\n'
fi

# Get release codename. Like `lsb_release -sc`. (`lsb_release` may be absent.)
release="$(sed -n 's/^VERSION_CODENAME=//p' /etc/os-release)"

# Add the git-core PPA manually. (Faster than installing `add-apt-repository`.)
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

if test -n "$apt_suffix"; then
    # Install everything we need except `git` (and what we already have). We
    # can't necessarily install `git` this way, because it insists on `perl`
    # and `liberror-perl` dependencies of the same architecture as it. These
    # may not be possible to install in a mixed environment, where most
    # packages are a different architecture, and where `perl` is a dependency
    # of other important packages. So we will install everything else first
    # (then manually add `git`).
    apt-get install --no-install-recommends -y \
        "${git_deps[@]}" "${gix_test_deps[@]}" file

    # Add `git` by manually downloading it and installing it with `dpkg`,
    # forcing installation to proceed even if its `perl` and `liberror-perl`
    # dependencies, as declared by `git`, are absent. (We have already
    # installed them, but in a possibly different architecture. `git` can still
    # use them, because its use is to run scripts, rather than to link to a
    # shared library they provide.) It is preferred to let `apt-get download`
    # drop privileges to the `_apt` user during download, so we download it
    # inside `/tmp`. But we create a subdirectory so it is safe to make
    # assumptions about what files globs can expand to, even if `/tmp` is
    # mounted to an outside share temp dir on a multi-user system.
    mkdir /tmp/dl  # Don't use `-p`; if it exists already, we cannot trust it.
    chown _apt /tmp/dl  # Use owner, as the container may have no `_apt` group.
    (cd /tmp/dl && apt-get download "git$apt_suffix")
    dpkg --ignore-depends="perl$apt_suffix,liberror-perl$apt_suffix" \
        -i /tmp/dl/git[-_]*.deb
    rm -r /tmp/dl
else
    # Install everything we need, including `git`.
    apt-get install --no-install-recommends -y git "${gix_test_deps[@]}" file
fi

# Show information about the newly installed `git` (and ensure it can run).
git version --build-options
git="$(command -v git)"
file -- "$git"

# Clean up files related to package management that we won't need anymore.
apt-get clean
rm -rf /var/lib/apt/lists/*

# If this image has a runner script `cross` uses for Android, patch the script
# to add the ability to suppress its customization of `LD_PRELOAD`. The runner
# script sets `LD_PRELOAD` to the path of `libc++_shared.so` in the Android NDK
# (https://github.com/cross-rs/cross/blob/v0.2.5/docker/android-runner#L34).
# But this causes a problem for us. When a host-architecture program is run,
# `ld.so` shows a message about the "wrong ELF class". Such programs can still
# run, but when we rely on their specific output to stderr, fixtures and tests
# fail. The change we make here lets us set `NO_PRELOAD_CXX=1` to avoid that.
runner=/android-runner
patch='s/^[[:blank:]]*export LD_PRELOAD=/test "${NO_PRELOAD_CXX:-0}" != 0 || &/'
if test -f "$runner"; then sed -i.orig "$patch" -- "$runner"; fi

# Ensure a nonempty Git `system` scope (for the `installation_config` tests).
git config --system gitoxide.imaginary.arbitraryVariable arbitraryValue
