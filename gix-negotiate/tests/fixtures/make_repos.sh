#!/usr/bin/env bash
set -eu -o pipefail

function tick () {
  if test -z "${tick+set}"
  then
    tick=1112911993
  else
    tick=$(($tick + 60))
  fi
  GIT_COMMITTER_DATE="$tick -0700"
  GIT_AUTHOR_DATE="$tick -0700"
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE
}

tick
function commit() {
  local message=${1:?first argument is the commit message}
  local file="$message.t"
  echo "$1" > "$file"
  git add -- "$file"
  tick
  git commit -m "$message"
  git tag "$message"
}

function negotiation_tips () {
  local tips=""
  for arg in "$@"; do
    tips+=" --negotiation-tip=$arg"
  done
  echo "$tips"
}

function trace_fetch_baseline () {
  local remote="${1:?need remote url}"; shift
  git -C client commit-graph write --no-progress --reachable
  git -C client repack -adq

  for algo in noop consecutive skipping; do
    GIT_TRACE_PACKET="$PWD/baseline.$algo" \
    git -C client -c fetch.negotiationAlgorithm="$algo" fetch --negotiate-only $(negotiation_tips "$@") \
      --upload-pack 'unset GIT_TRACE_PACKET; git-upload-pack' \
      "$remote" || :
  done
}


(mkdir no_parents && cd no_parents
  git init -q server && cd server
    commit to_fetch
  cd ..

  (git init -q client && cd client
    for i in $(seq 7); do
      commit c$i
    done
  )

  trace_fetch_baseline file://$PWD/server main
)

(mkdir two_colliding_skips && cd two_colliding_skips
  git init -q server && cd server
    commit to_fetch
  cd ..

  (git init -q client && cd client
    for i in $(seq 11); do
      commit c$i
    done
    git checkout c5
    commit c5side
  )

  trace_fetch_baseline file://$PWD/server HEAD main
)

(mkdir advertisement_as_filter && cd advertisement_as_filter
  git init -q server && cd server
    commit c1
    commit c2
    commit c3
    git tag -d c1 c2 c3
  cd ..
  git clone server client && cd client
    commit c4
    commit c5
    git checkout c4^^
    commit c2side
  cd ..
  (cd server
    git checkout --orphan anotherbranch
    commit to_fetch
  )

  trace_fetch_baseline origin HEAD main
)


(mkdir multi_round && cd multi_round
  git init -q server && cd server
    commit to_fetch
  cd ..

  git init -q client && cd client
    # Eight independent histories with interleaved commit dates force multiple negotiation rounds.
    # Import their 160 commits in one Git process; checkout/add/commit/tag per commit is costly on Windows.
    # The original orphan checkouts retained earlier root files, so root bN contains b1.c0.t through bN.c0.t.
    {
      for ((j = 0; j <= 19; j++)); do
        for ((i = 1; i <= 8; i++)); do
          message=b$i.c$j
          mark=$((j * 8 + i))
          tick
          printf 'commit refs/heads/b%s\nmark :%s\n' "$i" "$mark"
          printf 'author %s <%s> %s\n' "$GIT_AUTHOR_NAME" "$GIT_AUTHOR_EMAIL" "$GIT_AUTHOR_DATE"
          printf 'committer %s <%s> %s\n' "$GIT_COMMITTER_NAME" "$GIT_COMMITTER_EMAIL" "$GIT_COMMITTER_DATE"
          printf 'data %s\n%s\n' "$((${#message} + 1))" "$message"
          if ((j == 0)); then
            for ((k = 1; k < i; k++)); do
              root_message=b$k.c0
              printf 'M 100644 inline %s.t\ndata %s\n%s\n' "$root_message" "$((${#root_message} + 1))" "$root_message"
            done
          fi
          printf 'M 100644 inline %s.t\ndata %s\n%s\n\n' "$message" "$((${#message} + 1))" "$message"
          printf 'reset refs/tags/%s\nfrom :%s\n\n' "$message" "$mark"
        done
      done
    } >.git/multi-round-import
    git fast-import --quiet <.git/multi-round-import
    rm .git/multi-round-import
    git symbolic-ref HEAD refs/heads/b8
    git reset --hard
  cd ..
  (cd server
    git fetch --no-tags "$PWD/../client" b1:refs/heads/b1
    git checkout b1
    commit commit-on-b1
  )
  trace_fetch_baseline file://$PWD/server $(ls client/.git/refs/heads | sort)
)

(mkdir clock_skew && cd clock_skew
  git init -q server && cd server
    commit to_fetch
  cd ..

  (git init -q client && cd client
    tick=2000000000
    commit c1
    commit c2

    tick=1000000000
    git checkout c1
    commit old1
    commit old2
    commit old3
    commit old4
  )

  trace_fetch_baseline file://$PWD/server HEAD main
)
