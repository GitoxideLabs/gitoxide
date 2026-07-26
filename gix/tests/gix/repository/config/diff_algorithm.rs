use gix_diff::blob::Algorithm;

use crate::repository::config::{repo, repo_opts};

#[test]
fn patience_falls_back_to_histogram_when_lenient() -> crate::Result {
    let repo = repo_opts("diff-algorithm-patience", |opts| opts.strict_config(false));
    assert_eq!(
        repo.diff_algorithm()?,
        Algorithm::Histogram,
        "lenient config quietly substitutes the unimplemented 'patience' algorithm with 'histogram'"
    );
    Ok(())
}

#[test]
fn patience_errors_when_strict() {
    let repo = repo("diff-algorithm-patience");
    let err = repo.diff_algorithm().unwrap_err();
    assert_eq!(
        err.to_string(),
        "The 'patience' algorithm is not yet implemented",
        "strict config surfaces the same error the algorithm-key parser produces"
    );
}
