## How to run diff-slider tests

The idea is to use https://github.com/mhagger/diff-slider-tools to create slider information for use to generate a test which invokes our own implementation and compares it to Git itself.
Follow these instructions to set it up.

1. DIFF_SLIDER_TOOLS=/your/anticipated/path/to/diff-slider-tools
2. `git clone https://github.com/mhagger/diff-slider-tools $DIFF_SLIDER_TOOLS`
3. `pushd $DIFF_SLIDER_TOOLS`
4. Follow [these instructions](https://github.com/mhagger/diff-slider-tools/blob/b59ed13d7a2a6cfe14a8f79d434b6221cc8b04dd/README.md?plain=1#L122-L146) to     generate a file containing the slider information. Be sure to set the `repo` variable as it's used in later script invocations.
   - Note that `get-corpus` must be run with `./get-corpus`.
   - You can use an existing repository, for instance via `repo=git-human`, so there is no need to find your own repository to test.
   - The script suite is very slow, and it's recommended to only set a range of commits, or use a small repository for testing.

Finally, run the `internal-tools` program to turn that file into a fixture called `make_diff_for_sliders_repo.sh`.

```shell
# run inside `gitoxide`
popd
cargo run --package internal-tools -- \
  create-diff-cases \
    --sliders-file $DIFF_SLIDER_TOOLS/corpus/$repo.sliders \
    --worktree-dir $DIFF_SLIDER_TOOLS/corpus/$repo.git/ \
    --destination-dir gix-diff/tests/fixtures/
```

Finally, run `cargo test -p gix-diff sliders -- --nocapture` to execute the actual tests to compare.
