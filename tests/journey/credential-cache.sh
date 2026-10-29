# Must be sourced into the main journey test. Native credential caching uses Unix sockets.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) return 0 ;;
esac

title "native credential cache"
(with "a cache daemon independent of Git and the client process"
  sandbox
  mkdir no-programs
  socket="$PWD/cache/socket"
  trap '"$exe_plumbing" credential-cache --socket "$socket" exit; popd >/dev/null' EXIT
  cat >credential <<'EOF'
protocol=https
host=cache.example
username=alice
password=secret

EOF
  cat >query <<'EOF'
protocol=https
host=cache.example

EOF
  cat >expected <<'EOF'
capability[]=authtype
username=alice
password=secret
EOF

  it "launches its own daemon and returns after readiness without Git on PATH" && {
    expect_run $SUCCESSFULLY env PATH="$PWD/no-programs" "$exe_plumbing" \
      credential-cache --socket "$socket" store <credential
    "$exe_plumbing" credential-cache --socket "$socket" get <query >actual
    expect_run $SUCCESSFULLY cmp expected actual
    git credential-cache --socket "$socket" get <query >actual
    expect_run $SUCCESSFULLY cmp expected actual
    expect_run $SUCCESSFULLY "$exe_plumbing" credential-cache --socket "$socket" exit
  }

  it "passes the launcher through repository credential configuration" && {
    git init -q repo
    expect_run $SUCCESSFULLY env PATH="$PWD/no-programs" "$exe_plumbing" -r repo \
      -c "credential.helper=cache --socket=$socket" credential approve <credential
    "$exe_plumbing" credential-cache --socket "$socket" get <query >actual
    expect_run $SUCCESSFULLY cmp expected actual
    expect_run $SUCCESSFULLY "$exe_plumbing" credential-cache --socket "$socket" exit
  }

  it "passes the launcher and overrides through global-only credential configuration" && {
    expect_run $SUCCESSFULLY env PATH="$PWD/no-programs" "$exe_plumbing" \
      -c "credential.helper=cache --socket=$socket" credential approve <credential
    "$exe_plumbing" credential-cache --socket "$socket" get <query >actual
    expect_run $SUCCESSFULLY cmp expected actual
    expect_run $SUCCESSFULLY "$exe_plumbing" credential-cache --socket "$socket" exit
  }

  for policy in false true local global; do
    ignore_sighup="$policy"
    it "uses the $policy SIGHUP policy before announcing readiness" && {
      case "$policy" in
        local)
          git -C repo config credentialCache.ignoreSighup true
          ignore_sighup=false
          "$exe_plumbing" -r repo credential-cache--daemon "$socket" >ready &
          ;;
        global)
          git config --file global-config credentialCache.ignoreSighup true
          ignore_sighup=true
          GIT_CONFIG_GLOBAL="$PWD/global-config" "$exe_plumbing" \
            credential-cache--daemon "$socket" >ready &
          ;;
        *)
          "$exe_plumbing" -c "credentialCache.ignoreSighup=$policy" \
            credential-cache--daemon "$socket" >ready &
          ;;
      esac
      daemon_pid=$!
      for attempt in {1..100}; do
        test -s ready && break
        sleep 0.05
      done
      expect_run $SUCCESSFULLY test "$(cat ready)" = ok
      expect_run $SUCCESSFULLY kill -HUP "$daemon_pid"
      if test "$ignore_sighup" = true; then
        "$exe_plumbing" credential-cache --socket "$socket" store <credential
        "$exe_plumbing" credential-cache --socket "$socket" get <query >actual
        expect_run $SUCCESSFULLY cmp expected actual
        expect_run $SUCCESSFULLY "$exe_plumbing" credential-cache --socket "$socket" exit
      else
        for attempt in {1..100}; do
          test ! -e "$socket" && break
          sleep 0.05
        done
      fi
      expect_run $SUCCESSFULLY test ! -e "$socket"
      wait "$daemon_pid"
      rm ready
    }
  done
)
