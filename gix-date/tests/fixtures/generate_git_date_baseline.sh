#!/usr/bin/env bash
set -eu -o pipefail

git init

function baseline() {
    local test_date="$1" # first argument is the date to test
    local test_name="$2" # second argument is the format name for re-formatting

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

# Subsecond precision (Git ignores the subseconds)
baseline '2008-02-14 20:30:45.019-04:00' ''

# Various timezone formats from git t0006
baseline '2008-02-14 20:30:45 -0015' ''  # 15-minute offset
baseline '2008-02-14 20:30:45 -05' ''    # 2-digit hour offset
baseline '2008-02-14 20:30:45 -05:00' '' # colon-separated offset
baseline '2008-02-14 20:30:45 +00' ''    # 2-digit +00

# Timezone edge cases from git t0006
baseline '1970-01-01 00:00:00 +0000' ''
baseline '1970-01-01 01:00:00 +0100' ''
baseline '1970-01-02 00:00:00 +1100' ''

# DEFAULT format (Git's default): "Day Mon D HH:MM:SS YYYY +/-ZZZZ"
baseline 'Thu Sep 04 2022 10:45:06 -0400' '' # cannot round-trip, incorrect day-of-week
baseline 'Sun Sep 04 2022 10:45:06 -0400' 'GITOXIDE'
baseline 'Thu Aug 18 12:45:06 2022 +0800' ''
baseline 'Wed Jun 15 16:13:20 2016 +0200' ''  # from git t0006

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
# 'now' and 'today' don't seem to work.
baseline_relative 'yesterday' ''
 
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
