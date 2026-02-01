# Must be sourced into the main journey test

title plumbing "${kind}"
snapshot="$snapshot/plumbing"
title "gix-tempfile crate"
(when "testing 'gix-tempfile'"
  snapshot="$snapshot/gix-tempfile"
  cd gix-tempfile
  ABORTED=143

  (when "running the example program to raise a signal with a tempfile present"
    it "fails as the process aborts" && {
      expect_run $ABORTED cargo run --features signals --example delete-tempfiles-on-sigterm
    }
    TEMPFILE="$(cargo run --features signals --example delete-tempfiles-on-sigterm 2>/dev/null || true)"
    it "outputs a tempfile with an expected name" && {
      expect_run $SUCCESSFULLY test "$TEMPFILE" = "tempfile.ext"
    }
    it "cleans up the tempfile '$TEMPFILE' it created" && {
      expect_run $WITH_FAILURE test -e "$TEMPFILE"
    }
  )

  (when "running the example program to help assure there cannot be deadlocks"
    ABORTED=134
    it "succeeds as it won't deadlock" && {
      expect_run $ABORTED cargo run --release --features signals --example try-deadlock-on-cleanup -- 1
    }
  )
)

title '`gix` crate'
(when "testing 'gix'"
  snapshot="$snapshot/gix"
  cd gix
  ABORTED=143

  (when "running the example program to check order of signal handlers"
    it "fails as the process aborts" && {
      expect_run $ABORTED cargo run --no-default-features --features interrupt --example interrupt-handler-allows-graceful-shutdown
    }
    it "cleans up the tempfile it created" && {
      expect_run $WITH_FAILURE test -e "example-file.tmp"
    }
  )
  (when "running the example program to check reversibility of signal handlers"
    it "fails as the process aborts" && {
      expect_run $ABORTED cargo run --no-default-features --features interrupt --example reversible-interrupt-handlers
    }
  )
)

title "gix (with repository)"
(with "a git repository"
  snapshot="$snapshot/repository"
  (small-repo-in-sandbox
    (with "the 'verify' sub-command"
      snapshot="$snapshot/verify"
      (with 'human output format'
        it "generates correct output" && {
          WITH_SNAPSHOT="$snapshot/success-format-human" \
          expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format human verify -s
        }
      )
      if test "$kind" = "max" || test "$kind" = "max-pure"; then
      (with "--format json"
        it "generates the correct output in JSON format" && {
          WITH_SNAPSHOT="$snapshot/success-format-json" \
          expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json verify --statistics
        }
      )
      fi
    )
  )

  title "gix remote"
  (when "running 'remote'"
    snapshot="$snapshot/remote"
    title "gix remote refs"
    (with "the 'refs' subcommand"
      snapshot="$snapshot/refs"
      (small-repo-in-sandbox
        if [[ "$kind" != "small" ]]; then

        if [[ "$kind" != "async" ]]; then
        (with "file:// protocol"
          (with "version 1"
            it "generates the correct output" && {
              WITH_SNAPSHOT="$snapshot/file-v-any" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose -c protocol.version=1 remote -n .git refs
            }
          )
          (with "version 2"
            it "generates the correct output" && {
              WITH_SNAPSHOT="$snapshot/file-v-any" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose -c protocol.version=2  remote -n "$PWD" refs
            }
          )
          if test "$kind" = "max" || test "$kind" = "max-pure"; then
          (with "--format json"
            it "generates the correct output in JSON format" && {
              WITH_SNAPSHOT="$snapshot/file-v-any-json" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json remote -n . refs
            }
          )
          fi
        )
        fi

        # for some reason, on CI the daemon always shuts down before we can connect,
        # or isn't actually ready despite having accepted the first connection already.
        (with "git:// protocol"
          launch-git-daemon
          (with "version 1"
            it "generates the correct output" && {
              WITH_SNAPSHOT="$snapshot/file-v-any" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --config protocol.version=1 remote --name git://localhost/ refs
            }
          )
          (with "version 2"
            it "generates the correct output" && {
              WITH_SNAPSHOT="$snapshot/file-v-any" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose -c protocol.version=2 remote -n git://localhost/ refs
            }
          )
        )
        if [[ "$kind" == "small" ]]; then
        (with "https:// protocol (in small builds)"
          it "fails as http is not compiled in" && {
            WITH_SNAPSHOT="$snapshot/fail-http-in-small" \
            expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose -c protocol.version=1 remote -n https://github.com/byron/gitoxide refs
          }
        )
        fi
        (on_ci
          if test "$kind" = "max" || test "$kind" = "max-pure"; then
          (with "https:// protocol"
            (with "version 1"
              it "generates the correct output" && {
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose -c protocol.version=1 remote -n https://github.com/byron/gitoxide refs
              }
            )
            (with "version 2"
              it "generates the correct output" && {
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose -c protocol.version=2 remote -n https://github.com/byron/gitoxide refs
              }
            )
          )
          fi
        )
        else
          it "fails as the CLI doesn't include networking in 'small' mode" && {
            WITH_SNAPSHOT="$snapshot/remote ref-list-no-networking-in-small-failure" \
            expect_run 2 "$exe_plumbing" --no-verbose -c protocol.version=1 remote -n .git refs
          }
        fi
      )
    )
  )
)

title "gix attributes"
(with "gix attributes"
  (with "the 'validate-baseline' sub-command"
    it "passes when operating on all of our files" && {
      expect_run_sh_no_pipefail $SUCCESSFULLY "find . -type f | sed 's|^./||' | $exe_plumbing --no-verbose attributes validate-baseline"
    }
  )
)

title "gix commit-graph"
(when "running 'commit-graph'"
  snapshot="$snapshot/commit-graph"
  title "gix commit-graph verify"
  (with "the 'verify' sub-command"
    snapshot="$snapshot/verify"

    (small-repo-in-sandbox
      (with "a valid and complete commit-graph file"
        git commit-graph write --reachable
        (with "statistics"
          it "generates the correct output" && {
            WITH_SNAPSHOT="$snapshot/statistics-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose commit-graph verify -s
          }
        )
        if test "$kind" = "max" || test "$kind" = "max-pure"; then
        (with "statistics --format json"
          it "generates the correct output" && {
            WITH_SNAPSHOT="$snapshot/statistics-json-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json commit-graph verify -s
          }
        )
        fi
      )
    )
  )
)

(with "gix free"
  snapshot="$snapshot/no-repo"
  title "gix free pack"
  (when "running 'pack'"
    snapshot="$snapshot/pack"

    title "gix free pack receive"
    (with "the 'receive' sub-command"
      snapshot="$snapshot/receive"
      (small-repo-in-sandbox
        if [[ "$kind" != 'small' ]]; then

        if [[ "$kind" != 'async' ]]; then
        (with "file:// protocol"
          (with "version 1"
            (with "NO output directory"
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-no-output" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 .git
              }
            )
            (with "output directory"
              mkdir out
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-with-output" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 .git out/
              }
              it "creates an index and a pack in the output directory" && {
                WITH_SNAPSHOT="$snapshot/ls-in-output-dir" \
                expect_run $SUCCESSFULLY ls out/
              }
              (with "--write-refs set"
                it "generates the correct output" && {
                  WITH_SNAPSHOT="$snapshot/file-v-any-with-output" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 --refs-directory out/all-refs .git out/
                }
                it "writes references into the refs folder of the output directory" && {
                  expect_snapshot "$snapshot/repo-refs" out/all-refs
                }
              )
              rm -Rf out
            )
            if test "$kind" = "max" || test "$kind" = "max-pure"; then
            (with "--format json"
              it "generates the correct output in JSON format" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-no-output-json" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json free pack receive --protocol 1 .git
              }
            )
            fi
          )
          (with "version 2"
            (with "NO output directory"
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-no-output-p2" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 2 .git
              }
            )
            (with "output directory"
              mkdir out/
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-with-output-p2" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive .git out/
              }
              it "creates an index and a pack in the output directory" && {
                WITH_SNAPSHOT="$snapshot/ls-in-output-dir" \
                expect_run $SUCCESSFULLY ls out/
              }
              rm -Rf out
            )
            if test "$kind" = "max" || test "$kind" = "max-pure"; then
            (with "--format json"
              it "generates the correct output in JSON format" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-no-output-json-p2" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json free pack receive --protocol 2 .git
              }
            )
            fi
          )
        )
        fi
        (with "git:// protocol"
          launch-git-daemon
          (with "version 1"
            (with "NO output directory"
              (with "no wanted refs"
                it "generates the correct output" && {
                  WITH_SNAPSHOT="$snapshot/file-v-any-no-output" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 git://localhost/
                }
              )
              (with "wanted refs"
                it "generates the correct output" && {
                  WITH_SNAPSHOT="$snapshot/file-v-any-no-output-wanted-ref-p1" \
                  expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose free pack receive -p 1 git://localhost/ -r =refs/heads/main
                }
              )
            )
            (with "output directory"
              mkdir out
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-with-output" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 git://localhost/ out/
              }
            )
          )
          (with "version 2"
            (with "NO output directory"
              (with "NO wanted refs"
                it "generates the correct output" && {
                  WITH_SNAPSHOT="$snapshot/file-v-any-no-output-p2" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 2 git://localhost/
                }
              )
              (with "wanted refs"
                it "generates the correct output" && {
                  WITH_SNAPSHOT="$snapshot/file-v-any-no-output-single-ref" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 2 git://localhost/ -r refs/heads/main
                }
                (when "ref does not exist"
                  it "fails with a detailed error message including what the server said" && {
                    WITH_SNAPSHOT="$snapshot/file-v-any-no-output-non-existing-single-ref" \
                    expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose free pack receive -p 2 git://localhost/ -r refs/heads/does-not-exist
                  }
                )
              )
            )
            (with "output directory"
              it "generates the correct output" && {
                WITH_SNAPSHOT="$snapshot/file-v-any-with-output-p2" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive git://localhost/ out/
              }
            )
          )
        )
        (on_ci
          if test "$kind" = "max" || test "$kind" = "max-pure"; then
          (with "https:// protocol"
            (with "version 1"
              it "works" && {
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 1 https://github.com/byron/gitoxide
              }
            )
            (with "version 2"
              it "works" && {
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack receive -p 2 https://github.com/byron/gitoxide
              }
            )
          )
          (with "an ambiguous ssh username which could be mistaken for an argument"
            snapshot="$snapshot/fail-ambiguous-username"
            (with "explicit ssh (true url with scheme)"
              it "fails without trying to pass it to command-line programs" && {
                WITH_SNAPSHOT="$snapshot/explicit-ssh" \
                expect_run $WITH_FAILURE "$exe_plumbing" free pack receive 'ssh://-Fconfigfile@foo/bar'
              }
            )
            (with "implicit ssh (special syntax with no scheme)"
              it "fails without trying to pass it to command-line programs" && {
                WITH_SNAPSHOT="$snapshot/implicit-ssh" \
                expect_run $WITH_FAILURE "$exe_plumbing" free pack receive -- '-Fconfigfile@foo:bar/baz'
              }
            )
          )
          (with "an ambiguous ssh host which could be mistaken for an argument"
              it "fails without trying to pass it to command-line programs" && {
                WITH_SNAPSHOT="$snapshot/fail-ambiguous-host" \
                expect_run $WITH_FAILURE "$exe_plumbing" free pack receive 'ssh://-oProxyCommand=open$IFS-aCalculator/foo'
              }
          )
          (with "an ambiguous ssh path which could be mistaken for an argument"
              it "fails without trying to pass it to command-line programs" && {
                WITH_SNAPSHOT="$snapshot/fail-ambiguous-path" \
                expect_run $WITH_FAILURE "$exe_plumbing" free pack receive 'git@foo:-oProxyCommand=open$IFS-aCalculator/bar'
              }
          )
          fi
        )
        elif [[ "$kind" = "small" ]]; then
          it "fails as the CLI doesn't have networking in 'small' mode" && {
            WITH_SNAPSHOT="$snapshot/pack receive-no-networking-in-small-failure" \
            expect_run 2 "$exe_plumbing" --no-verbose free pack receive -p 1 .git
          }
        fi
      )
    )
    if test "$kind" = "max" || test "$kind" = "max-pure"; then
    (with "the 'clone' sub-command"
        snapshot="$snapshot/clone"
        (with "an ambiguous ssh username which could be mistaken for an argument"
          snapshot="$snapshot/fail-ambiguous-username"
          (with "explicit ssh (true url with scheme)"
            it "fails without trying to pass it to command-line programs" && {
              WITH_SNAPSHOT="$snapshot/explicit-ssh" \
              expect_run $WITH_FAILURE "$exe_plumbing" clone 'ssh://-Fconfigfile@foo/bar'
            }
          )
          (with "implicit ssh (special syntax with no scheme)"
            it "fails without trying to pass it to command-line programs" && {
              WITH_SNAPSHOT="$snapshot/implicit-ssh" \
              expect_run $WITH_FAILURE "$exe_plumbing" clone -- '-Fconfigfile@foo:bar/baz'
            }
          )
        )
        (with "an ambiguous ssh host which could be mistaken for an argument"
            it "fails without trying to pass it to command-line programs" && {
              WITH_SNAPSHOT="$snapshot/fail-ambiguous-host" \
              expect_run $WITH_FAILURE "$exe_plumbing" clone 'ssh://-oProxyCommand=open$IFS-aCalculator/foo'
            }
        )
        (with "an ambiguous ssh path which could be mistaken for an argument"
            it "fails without trying to pass it to command-line programs" && {
              WITH_SNAPSHOT="$snapshot/fail-ambiguous-path" \
              expect_run $WITH_FAILURE "$exe_plumbing" clone 'git@foo:-oProxyCommand=open$IFS-aCalculator/bar'
            }
        )
    )
    fi
    (with "the 'index' sub-command"
      snapshot="$snapshot/index"
      title "gix free pack index create"
      (with "the 'create' sub-command"
        snapshot="$snapshot/create"
        PACK_FILE="$fixtures/packs/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
        (with "a valid and complete pack file"
          (with "NO output directory specified"
            (with "pack file passed as file"
              it "generates an index into a sink and outputs pack and index information" && {
                WITH_SNAPSHOT="$snapshot/no-output-dir-success" \
                expect_run $SUCCESSFULLY "$exe_plumbing" free pack index create -p "$PACK_FILE"
              }
            )
            (with "pack file passed from stdin"
              it "generates an index into a sink and outputs pack and index information" && {
                WITH_SNAPSHOT="$snapshot/no-output-dir-success" \
                expect_run $SUCCESSFULLY "$exe_plumbing" free pack index create < "$PACK_FILE"
              }
              if test "$kind" = "max" || test "$kind" = "max-pure"; then
              (with "--format json"
                it "generates the index into a sink and outputs information as JSON" && {
                  WITH_SNAPSHOT="$snapshot/no-output-dir-as-json-success" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --format json free pack index create < "$PACK_FILE"
                }
              )
              fi
            )
          )
          (sandbox
            (with "with an output directory specified"
              it "generates an index and outputs information" && {
                WITH_SNAPSHOT="$snapshot/output-dir-success" \
                expect_run $SUCCESSFULLY "$exe_plumbing" free pack index create -p "$PACK_FILE" "$PWD"
              }
              it "writes the index and pack into the directory (they have the same names, different suffixes)" && {
                WITH_SNAPSHOT="$snapshot/output-dir-content" \
                expect_run $SUCCESSFULLY ls
              }
            )
          )
        )
        (with "'restore' iteration mode"
          (sandbox
            cp "${PACK_FILE}" .
            PACK_FILE="${PACK_FILE##*/}"
            "$jtt" mess-in-the-middle "${PACK_FILE}"

            it "generates an index and outputs information (instead of failing)" && {
              WITH_SNAPSHOT="$snapshot/output-dir-restore-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" free pack index create -i restore -p "$PACK_FILE" "$PWD"
            }

            if test "$kind" = "max" || test "$kind" = "max-pure"; then
            (with "--format json and the very same output directory"
              it "generates the index, overwriting existing files, and outputs information as JSON" && {
                WITH_SNAPSHOT="$snapshot/output-dir-restore-as-json-success" \
                SNAPSHOT_FILTER=remove-paths \
                expect_run $SUCCESSFULLY "$exe_plumbing" --format json free pack index create -i restore $PWD < "$PACK_FILE"
              }
            )
            fi
          )
        )
      )
    )

    title "gix free pack multi-index"
    (with "the 'multi-index' sub-command"
        snapshot="$snapshot/multi-index"
        title "gix free pack multi-index create"
        (with "the 'create' sub-command"
            snapshot="$snapshot/create"
            (with 'multiple pack indices'
              (sandbox
                it "creates a multi-index successfully" && {
                  expect_run $SUCCESSFULLY "$exe_plumbing" free pack multi-index -i multi-pack-index create $fixtures/packs/pack-*.idx
                }
              )
            )
        )
    )

    title "gix free pack explode"
    (with "the 'explode' sub-command"
      snapshot="$snapshot/explode"
      PACK_FILE="$fixtures/packs/pack-11fdfa9e156ab73caae3b6da867192221f2089c2"
      (with "no objects directory specified"
        it "explodes the pack successfully and with desired output" && {
          WITH_SNAPSHOT="$snapshot/to-sink-success" \
          expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose no-repo pack explode "${PACK_FILE}.idx"
        }

        (when "using the --delete-pack flag"
          (sandbox
            (with "a valid pack"
              cp "${PACK_FILE}".idx "${PACK_FILE}".pack .
              PACK_FILE="${PACK_FILE##*/}"
              it "explodes the pack successfully and deletes the original pack and index" && {
                WITH_SNAPSHOT="$snapshot/to-sink-delete-pack-success" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack explode --check skip-file-checksum --delete-pack "${PACK_FILE}.pack"
              }
              it "removes the original files" && {
                expect_run $WITH_FAILURE test -e "${PACK_FILE}".pack
                expect_run $WITH_FAILURE test -e "${PACK_FILE}".idx
              }
            )
            (with "a pack file that is invalid somewhere"
              cp ${PACK_FILE}.idx ${PACK_FILE}.pack .
              PACK_FILE="${PACK_FILE##*/}"
              "$jtt" mess-in-the-middle "${PACK_FILE}".pack

              (with "and all safety checks"
                it "does not explode the file at all" && {
                  WITH_SNAPSHOT="$snapshot/broken-delete-pack-to-sink-failure" \
                  expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose free pack explode --sink-compress --check all --delete-pack "${PACK_FILE}.pack"
                }

                it "did not touch index or pack file" && {
                  expect_exists "${PACK_FILE}".pack
                  expect_exists "${PACK_FILE}".idx
                }
              )

              (with "and no safety checks at all (and an output directory)"
                it "does explode the file" && {
                  WITH_SNAPSHOT="$snapshot/broken-delete-pack-to-sink-skip-checks-success" \
                  expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack explode --verify --check skip-file-and-object-checksum-and-no-abort-on-decode \
                                            --delete-pack "${PACK_FILE}.pack" .
                }

                it "removes the original files" && {
                  expect_run $WITH_FAILURE test -e "${PACK_FILE}".pack
                  expect_run $WITH_FAILURE test -e "${PACK_FILE}".idx
                }

                it "creates all pack objects, but the broken ones" && {
                  WITH_SNAPSHOT="$snapshot/broken-with-objects-dir-skip-checks-success-tree" \
                  expect_run_sh $SUCCESSFULLY 'find . -type f | sort'
                }
              )
            )
          )
        )
      )
      (with "a non-existing directory specified"
        it "fails with a helpful error message" && {
          WITH_SNAPSHOT="$snapshot/missing-objects-dir-fail" \
          expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose free pack explode -c skip-file-and-object-checksum "${PACK_FILE}.idx" does-not-exist
        }
      )
      (with "an existing directory specified"
        (sandbox
          it "succeeds" && {
            WITH_SNAPSHOT="$snapshot/with-objects-dir-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack explode -c skip-file-and-object-checksum-and-no-abort-on-decode \
                                                     "${PACK_FILE}.pack" .
          }

          it "creates all pack objects" && {
            WITH_SNAPSHOT="$snapshot/with-objects-dir-success-tree" \
            expect_run_sh $SUCCESSFULLY 'find . -type f | sort'
          }
        )
      )
    )

    title "gix free pack verify"
    (with "the 'verify' sub-command"
      snapshot="$snapshot/verify"
      (with "a valid pack file"
        PACK_FILE="$fixtures/packs/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
        it "verifies the pack successfully and with desired output" && {
          WITH_SNAPSHOT="$snapshot/success" \
          expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify "$PACK_FILE"
        }
      )
      (with "a valid pack INDEX file"
        MULTI_PACK_INDEX="$fixtures/packs/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.idx"
        (with "no statistics"
          it "verifies the pack index successfully and with desired output" && {
            WITH_SNAPSHOT="$snapshot/index-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify "$MULTI_PACK_INDEX"
          }
        )
        (with "statistics"
          it "verifies the pack index successfully and with desired output" && {
            WITH_SNAPSHOT="$snapshot/index-with-statistics-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --statistics "$MULTI_PACK_INDEX"
          }

          (with "and the less-memory algorithm"
            it "verifies the pack index successfully and with desired output" && {
              WITH_SNAPSHOT="$snapshot/index-with-statistics-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --algorithm less-memory --statistics "$MULTI_PACK_INDEX"
            }
          )
        )
        (with "decode"
          it "verifies the pack index successfully and with desired output, and decodes all objects" && {
            WITH_SNAPSHOT="$snapshot/index-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose  free pack verify --algorithm less-memory --decode "$MULTI_PACK_INDEX"
          }
        )
        (with "re-encode"
          it "verifies the pack index successfully and with desired output, and re-encodes all objects" && {
            WITH_SNAPSHOT="$snapshot/index-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --algorithm less-time --re-encode "$MULTI_PACK_INDEX"
          }
        )
        if test "$kind" = "max" || test "$kind" = "max-pure"; then
        (with "statistics (JSON)"
          it "verifies the pack index successfully and with desired output" && {
            WITH_SNAPSHOT="$snapshot/index-with-statistics-json-success" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json --threads 1 free pack verify --statistics "$MULTI_PACK_INDEX"
          }
        )
        fi
      )
      (with "a valid multi-pack index"
        snapshot="$snapshot/multi-index"
        (sandbox
          MULTI_PACK_INDEX=multi-pack-index
          cp $fixtures/packs/pack-* .
          $exe_plumbing free pack multi-index -i $MULTI_PACK_INDEX create *.idx

          (when "using fast validation via 'pack multi-index verify'"
            it "verifies the pack index successfully and with desired output" && {
              WITH_SNAPSHOT="$snapshot/fast-index-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack multi-index -i "$MULTI_PACK_INDEX" verify
            }
          )

          (with "no statistics"
            it "verifies the pack index successfully and with desired output" && {
              WITH_SNAPSHOT="$snapshot/index-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify "$MULTI_PACK_INDEX"
            }
          )
          (with "statistics"
            it "verifies the pack index successfully and with desired output" && {
              WITH_SNAPSHOT="$snapshot/index-with-statistics-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --statistics "$MULTI_PACK_INDEX"
            }

            (with "and the less-memory algorithm"
              it "verifies the pack index successfully and with desired output" && {
                WITH_SNAPSHOT="$snapshot/index-with-statistics-success" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --algorithm less-memory --statistics "$MULTI_PACK_INDEX"
              }
            )
          )
          (with "decode"
            it "verifies the pack index successfully and with desired output, and decodes all objects" && {
              WITH_SNAPSHOT="$snapshot/index-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --algorithm less-memory --decode "$MULTI_PACK_INDEX"
            }
          )
          (with "re-encode"
            it "verifies the pack index successfully and with desired output, and re-encodes all objects" && {
              WITH_SNAPSHOT="$snapshot/index-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose free pack verify --algorithm less-time --re-encode "$MULTI_PACK_INDEX"
            }
          )
          if test "$kind" = "max" || test "$kind" = "max-pure"; then
          (with "statistics (JSON)"
            it "verifies the pack index successfully and with desired output" && {
              WITH_SNAPSHOT="$snapshot/index-with-statistics-json-success" \
              expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose --format json --threads 1 free pack verify --statistics "$MULTI_PACK_INDEX"
            }
          )
          fi
        )
      )
      (sandbox
        (with "an INvalid pack INDEX file"
          MULTI_PACK_INDEX="$fixtures/packs/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.idx"
          cp $MULTI_PACK_INDEX index.idx
          echo $'\0' >> index.idx
          it "fails to verify the pack index and with desired output" && {
            WITH_SNAPSHOT="$snapshot/index-failure" \
            expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose free pack verify index.idx
          }
        )
      )
    )
  )
)

if [[ "$kind" != "small" && "$kind" != "async" ]]; then
# Testing repository - local reproduction of https://github.com/staehle/gitoxide-testing
testrepo_name="gitoxide-testing"
# Path relative to tests/fixtures/ for use with jtt run-script
testrepo_fixture_script="scripts/make_gitoxide_testing_repo.sh"
# This repo has various tags with noted differences in README.md, all in form `vN`
# `v5` is latest on main and should be the default cloned
testrepo_v5_tag="v5"
testrepo_v4_tag="v4"
testrepo_v3_tag="v3"
testrepo_v2_tag="v2"
testrepo_v1_tag="v1"
# This file exists in all versions, but with different content:
testrepo_common_file_name="version.txt"
# gix options:
gix_clone_blobless="--filter=blob:none"
gix_clone_limit="--filter=blob:limit=1024"

title "gix clone (functional tests)"
(when "running functional clone tests"
  title "gix clone with partial clone filters"
  (with "blobless clone of $testrepo_name repository"
    snapshot="$snapshot/blobless-clone"
    testrepo_path="${testrepo_name}-bare-blobless"
    testworktree_path="${testrepo_name}-worktree"

    (sandbox
      # Create/reuse the source repository using the fixture script (cached read-only)
      testrepo_source=$("$jtt" run-script "$root/.." "$testrepo_fixture_script")
      testrepo_url="file://${testrepo_source}"

      # Resolve commit hashes from tags dynamically
      testrepo_v5_commit=$(cd "${testrepo_source}" && git rev-parse ${testrepo_v5_tag})
      testrepo_v4_commit=$(cd "${testrepo_source}" && git rev-parse ${testrepo_v4_tag})
      testrepo_v3_commit=$(cd "${testrepo_source}" && git rev-parse ${testrepo_v3_tag})
      testrepo_v2_commit=$(cd "${testrepo_source}" && git rev-parse ${testrepo_v2_tag})
      testrepo_v1_commit=$(cd "${testrepo_source}" && git rev-parse ${testrepo_v1_tag})

      # Test blobless bare clone with --filter=blob:none
      it "creates a blobless (${gix_clone_blobless}) bare clone successfully" && {
        expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose clone --bare ${gix_clone_blobless} ${testrepo_url} ${testrepo_path}
      }
      (with "the cloned blobless bare repository"
        (cd ${testrepo_path}
          it "should be a bare repository" && {
            expect_run $SUCCESSFULLY test -f HEAD
            expect_run $SUCCESSFULLY test -d objects
            expect_run $SUCCESSFULLY test -d refs
            expect_run $WITH_FAILURE test -d .git
          }
          it "gix can see remote.origin.partialclonefilter configuration" && {
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose config remote.origin.promisor
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose config remote.origin.partialclonefilter
          }
          it "real git client also sees partialclonefilter" && {
            WITH_SNAPSHOT="$snapshot/filter-config" \
            expect_run $SUCCESSFULLY git --no-pager config remote.origin.partialclonefilter
          }
          it "tag ${testrepo_v5_tag} has appropriate tree entries (all missing)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v5_tag}-tree-entries-missing" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v5_tag}
          }
          it "tag ${testrepo_v4_tag} has appropriate tree entries (all missing)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v4_tag}-tree-entries-missing" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v4_tag}
          }
          it "tag ${testrepo_v2_tag} has appropriate tree entries (all missing)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v2_tag}-tree-entries-missing" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v2_tag}
          }
          it "real git client can detect all the missing blobs" && {
            WITH_SNAPSHOT="$snapshot/git-reported-missing-blobs" \
            expect_run $SUCCESSFULLY git --no-pager rev-list --objects --quiet --missing=print HEAD
          }
        )
      )

      # Test creating a worktree from the bare blobless clone, using non-default tag
      it "can create a worktree from the bare blobless clone for tag ${testrepo_v4_tag}" && {
        (cd ${testrepo_path}
          # Once gix supports 'worktree add', we can use that directly:
          # expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose worktree add ../${testworktree_path} ${testrepo_v4_tag}
          # Until then, use the real git client:
          expect_run $SUCCESSFULLY git --no-pager worktree add ../${testworktree_path} ${testrepo_v4_tag}
        )
      }

      (with "the created worktree (on tag ${testrepo_v4_tag})"
        (cd ${testworktree_path}
          it "should be a valid worktree" && {
            expect_run $SUCCESSFULLY test -f .git
            expect_run $SUCCESSFULLY test -f ${testrepo_common_file_name}
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose status
          }

          WORKTREE_HEAD=$("$exe_plumbing" --no-verbose rev resolve HEAD)
          it "HEAD should NOT match the bare repo" && {
            BARE_HEAD=$(cd ../${testrepo_path} && "$exe_plumbing" --no-verbose rev resolve HEAD)
            expect_run $SUCCESSFULLY test "$BARE_HEAD" != "$WORKTREE_HEAD"
          }
          it "HEAD should match the tag ${testrepo_v4_tag}'s commit" && {
            expect_run $SUCCESSFULLY test "$WORKTREE_HEAD" = "${testrepo_v4_commit}"
          }

          it "should pass fsck validation" && {
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose fsck
          }

          it "real git client can detect all the missing blobs for ${testrepo_v4_tag}" && {
            WITH_SNAPSHOT="$snapshot/git-reported-missing-blobs-${testrepo_v4_tag}" \
            expect_run $SUCCESSFULLY git --no-pager rev-list --objects --quiet --missing=print ${testrepo_v4_tag}
          }
          it "real git client can detect all the missing blobs for ${testrepo_v5_tag}" && {
            WITH_SNAPSHOT="$snapshot/git-reported-missing-blobs-${testrepo_v5_tag}" \
            expect_run $SUCCESSFULLY git --no-pager rev-list --objects --quiet --missing=print ${testrepo_v5_tag}
          }
          it "tag ${testrepo_v4_tag} has appropriate tree entries (all populated)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v4_tag}-tree-entries-populated" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v4_tag}
          }
          it "tag ${testrepo_v5_tag} has appropriate tree entries (partially populated)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v5_tag}-tree-entries-partial" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v5_tag}
          }
          it "tag ${testrepo_v2_tag} has appropriate tree entries (partially populated)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v2_tag}-tree-entries-partial" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v2_tag}
          }

          it "accessing blobs for '${testrepo_common_file_name}' from different commits..." && {
            (with "the gix client"
              # try to access the current version.txt blob
              it "succeeds in finding the ${testrepo_v4_tag} tag's blob for this file" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v4_tag}" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose cat HEAD:${testrepo_common_file_name}
              }
              # try to access an old revision of ${testrepo_common_file_name}
              # the gix plumbing version should not have the blob, so it should fail
              it "fails to find a blob for an older tag '${testrepo_v2_tag}' (${testrepo_v2_commit}) as we are blobless" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v2_tag}-blobless-failure" \
                expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose cat ${testrepo_v2_commit}:${testrepo_common_file_name}
              }
            )
            (with "the git client"
              # however, the real git client should be able to fetch the old revision:
              it "real git succeeds in finding the ${testrepo_v2_tag} blob as it fetches by default" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v2_tag}" \
                expect_run $SUCCESSFULLY git --no-pager cat-file blob ${testrepo_v2_commit}:${testrepo_common_file_name}
              }
            )
            (with "the gix client (round 2)"
              # try to access the newly fetched by the real git client ${testrepo_v2_commit} blob
              it "succeeds in finding the ${testrepo_v2_tag} blob just populated by real git" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v2_tag}" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose cat ${testrepo_v2_commit}:${testrepo_common_file_name}
              }
              ## Now, we're going to do the same thing again, but for an even older revision, except use
              ## the new gix explicit fetch functionality:
              it "fails to find a blob for an older tag ${testrepo_v1_tag} (${testrepo_v1_commit})" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v1_tag}-blobless-failure" \
                expect_run $WITH_FAILURE "$exe_plumbing" --no-verbose cat ${testrepo_v1_commit}:${testrepo_common_file_name}
              }
              it "gix explicit fetch succeeds (and hopefully gets all blobs for this commit)" && {
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose fetch ${testrepo_v1_commit}
              }
              it "succeeds in finding the ${testrepo_v1_tag} blob" && {
                WITH_SNAPSHOT="$snapshot/cat-${testrepo_v1_tag}" \
                expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose cat ${testrepo_v1_commit}:${testrepo_common_file_name}
              }
            )
          }

          it "tag ${testrepo_v2_tag} has appropriate tree entries (more partially populated)" && {
            WITH_SNAPSHOT="$snapshot/${testrepo_v2_tag}-tree-entries-partial2" \
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose tree entries -r -e ${testrepo_v2_tag}
          }
        )
      )
    )
  )

  (with "blob-limit clone of gitoxide repository"
    snapshot="$snapshot/bloblimit-clone"
    testrepo_path="${testrepo_name}-bare-bloblimit"

    (sandbox
      # Create/reuse the source repository using the fixture script (cached read-only)
      testrepo_source=$("$jtt" run-script "$root/.." "$testrepo_fixture_script")
      testrepo_url="file://${testrepo_source}"

      # Test blob-limit (--filter=blob:limit=1024) bare clone
      it "creates a blob-limit (${gix_clone_limit}) bare clone successfully" && {
        expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose clone --bare ${gix_clone_limit} ${testrepo_url} ${testrepo_path}
      }

      (with "the blob-limit cloned repository"
        (cd ${testrepo_path}
          it "should be a bare repository" && {
            expect_run $SUCCESSFULLY test -f HEAD
            expect_run $SUCCESSFULLY test -d objects
            expect_run $SUCCESSFULLY test -d refs
            expect_run $WITH_FAILURE test -d .git
          }

          it "should have partial clone configuration with blob limit" && {
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose config remote.origin.promisor
            expect_run $SUCCESSFULLY "$exe_plumbing" --no-verbose config remote.origin.partialclonefilter
          }

          it "should have the expected blob limit filter configuration" && {
            WITH_SNAPSHOT="$snapshot/blob-limit-config" \
            expect_run $SUCCESSFULLY git --no-pager config remote.origin.partialclonefilter
          }
        )
      )
    )
  )
)
fi
