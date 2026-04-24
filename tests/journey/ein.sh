# Must be sourced into the main journey test


function remove-thread-id {
  sed -E 's/ \([0-9]+\)//'
}

if test "$kind" = "max" || test "$kind" = "max-pure"; then
title "Porcelain ${kind}"
(
  (when "running a debug-only panic test"
    snapshot="$snapshot/panic-behaviour"
    (with "the --quiet option set"
      it "fails as expected" && {
        WITH_SNAPSHOT="$snapshot/expected-failure" \
        SNAPSHOT_FILTER=remove-thread-id \
        expect_run_sh 0 "$exe -q panic"
      }
    )

    (with "NO --quiet option set"
      it "fails as expected" && {
        WITH_SNAPSHOT="$snapshot/expected-failure-in-thread" \
        SNAPSHOT_FILTER=remove-thread-id \
        expect_run_sh 0 "$exe panic"
      }
    )
    (not_on_ci # due to different TTY settings, the output differs, it's OK for now
      (with "progress option set"
        it "fails as expected" && {
          WITH_SNAPSHOT="$snapshot/expected-failure-in-thread-with-progress" \
        SNAPSHOT_FILTER=remove-thread-id \
          expect_run_sh 0 "$exe --progress panic"
        }
      )
    )
  )
  snapshot="$snapshot/porcelain"
  (when "using the 'tool' subcommand"
    title "ein tool"
    (with "a repo with a tiny commit history"
      (small-repo-in-sandbox
        title "ein tool estimate-hours"
        (when "running 'estimate-hours'"
          snapshot="$snapshot/estimate-hours"
          (with "no arguments"
            it "succeeds and prints only a summary" && {
              WITH_SNAPSHOT="$snapshot/no-args-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours 2>/dev/null"
            }
          )
          (with "the show-pii argument"
            it "succeeds and shows information identifying people before the summary" && {
              WITH_SNAPSHOT="$snapshot/show-pii-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours --show-pii 2>/dev/null"
            }
          )
          (with "the omit-unify-identities argument"
            it "succeeds and doesn't show unified identities (in this case there is only one author anyway)" && {
              WITH_SNAPSHOT="$snapshot/no-unify-identities-success" \
              expect_run_sh $SUCCESSFULLY "$exe t estimate-hours --omit-unify-identities 2>/dev/null"
            }
          )
          (with "the --file-stats argument"
            it "succeeds and shows file statistics" && {
              WITH_SNAPSHOT="$snapshot/file-stats-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours --file-stats 2>/dev/null"
            }
          )
          (with "the --line-stats argument"
            it "succeeds and shows line statistics" && {
              WITH_SNAPSHOT="$snapshot/line-stats-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours --line-stats 2>/dev/null"
            }
          )
          (with "all --stats arguments and pii"
            it "succeeds and shows all statistics" && {
              WITH_SNAPSHOT="$snapshot/all-stats-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours -pfl 2>/dev/null"
            }
          )
          (with "co-authored-by trailers and pii"
            (sandbox
              git init &>/dev/null
              git checkout -b main &>/dev/null
              git config commit.gpgsign false
              git config tag.gpgsign false
              touch a
              git add a
              git commit -m "first" &>/dev/null
              echo hi >> a
              git add a
              git commit -m "$(printf 'second\n\nCo-authored-by: Co Author <co@example.com>')" &>/dev/null
              it "counts co-authors as additional authors" && {
                WITH_SNAPSHOT="$snapshot/co-authored-by-success" \
                expect_run_sh $SUCCESSFULLY "$exe tool estimate-hours --show-pii 2>/dev/null"
              }
            )
          )
          (with "a branch name that doesn't exist"
            it "fails and shows a decent enough error message" && {
              WITH_SNAPSHOT="$snapshot/invalid-branch-name-failure" \
              expect_run_sh $WITH_FAILURE "$exe -q t estimate-hours . foobar"
            }
          )
        )
      )
    )
    (with "a mix of repositories"
      (sandbox
        repo-with-remotes dir/one-origin origin https://example.com/one-origin
        repo-with-remotes origin-and-fork origin https://example.com/origin-and-fork fork https://example.com/other/origin-and-fork
        repo-with-remotes special-origin special-name https://example.com/special-origin
        repo-with-remotes no-origin
        repo-with-remotes a-non-bare-repo-with-extension.git origin https://example.com/a-repo-with-extension.git
        snapshot="$snapshot/tool"

        title "ein tool find"
        (when "running 'find'"
          snapshot="$snapshot/find"
          (with "no arguments"
            it "succeeds and prints a list of repository work directories" && {
              WITH_SNAPSHOT="$snapshot/no-args-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool find 2>/dev/null"
            }
          )
        )
        title "ein tool organize"
        (when "running 'organize'"
          snapshot="$snapshot/organize"
          (with "no arguments"
            it "succeeds and informs about the operations that it WOULD do" && {
              WITH_SNAPSHOT="$snapshot/no-args-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize 2>/dev/null"
            }

            it "does not change the directory structure at all" && {
              WITH_SNAPSHOT="$snapshot/initial-directory-structure" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 | sort'
            }
          )

          (with "--execute"
            it "succeeds" && {
              WITH_SNAPSHOT="$snapshot/execute-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize --execute 2>/dev/null"
            }

            it "changes the directory structure" && {
              WITH_SNAPSHOT="$snapshot/directory-structure-after-organize" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 | sort'
            }
          )

          (with "--execute again"
            it "succeeds" && {
              WITH_SNAPSHOT="$snapshot/execute-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize --execute 2>/dev/null"
            }

            it "does not alter the directory structure as these are already in place" && {
              WITH_SNAPSHOT="$snapshot/directory-structure-after-organize" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 | sort'
            }
          )

          (with "--execute with move into subdirectory of itself"
            (cd example.com
              rm -Rf a-repo-with-extension origin-and-fork
              mv one-origin ../
              cd ..
              rmdir example.com && mv one-origin example.com
            )
            it "succeeds" && {
              WITH_SNAPSHOT="$snapshot/execute-success-new-root" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize --execute 2>/dev/null"
            }

            it "does alter the directory structure as these are already in place" && {
              WITH_SNAPSHOT="$snapshot/directory-structure-after-organize-to-new-root" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 -type d | sort'
            }
          )
        )
        if test "$kind" != "max-pure"; then
        (with "running with no further arguments"
          it "succeeds and informs about possible operations" && {
            WITH_SNAPSHOT="$snapshot/no-args-failure" \
            expect_run_sh $WITH_CLAP_FAILURE "$exe t"
          }
        )
        fi
      )
    )
    (with "a bare repository"
      (sandbox
        bare-repo-with-remotes bare-origin.git origin https://example.com/bare-origin
        snapshot="$snapshot/tool-bare"

        title "ein tool organize (bare)"
        (when "running 'organize'"
          snapshot="$snapshot/organize"
          (with "no arguments"
            it "succeeds and informs about the operations that it WOULD do" && {
              WITH_SNAPSHOT="$snapshot/no-args-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize 2>/dev/null"
            }

            it "does not change the directory structure at all" && {
              WITH_SNAPSHOT="$snapshot/initial-directory-structure" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 -type d | sort'
            }
          )

          (with "--execute"
            it "succeeds" && {
              WITH_SNAPSHOT="$snapshot/execute-success" \
              expect_run_sh $SUCCESSFULLY "$exe tool organize --execute 2>/dev/null"
            }

            it "moves the bare repository into the remote-host tree" && {
              WITH_SNAPSHOT="$snapshot/directory-structure-after-organize" \
              expect_run_sh $SUCCESSFULLY 'find . -maxdepth 2 -type d | sort'
            }
          )
        )
      )
    )
  )

  title "ein init"
  (when "running 'init'"
    snapshot="$snapshot/init"
    (with "no argument"
      (with "an empty directory"
        (sandbox
          it "succeeds" && {
            WITH_SNAPSHOT="$snapshot/success" \
            expect_run $SUCCESSFULLY "$exe" init
          }

          (when "trying to initialize the same directory again"
            it "fails" && {
              WITH_SNAPSHOT="$snapshot/fail" \
              expect_run $WITH_FAILURE "$exe" init
            }
          )
        )
      )
    )
    (with "a single argument denoting the directory to initialize"
      DIR=foo/bar
      (with "a multi-element directory: $DIR"
        (sandbox
          it "succeeds" && {
            WITH_SNAPSHOT="$snapshot/success-with-multi-element-directory" \
            expect_run $SUCCESSFULLY "$exe" init $DIR
          }

          (when "trying to initialize the same directory again"
            it "fails" && {
              WITH_SNAPSHOT="$snapshot/fail-with-multi-element-directory" \
              expect_run $WITH_FAILURE "$exe" init $DIR
            }
          )
        )
      )
    )
  )
)
fi
