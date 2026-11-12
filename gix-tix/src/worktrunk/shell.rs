//! Shell wrappers which let a child `tix` process select its caller's working directory.

/// The file into which `tix worktrunk` writes the selected worktree path.
pub(crate) const CD_FILE_ENV: &str = "TIX_WORKTRUNK_CD_FILE";

/// The launch directory while the calling shell waits outside the worktree.
pub(crate) const CWD_ENV: &str = "TIX_WORKTRUNK_CWD";

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
    local cd_file cd_exit exit_code=0 target original_dir="$PWD"
    local previous_dir="${OLDPWD-}" had_previous_dir="${OLDPWD+x}"
    cd_file="$(mktemp)" || return $?
    builtin cd / || { cd_exit=$?; command rm -f "$cd_file"; return "$cd_exit"; }

    TIX_WORKTRUNK_CD_FILE="$cd_file" TIX_WORKTRUNK_CWD="$original_dir" command @WORKTRUNK@ "$@" || exit_code=$?

    target="$original_dir"
    if [[ -s "$cd_file" ]]; then
        target="$(command cat "$cd_file"; printf x)"
        target="${target%x}"
    fi
    command rm -f "$cd_file"

    if [[ -n "$target" ]]; then
        if builtin cd -- "$target"; then
            if [[ "$target" != "$original_dir" ]]; then
                OLDPWD="$original_dir"
            elif [[ -n "$had_previous_dir" ]]; then
                OLDPWD="$previous_dir"
            else
                unset OLDPWD
            fi
        else
            cd_exit=$?
            [[ $exit_code -ne 0 ]] || exit_code=$cd_exit
        fi
    fi
    return "$exit_code"
}
"#;

const FISH: &str = r#"# tix worktrunk shell integration
function wt
    set -l original_dir "$PWD"
    set -l cd_file (mktemp); or return $status
    builtin cd /; or begin
        set -l cd_exit $status
        command rm -f "$cd_file"
        return $cd_exit
    end

    env TIX_WORKTRUNK_CD_FILE="$cd_file" TIX_WORKTRUNK_CWD="$original_dir" @WORKTRUNK@ $argv
    set -l exit_code $status
    set -l target "$original_dir"

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

// shortcut: Nushell retains its native launch CWD; release it when Nushell exposes native chdir.
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
    $original_dir = (Get-Location).ProviderPath
    $handoffDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    [System.IO.Directory]::CreateDirectory($handoffDir) | Out-Null
    $cdFile = Join-Path $handoffDir "cd"
    $oldCdFile = Get-Item Env:\TIX_WORKTRUNK_CD_FILE -ErrorAction SilentlyContinue
    $oldCwd = Get-Item Env:\TIX_WORKTRUNK_CWD -ErrorAction SilentlyContinue
    $exitCode = 1

    try {
        Set-Location -LiteralPath ([System.IO.Path]::GetPathRoot($original_dir)) -ErrorAction Stop
        [System.Environment]::CurrentDirectory = (Get-Location).ProviderPath
        $env:TIX_WORKTRUNK_CD_FILE = $cdFile
        $env:TIX_WORKTRUNK_CWD = $original_dir
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
        if ($null -eq $oldCwd) {
            Remove-Item Env:\TIX_WORKTRUNK_CWD -ErrorAction SilentlyContinue
        } else {
            $env:TIX_WORKTRUNK_CWD = $oldCwd.Value
        }
    }

    $target = $original_dir
    try {
        if ((Test-Path -LiteralPath $cdFile) -and (Get-Item -LiteralPath $cdFile).Length -gt 0) {
            $target = Get-Content -LiteralPath $cdFile -Raw -Encoding UTF8
        }
    }
    finally {
        Remove-Item -LiteralPath $handoffDir -Recurse -ErrorAction SilentlyContinue
    }

    if ($target) {
        try {
            Set-Location -LiteralPath $target -ErrorAction Stop
            [System.Environment]::CurrentDirectory = (Get-Location).ProviderPath
        }
        catch {
            Write-Error $_ -ErrorAction Continue
            if ($exitCode -eq 0) { $exitCode = 1 }
        }
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

    #[cfg(unix)]
    #[test]
    fn wrappers_leave_the_shell_directory_and_restore_it_without_a_handoff() -> gix_testtools::Result {
        use std::{os::unix::fs::PermissionsExt, process::Command};

        let temp = gix_testtools::tempfile::TempDir::new()?;
        let original = temp.path().join("worktree with spaces");
        let selected = temp.path().join("selected\nworktree");
        std::fs::create_dir(&original)?;
        std::fs::create_dir(&selected)?;
        let original = original.canonicalize()?;
        let selected = selected.canonicalize()?;
        let backend = temp.path().join("tix");
        std::fs::write(
            &backend,
            r#"#!/bin/sh
# Simulate Windows refusing deletion while the parent shell holds the worktree.
[ "$PWD" != "$WT_TEST_ORIGINAL" ] || exit 91
[ "$TIX_WORKTRUNK_CWD" = "$WT_TEST_ORIGINAL" ] || exit 92
if [ "$1" = tix ]; then shift; fi
[ "$1" = worktrunk ] && [ "$2" = remove ] && [ "$3" = 'two words' ] && [ "$4" = '*.txt' ] || exit 93
if [ "$WT_TEST_HANDOFF" = yes ]; then
    printf %s "$WT_TEST_SELECTED" > "$TIX_WORKTRUNK_CD_FILE"
fi
exit "$WT_TEST_STATUS"
"#,
        )?;
        std::fs::set_permissions(&backend, std::fs::Permissions::from_mode(0o755))?;
        std::fs::copy(&backend, temp.path().join("gix"))?;
        let path = std::env::join_paths(
            std::iter::once(temp.path().to_owned())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
        )?;

        for (shell, executable, flags, report) in [
            (
                Shell::Bash,
                "bash",
                vec!["--noprofile", "--norc"],
                r#"result=$?; printf '%s\0%s\0%s\0' "$PWD" "$result" "$OLDPWD""#,
            ),
            (
                Shell::Zsh,
                "zsh",
                vec!["-f"],
                r#"result=$?; printf '%s\0%s\0%s\0' "$PWD" "$result" "$OLDPWD""#,
            ),
            (
                Shell::Fish,
                "fish",
                vec!["--no-config"],
                r#"set result $status; printf '%s\0%s\0' "$PWD" "$result""#,
            ),
        ] {
            if Command::new(executable).arg("--version").output().is_err() {
                eprintln!("skipping {shell:?}: {executable} is not installed");
                continue;
            }
            for (backend, status, handoff) in [Backend::Tix, Backend::GixTix].into_iter().flat_map(|backend| {
                [(0, false), (7, false), (7, true)].map(|(status, handoff)| (backend, status, handoff))
            }) {
                let script = format!(
                    "{}\n{}wt remove 'two words' '*.txt'; {report}\n",
                    generate(shell, backend),
                    if shell == Shell::Fish {
                        ""
                    } else {
                        "OLDPWD=/previous-directory\n"
                    }
                );
                let output = Command::new(executable)
                    .args(&flags)
                    .args(["-c", &script])
                    .current_dir(&original)
                    .env("PATH", &path)
                    .env("WT_TEST_ORIGINAL", &original)
                    .env("WT_TEST_SELECTED", &selected)
                    .env("WT_TEST_STATUS", status.to_string())
                    .env("WT_TEST_HANDOFF", if handoff { "yes" } else { "no" })
                    .output()?;
                let mut expected = format!("{}\0{status}\0", if handoff { &selected } else { &original }.display());
                if shell != Shell::Fish {
                    expected.push_str(if handoff {
                        original.to_str().expect("temporary paths are Unicode")
                    } else {
                        "/previous-directory"
                    });
                    expected.push('\0');
                }
                assert_eq!(
                    output.stdout,
                    expected.as_bytes(),
                    "{shell:?} releases the shell CWD and preserves arguments, handoffs and status; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn powershell_releases_its_native_current_directory_before_removal() -> gix_testtools::Result {
        use std::process::Command;

        let temp = gix_testtools::tempfile::TempDir::new()?;
        let original = temp.path().join("worktree with spaces");
        let selected = temp.path().join("main worktree");
        std::fs::create_dir(&original)?;
        std::fs::create_dir(&selected)?;
        std::fs::write(
            temp.path().join("tix.cmd"),
            r#"@echo off
rd "%WT_TEST_ORIGINAL%"
if exist "%WT_TEST_ORIGINAL%" exit /b 91
<nul set /p "=%WT_TEST_SELECTED%" > "%TIX_WORKTRUNK_CD_FILE%"
exit /b 0
"#,
        )?;
        let path = std::env::join_paths(
            std::iter::once(temp.path().to_owned())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
        )?;
        let script = format!(
            "{}\nwt remove; if ($LASTEXITCODE -ne 0) {{ exit $LASTEXITCODE }}; (Get-Location).ProviderPath\n",
            generate(Shell::PowerShell, Backend::Tix)
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .current_dir(&original)
            .env("PATH", path)
            .env("WT_TEST_ORIGINAL", &original)
            .env("WT_TEST_SELECTED", &selected)
            .output()?;
        assert!(
            output.status.success(),
            "PowerShell permits removal of its launch directory: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !original.exists(),
            "the parent shell no longer locks the removed worktree"
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            selected.to_string_lossy()
        );
        Ok(())
    }

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
        assert!(
            powershell.contains("$env:TIX_WORKTRUNK_CWD = $oldCwd.Value"),
            "PowerShell restores the inherited launch directory"
        );
        assert!(
            powershell.find("Set-Location").expect("PowerShell parks the shell")
                < powershell.find("& tix worktrunk").expect("PowerShell invokes tix"),
            "PowerShell releases the shell CWD before invoking tix"
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
