#!/usr/bin/env bash
set -eu

exe=${1:?First argument must be the executable to test}
exe_plumbing=${2:?Second argument must be the plumbing executable to test}
jtt=${3:?Third argument the journey test tool}
kind=${4:?Fourth argument must an indicator of the kind of binary under test}

root="$(cd "${0%/*}" && pwd)"

# if relative paths are given eval them from the parent of this script's location 
if [[ $exe != /* ]]; then
  exe="${root}/../$exe"
fi
if [[ $exe_plumbing != /* ]]; then
  exe_plumbing="${root}/../$exe_plumbing"
fi
if [[ $jtt != /* ]]; then
  jtt="${root}/../$jtt"
fi

if [[ ${5:-} != --isolated ]]; then
  exec "$jtt" run "$BASH" "$root/journey.sh" "$exe" "$exe_plumbing" "$jtt" "$kind" --isolated
fi

# shellcheck disable=1090
source "$root/utilities.sh"
source "$root/helpers.sh"
snapshot="$root/snapshots"
fixtures="$root/fixtures"

SUCCESSFULLY=0
WITH_FAILURE=1
WITH_CLAP_FAILURE=2

# `sort` which is used in some of the snapshots tests depends on the value of
# `LC_ALL` when it comes to the order of dotfiles and non-dotfiles. It sorts
# differently when `LC_ALL=C` vs. when it is not set at all.
export LC_ALL=C

set-static-git-environment

source "$root/journey/ein.sh"
source "$root/journey/gix.sh"
