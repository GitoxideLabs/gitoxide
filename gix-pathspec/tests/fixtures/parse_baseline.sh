#!/usr/bin/env bash
set -eu -o pipefail

git init;

function baseline() {
    local pathspec=$1 # first argument is the pathspec to test

    if [[ -n ${MSYSTEM:-} && ( $pathspec == ': :some/path' || $pathspec == '\!a' ) ]]; then
        # Git for Windows rejects these repository-format pathspecs while the
        # platform-independent parser accepts them, as does Git on Unix.
        status=0
    else
        git ls-files "$pathspec" && status=0 || status=$?
    fi
    {
        echo "$pathspec"
        echo "$status"
    } >> baseline.git
}

# success

# special 'there is no pathspec' spec
baseline ':'

# repeated_matcher_keywords
baseline ':(glob,glob)'
baseline ':(literal,literal)'
baseline ':(top,top)'
baseline ':(icase,icase)'
baseline ':(attr,attr)'
baseline ':!^(exclude,exclude)'

# empty_signatures
baseline '.'
baseline ':'
baseline 'some/path'
baseline ':some/path'
baseline ':()some/path'
baseline '::some/path'
baseline ':::some/path'
baseline ':():some/path'

# whitespace_in_pathspec
baseline ' some/path'
baseline 'some/ path'
baseline 'some/path '
baseline ': some/path'
baseline ': !some/path'
baseline ': :some/path'
baseline ': ()some/path'
baseline ':! some/path'

# short_signatures
baseline ':/some/path'
baseline '://some/path'
baseline ':^some/path'
baseline ':^^some/path'
baseline ':!some/path'
baseline ':!!some/path'
baseline ':/!some/path'
baseline ':!/^/:some/path'

# signatures_and_searchmodes
baseline ':(top)'
baseline ':(icase)'
baseline ':(attr)'
baseline ':(exclude)'
baseline ':(literal)'
baseline ':(glob)'
baseline ':(top,exclude)'
baseline ':(icase,literal)'
baseline ':!(literal)some/*path'
baseline ':(top,literal,icase,attr,exclude)some/path'
baseline ':(top,glob,icase,attr,exclude)some/path'

# empty_keywords_are_ignored
baseline ':(top,)some/path'
baseline ':(,top)some/path'
baseline ':(top,,icase)some/path'
baseline ':(,)some/path'
baseline ':(,,)some/path'
baseline ':(icase,)some/path'
baseline ':(attr:someAttr,)some/path'

# attributes_in_signature
baseline ':(attr:someAttr)'
baseline ':(attr:!someAttr)'
baseline ':(attr:-someAttr)'
baseline ':(attr:someAttr=value)'
baseline ':(attr:a=one b=)'
baseline ':(attr:a= b=two)'
baseline ':(attr:a=one b=two)'
baseline ':(attr:a=one   b=two)'
baseline ':(attr:someAttr anotherAttr)'
baseline ':(attr:builtin_objectmode)'

# attributes_with_escape_chars_in_state_values
baseline ':(attr:v=one\-)'
baseline ':(attr:v=one\_)'
baseline ':(attr:v=one\,)'
baseline ':(attr:v=one\,two\,three)'
baseline ':(attr:a=\d b= c=\d)'

# failing

#empty_input
baseline ""

# invalid_short_signatures
baseline ':"()'
baseline ':#()'
baseline ':%()'
baseline ':&()'
baseline ":'()"
baseline ':,()'
baseline ':-()'
baseline ':;()'
baseline ':<()'
baseline ':=()'
baseline ':>()'
baseline ':@()'
baseline ':_()'
baseline ':`()'
baseline ':~()'

# invalid_keywords
baseline ':( )some/path'
baseline ':(tp)some/path'
baseline ':(top, exclude)some/path'
baseline ':(top,exclude,icse)some/path'

# invalid_attributes
baseline ':(attr:+invalidAttr)some/path'
baseline ':(attr:validAttr +invalidAttr)some/path'
baseline ':(attr:+invalidAttr,attr:valid)some/path'
baseline ':(attr:inva\lid)some/path'
baseline $':(attr:a\tb)some/path'
baseline $':(attr:a\rb)some/path'
baseline $':(attr:a=one\tb=two)some/path'
baseline ':(attr:!a=b)some/path'
baseline ':(attr:-a=b)some/path'

# invalid_attribute_values
baseline ':(attr:v=inva#lid)some/path'
baseline ':(attr:v=inva\\lid)some/path'
baseline ':(attr:v=invalid\\)some/path'
baseline ':(attr:v=invalid\#)some/path'
baseline ':(attr:v=inva\=lid)some/path'
baseline ':(attr:a=valid b=inva\#lid)some/path'
baseline ':(attr:v=val��)'
baseline ':(attr:pr=pre��x:,)�'

# escape_character_at_end_of_attribute_value
baseline ':(attr:v=invalid\)some/path'
baseline ':(attr:v=invalid\ )some/path'
baseline ':(attr:v=invalid\ valid)some/path'

# empty_attribute_specification
baseline ':(attr:)'

# multiple_attribute_specifications
baseline ':(attr:one,attr:two)some/path'

# missing_parentheses
baseline ':(top'

# glob_and_literal_keywords_present
baseline ':(glob,literal)some/path'
# trailing slash
baseline ':(glob,literal)some/path/'
baseline 'some/path/'
baseline 'path/'

baseline 'a/b/'
baseline 'a/'
baseline '!a'
baseline '\!a'
