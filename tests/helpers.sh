# Must be sourced into the main journey test

function set-static-git-environment() {
  set -a
  export GIT_AUTHOR_DATE="2020-09-09 09:06:03 +0800"
  export GIT_COMMITTER_DATE="${GIT_AUTHOR_DATE}"
  export GIT_AUTHOR_NAME="Sebastian Thiel"
  export GIT_COMMITTER_NAME="${GIT_AUTHOR_NAME}"
  export GIT_AUTHOR_EMAIL="git@example.com"
  export GIT_COMMITTER_EMAIL="${GIT_AUTHOR_EMAIL}"
  set +a
}

function remove-paths() {
  sed -E 's#/.*#"#g'
}

function repo-with-remotes() {
  if [[ $((($# - 1) % 2)) != 0 ]] || [[ $# = 0 ]]; then
    echo "need <path> (<remote> <url>)[,...] tuples"
    exit 42
  fi

  mkdir -p "$1"
  (
    cd "$1"
    shift
    git init
    while [[ $# != 0 ]]; do
        git remote add "$1" "$2"
        shift 2
    done
    git config commit.gpgsign false
    git config tag.gpgsign false
    touch a
    git add a
    git commit -m "non-bare"
  ) &>/dev/null
}

function small-repo-in-sandbox() {
  sandbox
  {
    git init
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    touch a
    git add a
    git commit -m "first"
    git tag unannotated
    touch b
    git add b
    git commit -m "second"
    git tag annotated -m "tag message"
    git branch dev
    echo hi >> b
    git commit -am "third"
  } &>/dev/null
}

# Creates a repo with 4 versions of a file (blob#0..#3) in 50-line increments.
# Aggressive repacking forces Git to build a delta chain.
# blob#3 (latest/largest) becomes the base, with others as sequential deltas.
function pack-repo-in-sandbox() {
  sandbox
  {
    git init
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false

    for i in 0 1 2 3; do
      python3 -c "
for j in range(50 * ($i + 1)):
    print(f'line {j}')
" > file.txt
      git add file.txt
      git commit -m "blob#${i}"
    done

    git repack -a -d --window=10 --depth=4
  } &>/dev/null
}

function launch-git-daemon() {
    local i git_daemon_url_file

    git_daemon_url_file="$(mktemp -t git-daemon-url.XXXXXX)"
    "$jtt" git-daemon "$git_daemon_url_file" &
    daemon_pid=$!

    for i in $(seq 1 50); do
      if test -s "$git_daemon_url_file"; then
        git_daemon_url="$(cat "$git_daemon_url_file")"
        rm -f "$git_daemon_url_file"
        trap 'kill "$daemon_pid" 2>/dev/null || true; wait "$daemon_pid" 2>/dev/null || true' EXIT
        return
      fi
      sleep 0.1
    done

    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
    rm -f "$git_daemon_url_file"
    echo 1>&2 "failed to start git daemon"
    exit 1
}
