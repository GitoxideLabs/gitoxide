// Copyright 2026 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Most declaration patterns and keyword exclusions are adapted from Jujutsu's
// cli/src/source_symbol.rs at 987c084b8f3ff28e8246d8b38c55c323c85611d5:
// https://github.com/jj-vcs/jj/blob/987c084b8f3ff28e8246d8b38c55c323c85611d5/cli/src/source_symbol.rs
// They are translated to POSIX syntax, accept indentation, and capture headers.
// Ada, C++, CSS, Fortran, Fountain, and Swift definitions were written separately.
// Git's named drivers are the compatibility target; fixture comparisons use Git
// as a behavior oracle. These expressions are heuristics, not language parsers.

use super::{Definition, Options, Syntax};

struct Builtin {
    name: &'static str,
    pattern: &'static str,
    case_insensitive: bool,
}

static BUILTINS: &[Builtin] = &[
    Builtin {
        name: "ada",
        pattern: r##"!^[[:blank:]]*(procedure|function|package|protected|task).*\brenames\b
!^[[:blank:]]*(procedure|function|package|protected|task).*\bis[[:blank:]]+(new|separate)\b
^[[:blank:]]*((procedure|function|package|protected|task)[[:blank:]]+.*)$"##,
        case_insensitive: true,
    },
    Builtin {
        name: "bash",
        pattern: r##"^[[:blank:]]*(([A-Za-z_][A-Za-z0-9_]*[[:space:]]*\([[:space:]]*\)|function[[:space:]]+[A-Za-z_][A-Za-z0-9_]*(([[:space:]]*\([[:space:]]*\))|[[:space:]]+)).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "bibtex",
        pattern: r##"^[[:blank:]]*(@[A-Za-z]+[ 	]*\{?[ 	]*[^ 	"@',\\#}{~%]*)"##,
        case_insensitive: false,
    },
    Builtin {
        name: "cpp",
        pattern: r##"!^[[:alpha:]_][[:alnum:]_]*:[[:blank:]]*($|/)
^([[:alpha:]_].*)$
^(::[[:blank:]]*[[:alpha:]_].*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "csharp",
        pattern: r##"!^[[:blank:]]*(do|while|for|foreach|if|else|new|default|return|switch|case|throw|catch|using|lock|fixed)([[:space:]({;]|$)
^[[:blank:]]*((([[:alnum:]_@_.]+(<[[:alnum:]_@_, 	<>]+>)?)([[:space:]]+[[:alnum:]_@_.]+(<[[:alnum:]_@_, 	<>]+>)?)+[[:space:]]*\([^;]*|((static|public|internal|private|protected|new|unsafe|sealed|abstract|partial)[[:space:]]+)*(class|enum|interface|struct|record)[[:space:]]+.*|namespace[[:space:]]+.*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "css",
        pattern: r##"![:;][[:blank:]]*$
^([.#:@[][_a-z0-9].*)$
^([_a-z][_a-z0-9-]*.*)$"##,
        case_insensitive: true,
    },
    Builtin {
        name: "dts",
        pattern: r##"!;
!=
^[[:blank:]]*((/[ 	]*\{|&?[A-Za-z_].*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "elixir",
        pattern: r##"^[[:blank:]]*((def(macro|module|impl|protocol|p)?|test)[[:space:]]+.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "fortran",
        pattern: r##"!^[c*][[:blank:]]
!^[[:blank:]]*!
!^[[:blank:]]*module[[:blank:]]+procedure\b
^[[:blank:]]*((end[[:blank:]]+)?(program|module|block[[:blank:]]+data)[[:blank:]]+[a-z_].*)$
^[[:blank:]]*(([a-z_][[:alnum:]_]*(\([^)]*\)|\*[0-9]+)?[[:blank:]]+)*(subroutine|function)[[:blank:]]+[a-z_].*)$"##,
        case_insensitive: true,
    },
    Builtin {
        name: "fountain",
        pattern: r##"^(\.[^.].*)$
^((INT|EXT|EST|I/E|INT\.?/EXT)[.[:blank:]].*)$"##,
        case_insensitive: true,
    },
    Builtin {
        name: "golang",
        pattern: r##"^[[:blank:]]*((func[[:space:]]*.*(\{[[:space:]]*)?|type[[:space:]]+.*(struct|interface)[[:space:]]*(\{[[:space:]]*)?).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "html",
        pattern: r##"^[[:blank:]]*(<[Hh][1-6]([[:space:]].*)?>.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "ini",
        pattern: r##"^[[:blank:]]*\[[^]]+\]"##,
        case_insensitive: false,
    },
    Builtin {
        name: "java",
        pattern: r##"!^[[:blank:]]*(catch|do|for|if|instanceof|new|return|switch|throw|while)([[:space:]({;]|$)
^[[:blank:]]*((([a-z-]+[[:space:]]+)*(class|enum|interface|record)[[:space:]]+.*|([A-Za-z_<>&][]?&<>.,A-Za-z_0-9]*[[:space:]]+)+[A-Za-z_][A-Za-z_0-9]*[[:space:]]*\([^;]*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "javascript",
        pattern: r##"!^[[:blank:]]*(catch|for|if|switch|while|with)([[:space:]({;]|$)
^[[:blank:]]*(((export|default|async|static|get|set)[[:space:]]+)*(function\b|class\b|[A-Za-z_$][[:alnum:]_$]*[[:space:]]*\([^)]*\)[[:space:]]*\{|[A-Za-z_$][[:alnum:]_$]*[[:space:]]*[:=][[:space:]]*(async[[:space:]]*)?(function\b|\([^)]*\)[[:space:]]*=>)).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "kotlin",
        pattern: r##"^[[:blank:]]*(([a-z]+[[:space:]]+)*(fun|class|interface)[[:space:]]+.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "markdown",
        pattern: r##"^ {0,3}#{1,6}[[:space:]]+.*"##,
        case_insensitive: false,
    },
    Builtin {
        name: "matlab",
        pattern: r##"^[[:blank:]]*(((classdef|function)[[:space:]]+.*|(%%%?|##)[[:space:]]+.*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "objc",
        pattern: r##"!^[[:blank:]]*(do|for|if|else|return|switch|while)([[:space:]({;]|$)
^[[:blank:]]*(([-+][[:space:]]*\([[:space:]]*[A-Za-z_][A-Za-z_0-9* 	]*\)[[:space:]]*[A-Za-z_].*|([A-Za-z_][A-Za-z_0-9]*[[:space:]]+)+[A-Za-z_][A-Za-z_0-9]*[[:space:]]*\([^;]*|@(implementation|interface|protocol)[[:space:]]+.*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "pascal",
        pattern: r##"^[[:blank:]]*((((class[[:space:]]+)?(procedure|function)|constructor|destructor|interface|implementation|initialization|finalization)[[:space:]]*.*|.*=[[:space:]]*(class|record).*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "perl",
        pattern: r##"^((package .*|sub [[:alnum:]_':]+[ 	]*(\([^)]*\)[ 	]*)?(:[^;#]*)?(\{[ 	]*)?(#.*)?|(BEGIN|END|INIT|CHECK|UNITCHECK|AUTOLOAD|DESTROY)[ 	]*(\{[ 	]*)?(#.*)?|=head[0-9] .*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "php",
        pattern: r##"^[[:blank:]]*((((public|protected|private|static|abstract|final)[[:space:]]+)*function.*|(((final|abstract)[[:space:]]+)?class|enum|interface|trait).*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "python",
        pattern: r##"^[[:blank:]]*((async[[:space:]]+def|def|class)[[:space:]]+[[:alnum:]_]+.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "r",
        pattern: r##"^[[:blank:]]*([A-Za-z][A-Za-z0-9_.]*[[:space:]]*(<-|=)[[:space:]]*function.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "ruby",
        pattern: r##"^[[:blank:]]*((class|module|def)[[:space:]]+.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "rust",
        pattern: r##"^[[:blank:]]*(((pub(\([^)]*\))?|const|async|unsafe|extern([[:space:]]+"[^"]+")?)[[:space:]]+)*((fn|struct|enum|impl|trait|mod|type|union)\b|macro_rules![[:space:]]*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "scheme",
        pattern: r##"^[[:blank:]]*((\(((define|def(struct|syntax|class|method|rules|record|proto|alias)?)[-*/ 	]|(library|module|struct|class)[*+ 	]).*|[Dd][Ee][Ff].*).*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "swift",
        pattern: r##"^[[:blank:]]*((@[[:alpha:]_][[:alnum:]_]*(\([^()]*\))?[[:blank:]]+)*([a-z]+[[:blank:]]+)*(func|init|deinit|subscript|class|struct|enum|protocol|extension|actor)\b.*)$"##,
        case_insensitive: false,
    },
    Builtin {
        name: "tex",
        pattern: r##"^(\\((sub)*section|chapter|part)\*?\{.*)$"##,
        case_insensitive: false,
    },
];

impl Definition {
    /// Return a built-in language definition by its case-sensitive driver name.
    ///
    /// Git's built-in names are supported, together with a JavaScript definition.
    /// This lookup does not infer a language from a path or file extension.
    pub fn builtin(name: impl AsRef<[u8]>) -> Option<Self> {
        let name = name.as_ref();
        let entry = BUILTINS.iter().find(|entry| entry.name.as_bytes() == name)?;
        Some(Self {
            pattern: entry.pattern.into(),
            options: Options {
                syntax: Syntax::Extended,
                case_insensitive: entry.case_insensitive,
            },
        })
    }

    /// Iterate over the available built-in driver names, in bytewise sorted order.
    pub fn builtin_names() -> impl Iterator<Item = &'static str> {
        BUILTINS.iter().map(|entry| entry.name)
    }
}
