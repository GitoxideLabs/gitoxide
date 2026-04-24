# Shared helper for libFuzzer corpus builder scripts.
#
# Usage from a fuzz target-specific builder:
#
#   root="$(readlink -f -- "$1")"
#   output_corpus="$2"
#   source "$root/etc/fuzz-corpus-builder.sh"
#   build_fuzz_corpus "$root" "$output_corpus" "gix-pack" "index_file"
#
# The target-specific script is invoked by Google OSS Fuzz and
# their build script at https://github.com/google/oss-fuzz/blob/master/projects/gitoxide/build.sh
# with the repository root and the output zip path.
# This helper adds all files from:
#
#   <crate>/fuzz/corpus/<target>/*
#
# If present, it also adds files from:
#
#   <crate>/fuzz/artifacts/<target>/*
#
# Corpus files and artifact files are zipped with `-j`, matching the flat corpus
# archive layout it expects. Empty input groups are skipped so artifact-only
# targets can still produce a corpus archive. Artifacts are added in a separate
# zip invocation so a duplicate basename updates the archive instead of making
# zip fail with "cannot repeat names in zip file".
build_fuzz_corpus() {
    local root="$1"
    local output_corpus="$2"
    local crate="$3"
    local target="$4"
    local fuzz_dir
    local corpus_dir
    local artifacts_dir

    fuzz_dir="$(readlink -f -- "$root/$crate/fuzz")"
    corpus_dir="$fuzz_dir/corpus/$target"
    artifacts_dir="$fuzz_dir/artifacts/$target"

    echo "$root"
    echo "$corpus_dir"
    echo "$artifacts_dir"

    shopt -s nullglob

    local corpus_files=("$corpus_dir"/*)
    if ((${#corpus_files[@]})); then
        zip -j "$output_corpus" "${corpus_files[@]}"
    fi

    if [[ -d "$artifacts_dir" ]]; then
        local artifact_files=("$artifacts_dir"/*)
        if ((${#artifact_files[@]})); then
            # A second zip invocation updates duplicate basenames instead of aborting.
            zip -j "$output_corpus" "${artifact_files[@]}"
        fi
    fi
}
