#!/usr/bin/env bash
set -eu -o pipefail

# Git is a reference, not the specification for known bugs. Each intentional difference
# below names its derivation and regression test. GIX_DIFF records signed seconds relative
# to an accepted Git date; GIX_VALUE records an independently derived timestamp (also when
# Git rejects it); GIX_REJECT checks that invalid/unsupported input is rejected.

# Keep local-timezone fallbacks reproducible on every machine.
export TZ=UTC

git init

function baseline() {
    local test_date="$1" # first argument is the date to test
    local test_name="$2" # format name, or GIX_DIFF:<seconds> for an intentional timestamp difference

    # Use Git's strict date parser, as commit dates must not fall back to approxidate.
    local status=0 ident seconds
    ident=$(GIT_AUTHOR_DATE="$test_date" git var GIT_AUTHOR_IDENT) || status="$?"

    {
      echo "$test_date"
      echo "$test_name"
      echo "$status"
      if [ "$status" = 0 ]; then
        seconds="${ident##*> }"
        echo "${seconds% *}"
      else
        echo '-1'
      fi
      echo ''
    } >> baseline.git
}

# Relative dates use a fixed "now" timestamp for reproducibility. The optional third argument can
# override the default 1000000000 (Sun Sep 9 01:46:40 UTC 2001) for calendar edge cases.
function baseline_relative() {
    local test_date="$1" # first argument is the relative date to test
    local test_name="$2" # second argument is the format name (usually empty for relative dates)
    local now="${3:-1000000000}" # case-specific reference time, recorded as the fifth field

    local status=0
    GIT_TEST_DATE_NOW="$now" git -c section.key="$test_date" config --type=expiry-date section.key || status="$?"

    {
      echo "$test_date"
      echo "$test_name"
      echo "$status"
      if [ "$status" = 0 ]; then
        GIT_TEST_DATE_NOW="$now" git -c section.key="$test_date" config --type=expiry-date section.key
      else
        echo '-1'
      fi
      if [ "$TZ" = UTC ]; then
        echo "$now"
      else
        echo "$now $TZ"
      fi
    } >> baseline.git
}

# ============================================================================
# FIXED DATE FORMATS
# ============================================================================
# Tests from https://github.com/git/git/blob/master/t/t0006-date.sh

# A date without a clock retains the current local time. Use the fixed-now helper
# to make Git's fallback into approxidate reproducible.
for date in 2008-12-01 2009-12-01 1979-02-26; do
    baseline_relative "$date" '' 1251660000
done

# Intentional difference: short dates keep the requested clock using target-era rules,
# with the earlier occurrence in folds and forward normalization in gaps. Git retains
# the reference tm_isdst through mktime, shifting these clocks by an hour (or half an hour).
TZ=America/New_York baseline_relative '2024-07-15' 'GIX_DIFF:-3600' 1736953200
TZ=America/New_York baseline_relative '2024-01-15' 'GIX_DIFF:3600' 1752588000
TZ=Australia/Lord_Howe baseline_relative '2024-01-15' 'GIX_DIFF:-1800' 1752535800
TZ=Australia/Lord_Howe baseline_relative '2024-07-15' 'GIX_DIFF:1800' 1736895600
TZ=America/New_York baseline_relative '2024-11-03' 'GIX_DIFF:-3600' 1736922600
TZ=America/New_York baseline_relative '2024-11-03' '' 1752557400
TZ=America/New_York baseline_relative '2024-03-10' '' 1736926200
TZ=America/New_York baseline_relative '2024-03-10' 'GIX_DIFF:3600' 1752561000
# A permanent offset change without DST must keep the target date's local clock.
TZ=Pacific/Apia baseline_relative '2010-07-15' '' 1736888400
# Britain used a +0100 base offset throughout 1970; keep 15:00 instead of Git's 14:00.
TZ=Europe/London baseline_relative '1970-01-15' 'GIX_DIFF:3600' 1752588000

# RFC2822 format: "Day, DD Mon YYYY HH:MM:SS +/-ZZZZ"
baseline 'Thu, 18 Aug 2022 12:45:06 +0800' 'RFC2822'
baseline 'Sat, 01 Jan 2000 00:00:00 +0000' 'RFC2822'
baseline 'Fri, 13 Feb 2009 23:31:30 +0000' 'RFC2822'  # Unix timestamp 1234567890
baseline 'Wed, 15 Jun 2016 16:13:20 +0200' 'RFC2822'  # from git t0006
baseline 'Thu, 7 Apr 2005 15:14:13 -0700' ''  # from git t0006

# Complete textual dates exercise Git's month-name matching independently of RFC 2822.
# Weekdays are ignored, and ordinal suffixes need not agree with the day number.
for date in 'February 14, 2008' 'February 14th, 2008' '14th February 2008' \
            'Monday, fEbRu 14st, 2008'; do
    baseline "$date 20:30:45 -0500" ''
done
# Intentional rejection: February 29 in a common year and February 31 are invalid dates,
# not requests for March 1 or March 2. See correctness::invalid_calendar_dates_meridians_and_future_directions_are_rejected.
baseline 'Feb 29 2009 20:30:45 -0500' 'GIX_REJECT'
baseline 'Feb 31 2008 20:30:45 -0500' 'GIX_REJECT'
baseline 'Feb 14 20:30:45 2008 -0500' ''
# Meridians adjust the parsed clock, month punctuation is harmless, and fractions are discarded.
for date in 'Feb 14 2008 8:30 PM' 'Feb 14 2008 8:30pm' \
            'Feb 14 2008 12:30 AM' 'Feb 14 2008 12:30 PM' \
            'Feb. 14, 2008 20:30:45' 'Feb 14 2008 20:30:45.123' \
            'Feb 14 2008 8:30:45.123pm'; do
    baseline "$date -0500" ''
done
baseline 'February 14th, 2008 at 20:30:45 -0500' ''
baseline 'on February 14th, 2008 at exactly 20:30:45 -0500' ''
baseline 'February 14th, 2008 at 20:30:45 CET' ''
baseline 'February 14 2008 20:30 -05' ''
baseline 'February 14 2008 20:30:45 -05:00' ''
baseline 'February 14 2008 20:30:45 CET' ''
baseline 'Feb 14 2008 20:30:45CET' ''
baseline 'Feb 14 20:30:45CET 2008' ''
baseline 'Feb 14 20:30:45.123CET 2008' ''
TZ=America/New_York baseline_relative 'February 14 2008 20:30:45 UT' '' 1251660000
TZ=America/New_York baseline_relative 'March 10 2024 02:30:00' '' 1710055800
baseline 'February 14 2008 20:30:45 Z' ''
baseline 'February 14 2008 20:30:45 +2359' ''
baseline 'February 14 2008 20:30:45 -2359' ''
# Intentional difference: -0001 is minus one minute, not an unset timezone sentinel.
# Retain 20:30:45 -0001 as 20:31:45 UTC. See correctness::explicit_offsets_are_never_unset_sentinels.
baseline 'February 14 2008 20:30:45 -0001' 'GIX_DIFF:60'
baseline 'Feb 14 2008 24:00:00 +0000' ''
# Intentional rejection: Time cannot represent leap seconds; reject second 60 instead
# of shifting this supplied date into the following day. See iso_end_of_day_is_normalized.
baseline 'Feb 14 2008 23:59:60 +0000' 'GIX_REJECT'
for month in January February March April May June July August September October November December; do
    for ((length=3; length<=${#month}; length++)); do
        baseline "${month:0:length} 14th, 2008 20:30:45 +0000" ''
    done
done

# Standalone years in textual dates use match_digit(), not set_date()'s wider
# numeric-date pivot. These shared cases retain their historical interpretation;
# gix-date also accepts 10..69 below instead of copying Git's unexplained gap.
for year in 00 01 02 03 04 05 06 07 08 09 {70..99}; do
    baseline "February 14 $year 20:30:45 -0500" ''
    baseline "14th February $year 20:30:45 -0500" ''
    baseline "Feb 14 20:30:45 $year -0500" ''
done

# Absolute textual parsing precedes approxidate, even without a trailing zone.
# Thus year 00 means 2000 when a day has been supplied.
for date in 'June 7 00 12:34:56' 'June 7 12:34:56 00' 'June 7 2009 12:34:56' \
            '7th June 12:34:56 2009' \
            'June 7 2009 +0200 12:34:56' 'June 7 2009 CET 12:34:56'; do
    baseline "$date" ''
done
# Intentional difference: default an absent day to June 1, rather than Git's unset sentinel (May 30).
for date in 'June 2008 12:34:56' 'June 12:34:56 2008' \
            'June 12:34:56 2008 -0500' 'June 2008 +0200 12:34:56'; do
    baseline "$date" 'GIX_DIFF:172800'
done

# GIT_RFC2822 format: like RFC2822 but with non-padded day
baseline 'Thu, 1 Aug 2022 12:45:06 +0800' ''
baseline 'Sat, 1 Jan 2000 00:00:00 +0000' ''

# ISO8601 format: "YYYY-MM-DD HH:MM:SS +/-ZZZZ" from git t0006
baseline '2022-08-17 22:04:58 +0200' 'ISO8601'
baseline '2000-01-01 00:00:00 +0000' 'ISO8601'
baseline '1970-01-01 00:00:00 +0000' 'ISO8601'
baseline '2008-02-14 20:30:45 +0000' ''  # from git t0006
baseline '2008-02-14 20:30:45 -0500' ''  # from git t0006
baseline '2016-06-15 16:13:20 +0200' 'ISO8601'  # from git t0006

# ISO8601 with dots: "YYYY.MM.DD HH:MM:SS +/-ZZZZ" from git t0006
baseline '2008.02.14 20:30:45 -0500' ''

# Git prefers month/day with slashes and day/month with dots, and accepts year-first
# variants. Include ambiguous dates and cases requiring the alternate ordering.
for date in 2008/02/14 02/14/2008 14.02.2008 02/03/2008 02.03.2008 \
            14/02/2008 02.14.2008 2008/14/02 2008.14.02 2008/2/3 3.2.2008; do
    baseline "$date 20:30:45 -0500" ''
done

# Numeric years in Git's accepted ranges keep their existing interpretation.
# The extra ranges of the consistent 70 pivot are covered explicitly below.
# The date precedes the clock so these complete dates don't depend on "now".
for date in 02/03/08 02.03.08 14/02/08 02.14.08 \
            02/14/00 02/14/10 02/14/37 02/14/71 02/14/99 \
            99/02/14 99/14/02 71.02.14 71.14.02 08/02/14 08.02.14 \
            2/14/0 2/14/8 14.2.000 14.2.008 2/14/071 071/2/14; do
    baseline "$date 20:30:45 +0000" ''
done
baseline '02/14/08 20:30:45 -0500' ''
baseline '14.02.08 20:30:45 -0500' ''


# Intentional year support: use the full 00..69 -> 2000..2069 / 70..99 -> 1970..1999
# pivot and literal four-digit years. Derive these instants from February 14 at 20:30:45 UTC.
# Git rejects 38..70 in numeric dates, misinterprets textual 10..69, and limits textual
# four-digit years to 1970..2099. See correctness::textual_and_numeric_years_share_a_complete_pivot_and_literal_years.
baseline '2/14/38 20:30:45 +0000' 'GIX_VALUE:2149792245' # year 2038
baseline 'Feb 14 38 20:30:45 +0000' 'GIX_VALUE:2149792245' # year 2038
baseline '2/14/69 20:30:45 +0000' 'GIX_VALUE:3128099445' # year 2069
baseline 'Feb 14 69 20:30:45 +0000' 'GIX_VALUE:3128099445' # year 2069
baseline '2/14/70 20:30:45 +0000' 'GIX_VALUE:3875445' # year 1970
baseline 'Feb 14 70 20:30:45 +0000' 'GIX_VALUE:3875445' # year 1970
baseline '2/14/1950 20:30:45 +0000' 'GIX_VALUE:-627276555' # year 1950
baseline 'Feb 14 1950 20:30:45 +0000' 'GIX_VALUE:-627276555' # year 1950
baseline '2/14/2100 20:30:45 +0000' 'GIX_VALUE:4106320245' # year 2100
baseline 'Feb 14 2100 20:30:45 +0000' 'GIX_VALUE:4106320245' # year 2100
baseline '2/14/0008 20:30:45 +0000' 'GIX_VALUE:-61910882955' # year 8
baseline 'Feb 14 0008 20:30:45 +0000' 'GIX_VALUE:-61910882955' # year 8

# ISO8601_STRICT format: "YYYY-MM-DDTHH:MM:SS+ZZ:ZZ"
baseline '2022-08-17T21:43:13+08:00' 'ISO8601_STRICT'
baseline '2000-01-01T00:00:00+00:00' 'ISO8601_STRICT'
baseline '2009-02-13T23:31:30+00:00' 'ISO8601_STRICT'  # Unix timestamp 1234567890
baseline '2016-06-15T16:13:20+02:00' 'ISO8601_STRICT'  # from git t0006

# Z suffix for UTC timezone from git t0006
baseline '1970-01-01 00:00:00 Z' ''

# Compact ISO8601 formats from git t0006 (YYYYMMDDTHHMMSS)
# Intentional extension across Git versions: complete compact clocks name unambiguous
# instants, even on hosts where Git rejects them. Derive these from February 14, 2008 UTC.
# See compact_iso8601's full_format, with_colons_in_time, hour_minute_only and hour_only tests.
baseline '20080214T20:30:45' 'GIX_VALUE:1203021045'
baseline '20080214T203045' 'GIX_VALUE:1203021045'
baseline '20080214T2030' 'GIX_VALUE:1203021000'
baseline '20080214T20' 'GIX_VALUE:1203019200'
baseline '20080214T203045-04:00' ''

# Short compact times must split off the timezone after HHMM or HH.
baseline '20080214T2030-04:00' ''
baseline '20080214T2030-0400' ''
baseline '20080214T2030+05:30' ''
baseline '20080214T20-0400' ''

# Subsecond precision (Git ignores the subseconds)
baseline '2008-02-14 20:30:45.019-04:00' ''

# Various timezone formats from git t0006
baseline '2008-02-14 20:30:45 -0015' ''  # 15-minute offset
baseline '2008-02-14 20:30:45 -05' ''    # 2-digit hour offset
baseline '2008-02-14 20:30:45 -05:00' '' # colon-separated offset
baseline '2008-02-14 20:30:45 +00' ''    # 2-digit +00

# Git falls back to UTC here for offsets beyond ±23:59. Jiff can honor them instead.
# Subtract the full offset, not UTC: +24 hours means the previous day's noon.
# See offsets_preserve_jiffs_range_and_precision.
baseline '2022-01-01 12:00:00 +2359' 'ISO8601'
baseline '2022-01-01 12:00:00 -2359' 'ISO8601'
baseline '2022-01-01 12:00:00 +2400' 'GIX_DIFF:-86400'
baseline '2022-01-01 12:00:00 -2400' 'GIX_DIFF:86400'
baseline '2022-01-01T12:00:00+24:00' 'GIX_DIFF:-86400'
baseline '2022-01-01 12:00:00 +2559' 'GIX_DIFF:-93540'
# Git ignores offset seconds, while Jiff preserves the more precise instant (three seconds
# earlier/later than minute precision). See offsets_preserve_jiffs_range_and_precision.
baseline '2008-02-14T20:30:45+01:02:03' 'GIX_DIFF:-3'
baseline '2008-02-14T20:30:45-01:02:03' 'GIX_DIFF:3'

# Git's named timezone table also applies to ISO dates; RFC 2822 alone treats unfamiliar
# abbreviations as UTC. Cover every alias, including mixed case, in both input formats.
for zone in IDLW NT CAT HST HDT YST YDT PST PDT MST MDT CST CDT EST EDT AST ADT WAT \
            GMT UTC Z WET BST CET MET MEWT MEST CEST MESZ FWT FST EET EEST \
            WAST WADT CCT JST EAST EADT GST NZT NZST NZDT IDLE cet CeSt z; do
    baseline "2008-02-14 20:30:45 $zone" ''
    baseline "Thu, 14 Feb 2008 20:30:45 $zone" ''
done
baseline '2008-02-14 20:30:45CET' ''
baseline 'Thu, 14 Feb 2008 20:30:45cet' ''
baseline 'Thu, 14 Feb 2008 20:30:45 UT' ''

# Timezone edge cases from git t0006
baseline '1970-01-01 00:00:00 +0000' ''
baseline '1970-01-01 01:00:00 +0100' ''
baseline '1970-01-02 00:00:00 +1100' ''

# DEFAULT format (Git's default): "Day Mon D HH:MM:SS YYYY +/-ZZZZ"
baseline 'Thu Sep 04 2022 10:45:06 -0400' '' # cannot round-trip, incorrect day-of-week
baseline 'Sun Sep 04 2022 10:45:06 -0400' 'GITOXIDE'
baseline 'Thu Aug 18 12:45:06 2022 +0800' ''
baseline 'Wed Jun 15 16:13:20 2016 +0200' ''  # from git t0006

# Leading/trailing whitespace must work uniformly across parser branches.
baseline '  1234567890  ' ''
baseline '  @1234567890  ' ''
baseline '  @1660874655 +0800  ' ''
baseline '  Thu, 18 Aug 2022 12:45:06 +0800  ' ''
baseline '  2022-08-17T21:43:13+08:00  ' ''

# UNIX timestamp format
# Note: Git only treats numbers >= 100000000 as UNIX timestamps.
# Smaller numbers are interpreted as date components.
baseline '1234567890' 'UNIX'
baseline '100000000' 'UNIX'
baseline '946684800' 'UNIX'  # 2000-01-01 00:00:00 UTC
baseline '1466000000' 'UNIX'  # from git t0006

# Bare numbers below the epoch threshold must not silently become timestamps (#3001).
# Leading zeroes do not turn a smaller value into an epoch timestamp.
for test_date in 20080214 19700101 99999999 020080214 0 -1000; do
    baseline "$test_date" ''
done

# RAW format: "SECONDS +/-ZZZZ"
# Note: Git only treats timestamps >= 100000000 as raw format.
# Smaller numbers are interpreted as date components.
baseline '1660874655 +0800' 'RAW'
baseline '1660874655 -0800' 'RAW'
baseline '100000000 +0000' 'RAW'
baseline '1234567890 +0000' 'RAW'
baseline '946684800 +0000' 'RAW'
baseline '1466000000 +0200' 'RAW'  # from git t0006
baseline '1466000000 -0200' 'RAW'  # from git t0006

# Raw timestamps allow every minute offset, including offsets beyond fourteen hours.
# Round-tripping the unprefixed form checks that its offset survives, not just its epoch.
for offset in +0001 -0059 +1234 +1500 +2359 -2359; do
    baseline "1660874655 $offset" 'RAW'
    baseline "@1660874655 $offset" ''
done

# Git accepts a leading `@` before either of the two forms above. Re-formatting is not checked,
# as the `@` isn't reproduced.
baseline '@1234567890' ''
baseline '@100000000' ''
baseline '@1660874655 +0800' ''
baseline '@1466000000 -0200' ''
baseline '@0 +0000' ''
baseline '@20080214 +0000' ''
baseline '@99999999 +0000' ''

# Intentional extension: epoch seconds before 1970 remain representable, instead of
# Git's rejection. See correctness::signed_epochs_are_exact_and_reference_free.
baseline '@-1000' 'GIX_VALUE:-1000'
baseline '-1000 +0100' 'GIX_VALUE:-1000'

# ============================================================================
# RELATIVE DATE FORMATS from git t0006
# ============================================================================
# These tests use GIT_TEST_DATE_NOW=1000000000 (Sun Sep 9 01:46:40 UTC 2001)

# Named
# Expiry-date treats `now` as a sentinel. `today` requires Git 2.55 (covered below).
baseline_relative 'yesterday' ''

# `never` resets all calendar/clock fields and clears a pending count. These cases
# avoid named clocks after a fixed date, whose behavior changed in Git 2.55.
for date in never NEVER '1 day never' '1 never' 1never 'noon never' 'never now' 'now never'; do
    baseline_relative "$date" '' 1251660000
done
# Intentional difference: the .3.days count applies after an epoch reset too, yielding
# 1969-12-29 12:34:56 UTC instead of Git's 1970-01-01. See never_resets_to_the_unix_epoch.
baseline_relative 'never 12:34:56.3.days.ago' 'GIX_DIFF:-259200' 1251660000

# Seconds - from git t0006 check_relative
baseline_relative '1 second ago' ''
baseline_relative '2 seconds ago' ''
baseline_relative '30 seconds ago' ''
baseline_relative '5 seconds ago' ''  # from git t0006 check_relative 5
baseline_relative '150 seconds ago' ''  # from git t0006 check_relative 5

# Minutes - from git t0006 check_relative 300 = 5 minutes
baseline_relative '1 minute ago' ''
baseline_relative '2 minutes ago' ''
baseline_relative '30 minutes ago' ''
baseline_relative '5 minutes ago' ''
baseline_relative '10 minutes ago' ''
baseline_relative '90 minutes ago' ''

# Hours - from git t0006 check_relative 18000 = 5 hours
baseline_relative '1 hour ago' ''
baseline_relative '2 hours ago' ''
baseline_relative '12 hours ago' ''
baseline_relative '5 hours ago' ''
baseline_relative '72 hours ago' ''

# Days - from git t0006 check_relative 432000 = 5 days
baseline_relative '1 day ago' ''
baseline_relative '2 days ago' ''
baseline_relative '7 days ago' ''
baseline_relative '5 days ago' ''
baseline_relative '3 days ago' ''
baseline_relative '100 days ago' ''

# Weeks - from git t0006 check_relative 1728000 = 3 weeks (20 days)
baseline_relative '1 week ago' ''
baseline_relative '2 weeks ago' ''
baseline_relative '4 weeks ago' ''
baseline_relative '3 weeks ago' ''
baseline_relative '8 weeks ago' ''

# Months - from git t0006 check_relative 13000000 ≈ 5 months
baseline_relative '1 month ago' ''
baseline_relative '2 months ago' ''
baseline_relative '6 months ago' ''
baseline_relative '5 months ago' ''
baseline_relative '3 months ago' ''
baseline_relative '12 months ago' ''
baseline_relative '24 months ago' ''

# Intentional difference: constrain each calendar subtraction to its target month's last day.
# Git rolls February 31 into March 3; gix-date stays in February. These civil dates follow
# month lengths, leap years, and sequential ordering. See months_and_years_constrain_month_ends.
baseline_relative '1 month ago' 'GIX_DIFF:-259200' 1774958400 # 2026-02-28 at noon UTC
baseline_relative '1 month ago' 'GIX_DIFF:-86400' 1780228800 # 2026-04-30 at noon UTC
baseline_relative '6 months ago' 'GIX_DIFF:-259200' 1788177600 # 2026-02-28 at noon UTC
baseline_relative '1 month ago' 'GIX_DIFF:-172800' 1774872000 # 2026-02-28 at noon UTC
baseline_relative '1 month ago' 'GIX_DIFF:-86400' 1711800000 # 2024-02-29 at noon UTC
baseline_relative '1 month ago' 'GIX_DIFF:-172800' 1711886400 # 2024-02-29 at noon UTC
baseline_relative '1 year ago' 'GIX_DIFF:-86400' 1709208000 # 2023-02-28 at noon UTC
baseline_relative '12 months ago' 'GIX_DIFF:-86400' 1709208000 # 2023-02-28 at noon UTC
baseline_relative '2 months ago' '' 1774958400             # Control: target January has day 31, so no rollover
baseline_relative '1 year ago' '' 1774958400               # Control: target March has day 31, so no rollover
baseline_relative '1 day 1 month ago' 'GIX_DIFF:-259200' 1775044800 # 2026-02-28 at noon UTC
baseline_relative '1 month 1 day ago' '' 1775044800 # 2026-02-28 at noon UTC
baseline_relative '1 month 1 year ago' 'GIX_DIFF:-172800' 1711886400 # 2023-02-28 at noon UTC
baseline_relative '1 year 1 month ago' 'GIX_DIFF:-259200' 1711886400 # 2023-02-28 at noon UTC
baseline_relative '1 month 1 month ago' 'GIX_DIFF:-518400' 1774958400 # 2026-01-28 at noon UTC
baseline_relative '23 months ago' 'GIX_DIFF:-172800' 1769860800 # 2024-02-29 at noon UTC
baseline_relative '1 month 1 day 1 month ago' 'GIX_DIFF:-86400' 1780228800 # 2026-03-29 at noon UTC
baseline_relative '1 month 1 month 1 day ago' 'GIX_DIFF:-172800' 1780228800 # 2026-03-29 at noon UTC
baseline_relative '1 day 1 day ago' '' 1774958400           # Repeated fixed units accumulate instead of replacing each other

# Years - from git t0006 check_relative 630000000 = 20 years
baseline_relative '1 year ago' ''
baseline_relative '2 years ago' ''
baseline_relative '10 years ago' ''
baseline_relative '20 years ago' ''

# Note that we can't necessarily put 64bit dates here yet as `git` on the system might not yet support it.

# ============================================================================
# RELATIVE FORMS GIT ACCEPTS BEYOND "<n> <unit> ago"
# ============================================================================
# ascii-alnum is the for relevant partitions, so anything else separates them.
baseline_relative '1.hour.ago' ''
baseline_relative '1-hour-ago' ''
baseline_relative '2 -days' '' 1251660000
baseline_relative '2..days' '' 1251660000
baseline_relative '2//days' '' 1251660000
baseline_relative '2--days' '' 1251660000

# Unit names are case-insensitive
baseline_relative '2 HOURS ago' ''
baseline_relative '2 Days ago' ''
baseline_relative '2 Days ago 1 hour' ''
baseline_relative '2 Days 1 hour ago' ''
baseline_relative '2 Days and 1 hour ago' ''

# Counts can be spelled out, 1-10.
# Intentional extension: zero days is the identity operation, while Git rejects the word zero.
# See correctness::zero_and_zero_padded_counts_are_numbers.
baseline_relative 'zero days ago' 'GIX_VALUE:1000000000'
baseline_relative 'two days ago' ''
baseline_relative 'ten minutes ago' ''
baseline_relative 'eleven minutes ago' ''

# `last` is a count of one, and the trailing `ago` is not required.
baseline_relative 'last week' ''
baseline_relative 'last day ago' ''

# Digits may directly precede a unit without a separating space.
baseline_relative '2days' ''
baseline_relative '2DAYS' ''
baseline_relative '2days 3hours ago' ''
baseline_relative 'two days 3hours ago' ''
baseline_relative '0days' ''
baseline_relative '1month' ''
baseline_relative '1year' ''
# Git fails to recognize a unit followed immediately by another count, then treats
# the numbers as calendar fields. Honor both count/unit pairs instead: August 28 at 16:20,
# not Git's August 2 at 19:20. See correctness::adjacent_count_unit_pairs_are_complete_operations.
baseline_relative '2days3hours' 'GIX_DIFF:2246400' 1251660000
baseline_relative '2 days3 hours ago' 'GIX_DIFF:2246400' 1251660000

# Git keeps a number pending across filler words, then consumes it as a unit
# count or flushes it into day/month/year fields. Raw timestamps retain absolute precedence.
for date in '2 long days ago' 'one or two days ago' \
            '5 noon' '5 6 noon' '5 6 2008 noon' '5 6 08 noon' \
            '5 6 70 noon' '37 noon' two '2 nonsense' \
            '2 now' '2 yesterday' '12:34:56.08 days'; do
    baseline_relative "$date" '' 1251660000
done

# Intentional rejections: unused or unrepresentable numbers must not turn into now.
# See pending_numbers_fill_calendar_fields_and_units and correctness's rejection test.
for date in '2 hours 3' '12345 florx ago' '0 nonsense'; do
    baseline_relative "$date" 'GIX_REJECT' 1251660000
done
# Intentional year mapping: 38 -> 2038 and 00 -> 2000 after June 5; Git ignores both.
# See pending_numbers_fill_calendar_fields_and_units.
baseline_relative '5 6 38 noon' 'GIX_DIFF:915148800' 1251660000
baseline_relative '5 6 00 noon' 'GIX_DIFF:-283996800' 1251660000
# Intentional decimal/count semantics: 008 is eight days, and a clock's .3.days always
# subtracts three days. See correctness's count and suffix tests.
baseline_relative '008 days' 'GIX_DIFF:-691200' 1251660000
baseline_relative '12:34:56.008 days' 'GIX_DIFF:-691200' 1251660000
baseline_relative '5 6 2008 12:34:56.3.days.ago' 'GIX_DIFF:-259200' 1251660000

# Incomplete textual dates infer missing fields from the reference time. A later
# month selects the previous year, but a later day in the same month can stay future.
# Month names leave pending day numbers intact, including days attached without whitespace.
for date in 'July 5th' '5 July' July December 'December 31' 'August 31' \
            'January 5th noon pm' '6AM, June 7, 2009' 'June 7 6am 2009' \
            'Dec 6, 1992' 'Dec 02' 'June 2008' 'June 7 10' 'June 7 70' \
            'July 5 2 days ago' 'June July 5' 'now December' 'December now' \
            'Sept 5' 'Septe 5' 'July 5th noon' 'June 7 2009 12:34:56'; do
    baseline_relative "$date" '' 1251660000
done
for month in January February March April May June July August September October November December; do
    for ((length=3; length<=${#month}; length++)); do
        baseline_relative "${month:0:length} 5th" '' 1251660000
    done
done

# Intentional decimal dates: 0002 means December 2, not Git's December 30.
# The consistent year pivot keeps 38 -> 2038, 00 -> 2000 and padded 0008 -> 2008.
# See textual_month_fragments_infer_missing_calendar_fields.
baseline_relative 'Dec 0002' 'GIX_DIFF:-2419200' 1251660000
baseline_relative 'June 7 38' 'GIX_DIFF:915148800' 1251660000
baseline_relative 'June 7 00' 'GIX_DIFF:-283996800' 1251660000
baseline_relative 'June 7 0008' 'GIX_DIFF:-31536000' 1251660000
# Intentional rejection: an explicit nonexistent calendar day is not a request for March.
baseline_relative 'Feb 31' 'GIX_REJECT' 1251660000
baseline_relative 'Feb 29 2009' 'GIX_REJECT' 1251660000
# Intentional token handling: preserve the .3.days count, the explicit July 5 after a
# duration, and attached days in June7. Git drops each of these fields in these contexts.
# See textual_month_fragments_infer_missing_calendar_fields and correctness's clock test.
baseline_relative 'June 7 2008 12:34:56.3.days.ago' 'GIX_DIFF:-259200' 1251660000
baseline_relative '2 days July 5' 'GIX_DIFF:-1987200' 1251660000
baseline_relative 'JUNE7' 'GIX_DIFF:-5270400' 1251660000
baseline_relative '6AM, June7, 2009' 'GIX_DIFF:-5270400' 1251660000

# Counted weekdays select the nth strictly previous occurrence, keeping the clock.
# Use Git's t0006 reference Sunday so requesting Sunday must go back a full week.
# Git accepts case-insensitive prefixes of at least three letters, including plurals.
for weekday in Sunday Monday Tuesday Wednesday Thursday Friday Saturday \
               Sundays Mondays Tuesdays Wednesdays Thursdays Fridays Saturdays \
               sun mon tue tues wed wednes thu thur thurs fri sat; do
    baseline_relative "last $weekday" '' 1251660000
    baseline_relative "2 $weekday ago" '' 1251660000
done
baseline_relative 'last WEDNESDAY' '' 1251660000
baseline_relative 'two fridays' '' 1251660000
baseline_relative 'ten mondays ago' '' 1251660000
baseline_relative '2Fridays' '' 1251660000
baseline_relative 'last.tuesday' '' 1251660000
baseline_relative '0 tuesday' '' 1251660000
baseline_relative '0 tuesday ago' '' 1251660000

# Intentional difference: select weekdays from the updated calendar, including after zero
# units. July 30, 2009 is Thursday: last Thursday is July 23, not Git's cached-weekday July 27.
# See correctness::weekdays_use_the_updated_calendar_and_keep_the_local_clock.
baseline_relative '2 days last Tuesday' '' 1251660000
baseline_relative 'last Tuesday 2 days' '' 1251660000
baseline_relative 'last Tuesday last Friday' '' 1251660000
baseline_relative '1 month last Thursday' 'GIX_DIFF:-345600' 1251660000
baseline_relative 'last Thursday 1 month' '' 1251660000
baseline_relative '1 month 1 second last Thursday' '' 1251660000
baseline_relative '1 month 0 days last Thursday' 'GIX_DIFF:-345600' 1251660000
baseline_relative '1 month 1 month last Thursday' 'GIX_DIFF:172800' 1251660000
baseline_relative '1 month 0 months last Thursday' 'GIX_DIFF:-345600' 1251660000
baseline_relative '1 month 0 tuesday last Thursday' 'GIX_DIFF:-345600' 1251660000
baseline_relative '2 tuesdays 1 month last Thursday' 'GIX_DIFF:259200' 1251660000
baseline_relative '1 year last Thursday' 'GIX_DIFF:86400' 1251660000
baseline_relative '1 month last Sunday' 'GIX_DIFF:-604800' 1774958400 # February 28 -> Sunday February 22
baseline_relative '1 year last Thursday' 'GIX_DIFF:86400' 1709208000 # February 28, 2023 -> Thursday February 23

# Named clock times select the most recent named hour while the day is still
# unspecified. Applying a relative unit or `now` first fixes the day instead.
# Morning and evening references exercise both sides of noon and tea (17:00).
for date in noon midnight tea NOON Midnight TEA \
            'noon yesterday' 'yesterday noon' 'midnight yesterday' 'yesterday tea' \
            'last Friday at noon' 'tea last saturday' \
            'noon 1 day ago' '1 day ago noon' 'noon 0 days' \
            '1 month noon' 'noon 1 month' \
            'noon midnight tea' 'now noon' 'noon now'; do
    baseline_relative "$date" '' 1251660000
done
# Git 2.55 fixed the day selection of composite named clocks before noon.
# Keep cross-version morning cases here; tests/time/parse/relative.rs pins the
# changed cases to the corrected results from Git's date.c and t0006-date.sh.
for date in noon midnight tea NOON Midnight TEA \
            'midnight yesterday' 'noon 0 days' 'noon now'; do
    baseline_relative "$date" '' 1251616800
done
baseline_relative 'noon' '' 1251633600 # Exactly noon does not go back a day
baseline_relative 'tea' '' 1251651600  # Exactly tea time does not go back a day
baseline_relative 'midnight' '' 1251590400 # Midnight is the beginning of the current day

# Intentional difference: July 30's last Friday is July 24; Git uses August's cached weekday
# and returns Tuesday, July 28. See named_clocks_compose_with_durations_and_calendar_weekdays.
baseline_relative '1 month noon last Friday' 'GIX_DIFF:-345600' 1251660000
# A month unit overrides noon's previous-day hint, so morning August 30 -> July 30,
# not Git's July 29. See correctness::calendar_subtraction_stays_in_the_target_month.
baseline_relative 'noon 1 month' 'GIX_DIFF:86400' 1251616800

# Explicit clocks keep today's date even when the clock is later than now.
# A dot following a relative clock starts the next count, not fractional seconds.
for now in 1251616800 1251660000; do
    for date in '3:00' '15:00' '23:59:59' '1:2:3' '12:34:56.3.days.ago' \
                '03:04:05 yesterday' 'last Friday 12:34:56' '12:34:56 last Friday' \
                '12:34:56 1 month' \
                '15:00 06:30' '24:00' '24:00 1 day ago'; do
        baseline_relative "$date" '' "$now"
    done
done
baseline_relative '24:00' '' 1251750000 # Crossing the end of August
# The same updated-weekday rule applies after explicit clocks: July 24 at 12:34:56.
for now in 1251616800 1251660000; do
    baseline_relative '1 month 12:34:56 last Friday' 'GIX_DIFF:-345600' "$now"
done

# Intentional difference: .3.days is a count/unit pair even after a date-setting operation.
# Git discards that count as a fraction; subtract three days in every context instead.
# See correctness::clocks_are_not_erased_by_today_or_confused_with_fractional_counts.
for date in '0 days 12:34:56.3.days.ago' \
            'now 12:34:56.123' '12:34:56.3.days.ago 1 hour'; do
    baseline_relative "$date" '' 1251616800
done

for date in 'now 12:34:56.3.days.ago' 'yesterday 12:34:56.3.days.ago' \
            '1 month 12:34:56.3.days.ago'; do
    baseline_relative "$date" 'GIX_DIFF:-259200' 1251616800
done

# AM/PM can follow an hour or a full clock, or adjust the current clock by itself.
# Intentional rejection: explicit AM/PM hours must be 1..12. Git treats zero as an
# absent hour and reduces 24/25 modulo 12. See correctness's invalid-meridian test.
for now in 1251616800 1251660000; do
    for date in '6am yesterday' '6pm yesterday' 'yesterday 6PM' \
                '6:30pm' '06:30:45 PM' '12am' '12pm' '12:30am' '12:30pm' \
                am PM 'two pm' 'last am' \
                'last Friday 6pm' '6pm last Friday' \
                '1 month 6pm' '6am noon' '6pm am' '1 hour pm' '6pm 1 hour ago'; do
        baseline_relative "$date" '' "$now"
    done
done

for now in 1251616800 1251660000; do
    # Invalid overflowing clocks are rejected too; only ISO end-of-day 24:00 is valid.
    # See correctness's rejection test and numeric_clocks_check_component_and_timestamp_ranges.
    for date in 0am 0pm 24am 25pm '23:59:60' '24:59:60' '11:59:60 noon' '11:59:60 pm'; do
        baseline_relative "$date" 'GIX_REJECT' "$now"
    done
done

# Git 2.55 introduced `today` with a midnight default. Probe the behavior rather
# than the version to accommodate backports; pinned unit tests cover older hosts.
if GIT_TEST_DATE_NOW=1251660000 git -c section.key=today config --type=expiry-date section.key 2>/dev/null | grep -qx '1251590400'; then
    # Intentional difference: midnight stays on the requested day, ignoring Git's stale DST flag.
    TZ=America/New_York baseline_relative today 'GIX_DIFF:3600' 1710055800
    for now in 1251616800 1251660000; do
        for date in today TODAY 'noon today' 'today at noon' '6pm today' 'today 6pm' \
                    '6am today' 'today now' 'now today' 'today 1 day' \
                    '1 month today' 'today 1 month' 'today never' 'never noon'; do
            baseline_relative "$date" '' "$now"
        done
        # Intentional ordered semantics: today keeps the accumulated date and explicit clock;
        # it does not restore now's day or discard a .3.days count. See today's composition test.
        baseline_relative '1 day today' 'GIX_DIFF:-86400' "$now"
        baseline_relative 'now today 12:34:56.3.days.ago' 'GIX_DIFF:-259200' "$now"
        baseline_relative 'never today' 'GIX_DIFF:-2505600' "$now" # January 1, not January 30
        if [ "$now" = 1251616800 ]; then
            baseline_relative '07:20 today' 'GIX_DIFF:26400' "$now" # explicit 07:20, not midnight
            baseline_relative 'December 37 noon 12:34:56.3' '' "$now" # December 29 at 12:34:56
        else
            baseline_relative '07:20 today' '' "$now"
            # A fraction without a unit is discarded; Git guesses the .3 as December 3.
            # See fractional_clock_suffixes_do_not_become_calendar_days.
            baseline_relative 'December 37 noon 12:34:56.3' 'GIX_DIFF:2332800' "$now" # December 30
        fi
    done
fi


# The same intentional difference keeps explicit and named midnight at the requested local clock.
TZ=America/New_York baseline_relative '00:00' 'GIX_DIFF:3600' 1710055800
TZ=America/New_York baseline_relative midnight 'GIX_DIFF:3600' 1710055800

# Intentional civil weekday semantics: subtract calendar days to keep the requested
# clock across DST. Fixed-duration "1 day ago" still means exactly 86400 seconds.
# See weekday_selection_and_fixed_duration_days_have_distinct_dst_semantics.
TZ=America/New_York baseline_relative 'last sunday' 'GIX_DIFF:3600' 1710135000 # March 10 01:30 EST, not 00:30
TZ=America/New_York baseline_relative 'last saturday' 'GIX_DIFF:-3600' 1730615400 # November 2 01:30 EDT, not 02:30

# Intentional rejection: an unsupported future direction must not become a past date.
# See correctness::invalid_calendar_dates_meridians_and_future_directions_are_rejected.
baseline_relative '1 hour from now' 'GIX_REJECT' 1251660000
baseline_relative '1.hour.from.now' 'GIX_REJECT' 1251660000
baseline_relative '1 hour FROM NOW' 'GIX_REJECT' 1251660000
baseline 'Feb 14 2008 0:30 PM -0500' 'GIX_REJECT'
baseline 'Feb 14 2008 13:30 PM -0500' 'GIX_REJECT'

# Numeric short dates and annotated ISO dates remain unsupported formats, not correctness
# extensions. Reject them rather than guess unrelated calendar fields or lose an offset.
# See delimited_numeric_dates_are_not_guessed_as_separate_fields and annotated_iso_dates_do_not_reach_relative_guessing.
baseline_relative '02/14/2008' 'GIX_REJECT' 1251660000
baseline_relative '2008-02-14 20:30:45 -0500 (EST)' 'GIX_REJECT' 1251660000

# Intentional reference-free short-date behavior: choose deterministic midnight UTC,
# instead of acquiring the process's current clock. See parse::short.
baseline '1979-02-26' 'GIX_VALUE:288835200'
