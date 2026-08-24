## Running the diff-slider tests

The tests compare `gix-diff` with Git using data produced by
[diff-slider-tools](https://github.com/mhagger/diff-slider-tools). The commands below don't create or discover
a corpus: they require a `.sliders` file and its corresponding Git repository.

### Prepare the corpus once

From a [diff-slider-tools](https://github.com/mhagger/diff-slider-tools) checkout:

```shell
repo=gitoxide
echo https://github.com/GitoxideLabs/gitoxide >"corpus/$repo.info"
./get-corpus "$repo"
git -C "corpus/$repo.git" log --min-parents=1 --max-parents=1 --format='%P..%H' HEAD |
  ./enumerate-sliders --repo="$repo" >"corpus/$repo.sliders"

export SLIDERS_FILE="$PWD/corpus/$repo.sliders"
export SLIDER_REPOSITORY="$PWD/corpus/$repo.git"
```

`get-corpus` reads `corpus/gitoxide.info` and creates `corpus/gitoxide.git`. `enumerate-sliders` then reads that
repository and creates `corpus/gitoxide.sliders`.

Optionally, see the complete
[upstream corpus instructions](https://github.com/mhagger/diff-slider-tools/blob/b59ed13d7a2a6cfe14a8f79d434b6221cc8b04dd/README.md?plain=1#L122-L146)
for other repositories or commit ranges.

If the corpus already exists, only set its explicit paths:

```shell
export SLIDERS_FILE=/path/to/diff-slider-tools/corpus/gitoxide.sliders
export SLIDER_REPOSITORY=/path/to/diff-slider-tools/corpus/gitoxide.git
```

Then run these two commands from the `gitoxide` root:

```shell
cargo run --package internal-tools -- create-diff-cases \
  --count 2024 \
  --sliders-file "$SLIDERS_FILE" \
  --worktree-dir "$SLIDER_REPOSITORY" \
  --destination-dir gix-diff/tests/fixtures/

cargo test -p gix-diff slider::baseline -- --nocapture
```

`--count` limits how many slider records are read; lower it for a quicker run. With the current `gitoxide`
corpus, `--count 2024` produces 3,094 cases after duplicate blob pairs are removed.

The default report prints all mismatch categories without failing. Set `GIX_DIFF_SLIDER_STRICT=1` to require a
non-empty external baseline and fail on any mismatch. The small built-in baseline always runs strictly.
