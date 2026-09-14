#!/usr/bin/env bash
set -eu -o pipefail

# Keep local-timezone fallbacks reproducible on every machine.
export TZ=UTC

git init

function baseline() {
    local test_date="$1" # first argument is the date to test
    local test_name="$2" # format name, or GIX_DIFF:<seconds> for an intentional timestamp difference

    local status=0
    git -c section.key="$test_date" config --type=expiry-date section.key || status="$?"

    {
      echo "$test_date"
      echo "$test_name"
      echo "$status"
      if [ "$status" = 0 ]; then
        git -c section.key="$test_date" config --type=expiry-date section.key
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
      echo "$now"
    } >> baseline.git
}

# ============================================================================
# FIXED DATE FORMATS
# ============================================================================
# Tests from https://github.com/git/git/blob/master/t/t0006-date.sh

# Note: SHORT format (YYYY-MM-DD) is NOT included in baseline tests because
# Git fills in current time-of-day, making it non-reproducible for baseline comparison.
# SHORT format is tested separately in the unit tests.

# RFC2822 format: "Day, DD Mon YYYY HH:MM:SS +/-ZZZZ"
baseline 'Thu, 18 Aug 2022 12:45:06 +0800' 'RFC2822'
baseline 'Sat, 01 Jan 2000 00:00:00 +0000' 'RFC2822'
baseline 'Fri, 13 Feb 2009 23:31:30 +0000' 'RFC2822'  # Unix timestamp 1234567890
baseline 'Wed, 15 Jun 2016 16:13:20 +0200' 'RFC2822'  # from git t0006
baseline 'Thu, 7 Apr 2005 15:14:13 -0700' ''  # from git t0006

# Complete textual dates exercise Git's month-name matching independently of RFC 2822.
# Weekdays are ignored, and ordinal suffixes need not agree with the day number.
for date in 'February 14, 2008' 'February 14th, 2008' '14th February 2008' \
            'Monday, fEbRu 14st, 2008' 'Feb 29 2009' 'Feb 31 2008'; do
    baseline "$date 20:30:45 -0500" ''
done
baseline 'Feb 14 20:30:45 2008 -0500' ''
baseline 'February 14 2008 20:30 -05' ''
baseline 'February 14 2008 20:30:45 -05:00' ''
baseline 'February 14 2008 20:30:45 CET' ''
baseline 'February 14 2008 20:30:45 Z' ''
baseline 'February 14 2008 20:30:45 +2359' ''
baseline 'February 14 2008 20:30:45 -2359' ''
# Git's -1-minute sentinel loses this explicit offset; retain it like other numeric offsets.
baseline 'February 14 2008 20:30:45 -0001' 'GIX_DIFF:60'
baseline 'Feb 14 2008 24:00:00 +0000' ''
baseline 'Feb 14 2008 23:59:60 +0000' ''
for month in January February March April May June July August September October November December; do
    for ((length=3; length<=${#month}; length++)); do
        baseline "${month:0:length} 14th, 2008 20:30:45 +0000" ''
    done
done

# Standalone years in textual dates use match_digit(), not set_date()'s wider
# numeric-date pivot. Exactly two digits and an already parsed day are significant.
for year in 00 01 02 03 04 05 06 07 08 09 {70..99}; do
    baseline "February 14 $year 20:30:45 -0500" ''
    baseline "14th February $year 20:30:45 -0500" ''
    baseline "Feb 14 20:30:45 $year -0500" ''
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

# Short numeric years use Git's 00..37 / 71..99 mapping, not strptime's pivot.
# The date precedes the clock so these complete dates don't depend on "now".
for date in 02/03/08 02.03.08 14/02/08 02.14.08 \
            02/14/00 02/14/10 02/14/37 02/14/71 02/14/99 \
            99/02/14 99/14/02 71.02.14 71.14.02 08/02/14 08.02.14 \
            2/14/0 2/14/8 14.2.000 14.2.008 2/14/071 071/2/14; do
    baseline "$date 20:30:45 +0000" ''
done
baseline '02/14/08 20:30:45 -0500' ''
baseline '14.02.08 20:30:45 -0500' ''

# ISO8601_STRICT format: "YYYY-MM-DDTHH:MM:SS+ZZ:ZZ"
baseline '2022-08-17T21:43:13+08:00' 'ISO8601_STRICT'
baseline '2000-01-01T00:00:00+00:00' 'ISO8601_STRICT'
baseline '2009-02-13T23:31:30+00:00' 'ISO8601_STRICT'  # Unix timestamp 1234567890
baseline '2016-06-15T16:13:20+02:00' 'ISO8601_STRICT'  # from git t0006

# Z suffix for UTC timezone from git t0006
baseline '1970-01-01 00:00:00 Z' ''

# Compact ISO8601 formats from git t0006 (YYYYMMDDTHHMMSS)
# Note: Some compact formats like 20080214T2030 are not universally supported
# across all Git versions and platforms, so we only test the most common ones.
# baseline '20080214T20:30:45' '' # doesn't work on the macOS version yet. TODO: fix this - it worked on Linux
# baseline '20080214T203045' ''
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
# GIX_DIFF records the intentional difference from Git in seconds.
baseline '2022-01-01 12:00:00 +2359' 'ISO8601'
baseline '2022-01-01 12:00:00 -2359' 'ISO8601'
baseline '2022-01-01 12:00:00 +2400' 'GIX_DIFF:-86400'
baseline '2022-01-01 12:00:00 -2400' 'GIX_DIFF:86400'
baseline '2022-01-01T12:00:00+24:00' 'GIX_DIFF:-86400'
baseline '2022-01-01 12:00:00 +2559' 'GIX_DIFF:-93540'
# Git ignores offset seconds, while Jiff preserves the more precise instant.
baseline '2008-02-14T20:30:45+01:02:03' 'GIX_DIFF:-3'
baseline '2008-02-14T20:30:45-01:02:03' 'GIX_DIFF:3'

# Git's named timezone table also applies to ISO dates; RFC 2822 alone treats unfamiliar
# abbreviations as UTC. Cover every alias, including mixed case, in both input formats.
for zone in IDLW NT CAT HST HDT YST YDT PST PDT MST MDT CST CDT EST EDT AST ADT WAT \
            GMT UTC UT Z WET BST CET MET MEWT MEST CEST MESZ FWT FST EET EEST \
            WAST WADT CCT JST EAST EADT GST NZT NZST NZDT IDLE cet CeSt z; do
    baseline "2008-02-14 20:30:45 $zone" ''
    baseline "Thu, 14 Feb 2008 20:30:45 $zone" ''
done

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

# Note: Git does not support negative timestamps through --type=expiry-date
# gix-date does support them, but they can't be tested via the baseline.

# ============================================================================
# RELATIVE DATE FORMATS from git t0006
# ============================================================================
# These tests use GIT_TEST_DATE_NOW=1000000000 (Sun Sep 9 01:46:40 UTC 2001)

# Named
# Expiry-date treats `now` as a sentinel. `today` requires Git 2.55 (covered below).
baseline_relative 'yesterday' ''

# `never` resets all calendar/clock fields and clears a pending count. These cases
# avoid named clocks after a fixed date, whose behavior changed in Git 2.55.
for date in never NEVER '1 day never' '1 never' 1never 'noon never' 'never now' 'now never' \
            'never 12:34:56.3.days.ago'; do
    baseline_relative "$date" '' 1251660000
done
 
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

# Month-end normalization, leap years, and pair ordering. Each expected value comes directly from
# Git using the case-specific "now" in the third argument.
baseline_relative '1 month ago' '' 1774958400              # Mar 31 -> invalid Feb 31 -> Mar 3, not Feb 28
baseline_relative '1 month ago' '' 1780228800              # May 31 -> invalid Apr 31 -> May 1, not Apr 30
baseline_relative '6 months ago' '' 1788177600             # Multi-month subtraction still preserves day 31 into February
baseline_relative '1 month ago' '' 1774872000              # Mar 30 -> invalid Feb 30 -> Mar 2 in a common year
baseline_relative '1 month ago' '' 1711800000              # Mar 30 -> Mar 1 because leap February has 29 days
baseline_relative '1 month ago' '' 1711886400              # Mar 31 -> Mar 2 because leap February has 29 days
baseline_relative '1 year ago' '' 1709208000               # Leap day -> invalid Feb 29 in a common year -> Mar 1
baseline_relative '12 months ago' '' 1709208000             # Month arithmetic must match the preceding year case
baseline_relative '2 months ago' '' 1774958400             # Control: target January has day 31, so no rollover
baseline_relative '1 year ago' '' 1774958400               # Control: target March has day 31, so no rollover
baseline_relative '1 day 1 month ago' '' 1775044800         # Day first creates Mar 31, which rolls over through February
baseline_relative '1 month 1 day ago' '' 1775044800         # Month first lands on Mar 1, proving input-order evaluation
baseline_relative '1 month 1 year ago' '' 1711886400        # Month then year normalizes leap February before subtracting a year
baseline_relative '1 year 1 month ago' '' 1711886400        # Year then month reaches common February and differs from prior case
baseline_relative '1 month 1 month ago' '' 1774958400       # First rollover is normalized before the second month is subtracted
baseline_relative '23 months ago' '' 1769860800            # Multi-year month subtraction targets leap February
baseline_relative '1 month 1 day 1 month ago' '' 1780228800 # A day between month pairs changes the second pair's starting day
baseline_relative '1 month 1 month 1 day ago' '' 1780228800 # Moving that day last produces a different result
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

# Unit names are case-insensitive
baseline_relative '2 HOURS ago' ''
baseline_relative '2 Days ago' ''
baseline_relative '2 Days ago 1 hour' ''
baseline_relative '2 Days 1 hour ago' ''
baseline_relative '2 Days and 1 hour ago' ''

# Counts can be spelled out, 1-10.
baseline_relative 'zero days ago' ''
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
# the numbers as calendar fields. Honor both count/unit pairs instead.
baseline_relative '2days3hours' 'GIX_DIFF:2246400' 1251660000
baseline_relative '2 days3 hours ago' 'GIX_DIFF:2246400' 1251660000

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

# Weekdays compose with duration and calendar pairs in input order. Git retains
# the cached weekday after changing month/year fields until the next nonzero
# duration or calendar pair is applied; zero-count units do not normalize it.
baseline_relative '2 days last Tuesday' '' 1251660000
baseline_relative 'last Tuesday 2 days' '' 1251660000
baseline_relative 'last Tuesday last Friday' '' 1251660000
baseline_relative '1 month last Thursday' '' 1251660000
baseline_relative 'last Thursday 1 month' '' 1251660000
baseline_relative '1 month 1 second last Thursday' '' 1251660000
baseline_relative '1 month 0 days last Thursday' '' 1251660000
baseline_relative '1 month 1 month last Thursday' '' 1251660000
baseline_relative '1 month 0 months last Thursday' '' 1251660000
baseline_relative '1 month 0 tuesday last Thursday' '' 1251660000
baseline_relative '2 tuesdays 1 month last Thursday' '' 1251660000
baseline_relative '1 year last Thursday' '' 1251660000
baseline_relative '1 month last Sunday' '' 1774958400 # Month-end rollover before subtraction
baseline_relative '1 year last Thursday' '' 1709208000 # Leap-day rollover before subtraction

# Named clock times select the most recent named hour while the day is still
# unspecified. Applying a relative unit or `now` first fixes the day instead.
# Morning and evening references exercise both sides of noon and tea (17:00).
for date in noon midnight tea NOON Midnight TEA \
            'noon yesterday' 'yesterday noon' 'midnight yesterday' 'yesterday tea' \
            'last Friday at noon' 'tea last saturday' \
            'noon 1 day ago' '1 day ago noon' 'noon 0 days' \
            '1 month noon' 'noon 1 month' '1 month noon last Friday' \
            'noon midnight tea' 'now noon' 'noon now'; do
    baseline_relative "$date" '' 1251660000
done
# Git 2.55 fixed the day selection of composite named clocks before noon.
# Keep cross-version morning cases here; tests/time/parse/relative.rs pins the
# changed cases to the corrected results from Git's date.c and t0006-date.sh.
for date in noon midnight tea NOON Midnight TEA \
            'midnight yesterday' 'noon 0 days' 'noon 1 month' 'noon now'; do
    baseline_relative "$date" '' 1251616800
done
baseline_relative 'noon' '' 1251633600 # Exactly noon does not go back a day
baseline_relative 'tea' '' 1251651600  # Exactly tea time does not go back a day
baseline_relative 'midnight' '' 1251590400 # Midnight is the beginning of the current day

# Explicit clocks keep today's date even when the clock is later than now.
# A dot following a relative clock starts the next count, not fractional seconds.
for now in 1251616800 1251660000; do
    for date in '3:00' '15:00' '23:59:59' '1:2:3' '12:34:56.3.days.ago' \
                '03:04:05 yesterday' 'last Friday 12:34:56' '12:34:56 last Friday' \
                '1 month 12:34:56 last Friday' '12:34:56 1 month' \
                '15:00 06:30' '24:00' '23:59:60' '24:59:60' \
                '11:59:60 noon' '24:00 1 day ago'; do
        baseline_relative "$date" '' "$now"
    done
done
baseline_relative '24:00' '' 1251750000 # Crossing the end of August
# Once an operation establishes the date, Git instead discards the clock's
# dot-suffix as fractional seconds. A zero-count unit does not establish a date.
for date in 'now 12:34:56.3.days.ago' 'yesterday 12:34:56.3.days.ago' \
            '1 month 12:34:56.3.days.ago' '0 days 12:34:56.3.days.ago' \
            'now 12:34:56.123' '12:34:56.3.days.ago 1 hour'; do
    baseline_relative "$date" '' 1251616800
done

# AM/PM can follow an hour or a full clock, or adjust the current clock by itself.
# A zero hour acts like no hour: it retains the current minutes and seconds.
for now in 1251616800 1251660000; do
    for date in '6am yesterday' '6pm yesterday' 'yesterday 6PM' \
                '6:30pm' '06:30:45 PM' '12am' '12pm' '12:30am' '12:30pm' \
                '0am' '0pm' am PM 'two pm' 'last am' '24am' '25pm' \
                '11:59:60 pm' 'last Friday 6pm' '6pm last Friday' \
                '1 month 6pm' '6am noon' '6pm am' '1 hour pm' '6pm 1 hour ago'; do
        baseline_relative "$date" '' "$now"
    done
done

# Git 2.55 introduced `today` with a midnight default. Probe the behavior rather
# than the version to accommodate backports; pinned unit tests cover older hosts.
if GIT_TEST_DATE_NOW=1251660000 git -c section.key=today config --type=expiry-date section.key 2>/dev/null | grep -qx '1251590400'; then
    for now in 1251616800 1251660000; do
        for date in today TODAY 'noon today' 'today at noon' '6pm today' 'today 6pm' \
                    '6am today' 'today now' 'now today' '1 day today' 'today 1 day' \
                    '1 month today' 'today 1 month' 'now today 12:34:56.3.days.ago' '07:20 today' \
                    'today never' 'never today' 'never noon'; do
            baseline_relative "$date" '' "$now"
        done
    done
fi
