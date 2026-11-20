#!/usr/bin/env bash
set -eu -o pipefail
export LC_ALL=C

# Each case selects Git's named built-in driver through attributes. The only
# unchanged line is the source candidate, so a zero-context diff reveals its
# capture without depending on a hunk-search heuristic.
mkdir cases
make_case() {
    local driver=$1 name=$2 line=$3
    mkdir "cases/$driver-$name"
    (
        cd "cases/$driver-$name"
        git init -q
        printf 'file diff=%s\n' "$driver" >.gitattributes
        printf '%s' "$driver" >driver
        printf '%s' "$line" >source
        printf '%s\n    before\n' "$line" >file
        git add file
        printf '%s\n    after\n' "$line" >file
        git diff --no-ext-diff --no-color --unified=0 -- file >baseline.diff
    )
}
make_case ada declaration $'\tprocedure Run is'
make_case ada excluded 'procedure Run is new Generic_Run;'
make_case bash declaration $'\tbuild() {'
make_case bash excluded '# build() {'
make_case bibtex declaration '@article{example,'
make_case bibtex excluded 'ordinary text'
make_case cpp declaration 'static int run(int value)'
make_case cpp excluded '    call(value);'
make_case csharp declaration $'\tpublic async Task<int> Run(int value)'
make_case csharp excluded 'if (condition)'
make_case css declaration '.example {'
make_case css excluded 'color: red;'
make_case dts declaration $'\troot_node {'
make_case dts excluded 'property = <1>;'
make_case elixir declaration $'\tdefmodule Example do'
make_case elixir excluded '# defmodule Example do'
make_case fortran declaration '      SUBROUTINE RUN(ARG)'
make_case fortran excluded '      MODULE PROCEDURE RUN'
make_case fountain declaration 'INT. EXAMPLE - DAY'
make_case fountain excluded 'ordinary text'
make_case golang declaration $'\tfunc Run(value int) {'
make_case golang excluded '    value := 1'
make_case html declaration $'\t<h2 class="example">Heading</h2>'
make_case html excluded '<div>ordinary</div>'
make_case ini declaration $'\t[example]'
make_case ini excluded 'key=value'
make_case java declaration $'\tpublic class Example {'
make_case java excluded 'return value;'
make_case kotlin declaration $'\tprivate fun run() {'
make_case kotlin excluded 'return value'
make_case markdown declaration '  ## Heading'
make_case markdown excluded 'ordinary text'
make_case matlab declaration $'\tfunction result = run(value)'
make_case matlab excluded '    value = 1;'
make_case objc declaration '- (void)run:(int)value'
make_case objc excluded 'if (condition)'
make_case pascal declaration 'procedure run(value: integer);'
make_case pascal excluded 'begin'
make_case perl declaration 'package Example;'
make_case perl excluded '    return $value;'
make_case php declaration $'\tpublic function run($value) {'
make_case php excluded '$value = 1;'
make_case python declaration $'\tasync def run(value):'
make_case python excluded '# def run(value):'
make_case r declaration $'\trun <- function(value) {'
make_case r excluded '# run <- function(value)'
make_case ruby declaration $'\tclass Example'
make_case ruby excluded '# class Example'
make_case rust declaration $'\tpub fn run(value: u32) {'
make_case rust excluded '    let value = 1;'
make_case scheme declaration '(define (run value)'
make_case scheme excluded '; ordinary comment'
make_case tex declaration '\subsection{Heading}'
make_case tex excluded '% ordinary comment'
