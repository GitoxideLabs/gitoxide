#!/usr/bin/env bash
set -eu -o pipefail

# list of urls that should be tested for all platforms
tests=()
# urls only intended for testing on Unix platforms
tests_unix=()
# urls only intended for testing on Windows
tests_windows=()
# HTTP URLs whose decoded paths are obtained through Git's credential plumbing.
credential_urls=(
  "https://example.com/a%2Fb/"
  "https://example.com/%3Fquery"
  "https://example.com/%23fragment"
  "https://example.com/%252F"
  "https://host?@redirected/repo"
  "https://host#@redirected/repo"
  "https://example.com/a%20b"
  "https://example.com/caf%C3%A9"
  "https://[2001:db8::1]:8443/repo"
  "https://example.com/%2f%3f%23%25"
)

# The contents and structure of this loop are an adaption
# from git's own test suite (t/t5500-fetch-pack.sh).
# Please do not change this loop and instead add additional
# test cases at the bottom of this file.
for path in "repo" "re:po" "re/po"; do
  # normal urls
  for protocol in "ssh+git" "git+ssh" "git" "ssh"; do
    for host in "host" "user@host" "user_name@host" "user.name@host" "user@[::1]" "user@::1"; do
      for port_separator in "" ":"; do
        tests+=("$protocol://$host$port_separator/$path")

        tests+=("$protocol://$host$port_separator/~$path")
      done
    done
    for host in "host" "User@host" "User@[::1]"; do
      tests+=("$protocol://$host:22/$path")
    done
  done
  # file protocol urls
  for protocol in "file"; do
    tests_unix+=("$protocol://$host/$path")

    tests_windows+=("$protocol://$host/$path")
    tests_windows+=("$protocol:///$path")

    tests_unix+=("$protocol://$host/~$path")
    tests_windows+=("$protocol://$host/~$path")
  done
  # local paths
  for host in "nohost" "nohost:12" "[::1]" "[::1]:23" "[" "[:aa"; do
    tests+=("./$host:$path")
    tests+=("./$protocol:$host/~$path")
  done
  # SCP like urls
  for host in "user@name@host" "user_name@host" "host" "[::1]" "[fe80::1%Eth0]"; do
    tests+=("$host:$path")
    tests+=("$host:/~$path")
  done
done

# These two test cases are from git's test suite as well.
tests_windows+=("file://c:/repo")
tests_windows+=("c:repo")
tests+=("ssh://[fe80::1%25Eth0]/repo")
tests+=(
  "ssh://host?@redirected/repo"
  "ssh://host#@redirected/repo"
  "ssh://host%2ename/repo"
  "git://host%2ename/repo"
  "ssh://host:0/repo"
  "ssh://foo:bar:baz/repo"
  "ssh://[not-ip]/repo"
  "file://::1/repo"
  "ssh://host:1/repo"
  "ssh://host:65535/repo"
  "ssh://user@[2001:db8::1]:2222/repo"
  "ssh://user@2001:db8::1/repo"
  "ssh://::1/repo"
  "git://::1/repo"
  "ssh://host/a%2Fb"
  "ssh://host/%3Fquery"
  "ssh://host/%23fragment"
  "ssh://host/%252F"
  "git://host/a%2Fb"
)
tests_unix+=(
  "file:///repo"
  "file://host/repo"
  "file://localhost/repo"
  "file://[::1]/repo"
  "file://x:/repo"
)
tests_windows+=(
  "file:///repo"
  "file://host/repo"
  "file://localhost/repo"
  "file://[::1]/repo"
)

tests_unix+=("${tests[@]}")
tests_windows+=("${tests[@]}")

# We will run `git fetch-pack` in this repo instead of the outer gitoxide repo,
# for full isolation. This avoids assuming there *is* a gitoxide repo, and also
# avoids `safe.directory` errors if the gitoxide repo has unusual ownership.
git init -q temp-repo

for url in "${tests_unix[@]}"
do
  echo ";" # there are no `;` in the tested urls
  git -C temp-repo fetch-pack --diag-url "$url"
done >git-baseline.unix

for url in "${tests_windows[@]}"
do
  echo ";" # there are no `;` in the tested urls
  git -C temp-repo fetch-pack --diag-url "$url"
done >git-baseline.windows

# `fetch-pack --diag-url` doesn't support HTTP, so use Git's credential parser as the baseline oracle.
for url in "${credential_urls[@]}"
do
  block=$(
    echo ";"
    echo "Diag: url=$url"
    printf 'url=%s\n\n' "$url" |
      git -c credential.useHttpPath=true \
        -c 'credential.helper=!f() { echo username=baseline; echo password=baseline; }; f' credential fill |
      sed -n 's/^protocol=/Diag: protocol=/p; s/^host=/Diag: hostandport=/p; s/^path=/Diag: path=/p'
  )
  printf '%s\n' "$block" | tee -a git-baseline.unix >>git-baseline.windows
done

rm -rf temp-repo
