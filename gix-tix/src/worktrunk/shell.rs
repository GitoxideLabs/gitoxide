//! Shell wrappers which let a child `tix` process select its caller's working directory.

/// The file into which `tix worktrunk` writes the selected worktree path.
pub(crate) const CD_FILE_ENV: &str = "TIX_WORKTRUNK_CD_FILE";

/// A shell supported by `worktrunk shell-init`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
#[expect(clippy::enum_variant_names, reason = "PowerShell is the shell's proper name")]
pub(crate) enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(alias = "nu")]
    Nushell,
    #[value(name = "powershell", alias = "pwsh")]
    PowerShell,
}

/// The executable form through which the worktrunk command was invoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Backend {
    Tix,
    GixTix,
}

impl Backend {
    fn worktrunk(self) -> &'static str {
        match self {
            Backend::Tix => "tix worktrunk",
            Backend::GixTix => "gix tix worktrunk",
        }
    }
}

/// Generate a `wt` wrapper for `shell` which invokes `backend`.
pub(crate) fn generate(shell: Shell, backend: Backend) -> String {
    let template = match shell {
        Shell::Bash | Shell::Zsh => POSIX,
        Shell::Fish => FISH,
        Shell::Nushell => NUSHELL,
        Shell::PowerShell => POWERSHELL,
    };
    template.replace("@WORKTRUNK@", backend.worktrunk())
}

const POSIX: &str = r#"# tix worktrunk shell integration
wt() {
    local cd_file cd_exit exit_code=0 target
    cd_file="$(mktemp)" || return $?

    TIX_WORKTRUNK_CD_FILE="$cd_file" command @WORKTRUNK@ "$@" || exit_code=$?

    if [[ -s "$cd_file" ]]; then
        target="$(command cat "$cd_file"; printf x)"
        target="${target%x}"
    fi
    command rm -f "$cd_file"

    if [[ -n "$target" ]]; then
        builtin cd -- "$target" || {
            cd_exit=$?
            [[ $exit_code -ne 0 ]] || exit_code=$cd_exit
        }
    fi
    return "$exit_code"
}
"#;

const FISH: &str = r#"# tix worktrunk shell integration
function wt
    set -l cd_file (mktemp); or return $status

    env TIX_WORKTRUNK_CD_FILE="$cd_file" @WORKTRUNK@ $argv
    set -l exit_code $status
    set -l target

    if test -s "$cd_file"
        set target (string collect -N < "$cd_file")
    end
    command rm -f "$cd_file"

    if test -n "$target"
        builtin cd -- "$target"
        set -l cd_exit $status
        if test $exit_code -eq 0
            set exit_code $cd_exit
        end
    end
    return $exit_code
end
"#;

const NUSHELL: &str = r#"# tix worktrunk shell integration
export def --env --wrapped wt [...args] {
    let handoff_dir = (mktemp --directory)
    let cd_file = ($handoff_dir | path join cd)
    let exit_code = (try {
        with-env {
            TIX_WORKTRUNK_CD_FILE: $cd_file,
        } {
            ^@WORKTRUNK@ ...$args
        }
        0
    } catch {
        $env.LAST_EXIT_CODE? | default 1
    })

    let target = if ($cd_file | path exists) and (($cd_file | path type) == "file") {
        open $cd_file --raw | decode utf-8
    } else {
        ""
    }

    if $nu.os-info.family == "windows" {
        try { rm -rf $handoff_dir }
    } else {
        try { ^rm -rf $handoff_dir }
    }

    if ($target | is-not-empty) {
        cd $target
    }
    if $exit_code != 0 {
        if $nu.os-info.family == "windows" {
            ^cmd.exe /c $"exit ($exit_code)"
        } else {
            ^sh -c $"exit ($exit_code)"
        }
    }
}
"#;

const POWERSHELL: &str = r#"# tix worktrunk shell integration
function wt {
    $handoffDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    [System.IO.Directory]::CreateDirectory($handoffDir) | Out-Null
    $cdFile = Join-Path $handoffDir "cd"
    $oldCdFile = Get-Item Env:\TIX_WORKTRUNK_CD_FILE -ErrorAction SilentlyContinue
    $exitCode = 1

    try {
        $env:TIX_WORKTRUNK_CD_FILE = $cdFile
        & @WORKTRUNK@ @args
        $exitCode = $LASTEXITCODE
    }
    catch {
        Write-Error $_ -ErrorAction Continue
    }
    finally {
        if ($null -eq $oldCdFile) {
            Remove-Item Env:\TIX_WORKTRUNK_CD_FILE -ErrorAction SilentlyContinue
        } else {
            $env:TIX_WORKTRUNK_CD_FILE = $oldCdFile.Value
        }
    }

    $target = $null
    try {
        if ((Test-Path -LiteralPath $cdFile) -and (Get-Item -LiteralPath $cdFile).Length -gt 0) {
            $target = Get-Content -LiteralPath $cdFile -Raw -Encoding UTF8
        }
    }
    finally {
        Remove-Item -LiteralPath $handoffDir -Recurse -ErrorAction SilentlyContinue
    }

    if ($target) {
        Set-Location -LiteralPath $target
        if (-not $? -and $exitCode -eq 0) { $exitCode = 1 }
    }
    $global:LASTEXITCODE = $exitCode
    if ($exitCode -ne 0) {
        Write-Error "wt exited with code $exitCode" -ErrorAction SilentlyContinue
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shell_uses_one_handoff_file_and_cleans_it_up() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Nushell, Shell::PowerShell] {
            let script = generate(shell, Backend::Tix);
            assert!(script.contains(CD_FILE_ENV), "{shell:?} passes the raw-path file");
            assert!(script.contains("tix worktrunk"), "{shell:?} invokes the picker");
            assert!(!script.contains("fullscreen"), "{shell:?} does not relaunch tix");
            assert!(
                script.contains("rm") || script.contains("Remove-Item"),
                "{shell:?} cleans temporary files"
            );
        }
    }

    #[test]
    fn backend_controls_the_worktrunk_invocation() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Nushell, Shell::PowerShell] {
            let script = generate(shell, Backend::GixTix);
            assert!(
                script.contains("gix tix worktrunk"),
                "{shell:?} invokes the gix subcommand"
            );
            assert!(
                !script.contains("@WORKTRUNK@"),
                "{shell:?} has no unresolved picker placeholder"
            );
        }
    }

    #[test]
    fn wrappers_survive_strict_shells_and_restore_caller_state() {
        for shell in [Shell::Bash, Shell::Zsh] {
            let script = generate(shell, Backend::Tix);
            assert!(
                script.contains("command tix worktrunk \"$@\" || exit_code=$?"),
                "{shell:?} captures failures even under errexit"
            );
        }
        let powershell = generate(Shell::PowerShell, Backend::Tix);
        assert!(
            powershell.contains("$env:TIX_WORKTRUNK_CD_FILE = $oldCdFile.Value"),
            "PowerShell restores inherited handoff variables"
        );
    }

    #[test]
    fn wrappers_honor_a_directory_handoff_after_command_failure() {
        let posix = generate(Shell::Bash, Backend::Tix);
        assert!(posix.contains("if [[ -s \"$cd_file\" ]]"));
        assert!(posix.contains("if [[ -n \"$target\" ]]"));

        let fish = generate(Shell::Fish, Backend::Tix);
        assert!(fish.contains("if test -s \"$cd_file\""));
        assert!(fish.contains("if test -n \"$target\""));

        let nushell = generate(Shell::Nushell, Backend::Tix);
        assert!(!nushell.contains("if $exit_code == 0 and ($cd_file | path exists)"));
        assert!(
            nushell.find("cd $target").expect("Nushell changes directory")
                < nushell.find("if $exit_code != 0").expect("Nushell restores failure")
        );

        let powershell = generate(Shell::PowerShell, Backend::Tix);
        assert!(!powershell.contains("if ($exitCode -eq 0 -and (Test-Path -LiteralPath $cdFile)"));
        assert!(powershell.contains("if ($target)"));
    }
}
