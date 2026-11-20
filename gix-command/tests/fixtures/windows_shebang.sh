#!/usr/bin/env bash
set -eu -o pipefail

# Git for Windows discards the interpreter directory and options, then resolves
# only .exe interpreters in PATH. An earlier extensionless file cannot shadow one.
mkdir first second third
printf '#!/not/installed/sh --ignored\n' >hook
cp hook first/hook
printf '#!C:\\not-installed\\sh --ignored\n' >backslash-hook
printf '#!/not/installed/sh.exe --ignored\n' >exe-hook
printf '#!/not/installed/python3.11 --ignored\n' >dotted-hook
printf '#!/not/installed/absent\n' >missing-hook
printf 'not executable\n' >first/sh
printf 'executable placeholder\n' >second/sh.exe
printf 'executable placeholder\n' >second/python3.11.exe
printf 'executable placeholder\n' >third/sh.exe

# Native Windows execution ignores the nonexistent Unix interpreter directory.
case "$(uname -s)" in
MINGW*|MSYS*)
  git init -q --bare reference.git
  cat >reference.git/hooks/pre-commit <<'HOOK'
#!/not/installed/sh --ignored
printf '%s\n' "$@"
HOOK
  chmod +x reference.git/hooks/pre-commit
  git -C reference.git hook run pre-commit -- 'one argument' 'literal $value' >reference-stdout 2>reference-output
  ;;
esac
