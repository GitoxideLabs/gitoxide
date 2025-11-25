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
    } >> baseline.git
}

# Relative dates use a fixed "now" timestamp for reproducibility
# GIT_TEST_DATE_NOW sets Git's internal "now" to a specific Unix timestamp
# We use 1000000000 (Sun Sep 9 01:46:40 UTC 2001) as our reference point
function baseline_relative() {
    local test_date="$1" # first argument is the relative date to test
    local test_name="$2" # second argument is the format name (usually empty for relative dates)

    local status=0
    GIT_TEST_DATE_NOW=1000000000 git -c section.key="$test_date" config --type=expiry-date section.key || status="$?"

    {
      echo "$test_date"
      echo "$test_name"
      echo "$status"
      if [ "$status" = 0 ]; then
        GIT_TEST_DATE_NOW=1000000000 git -c section.key="$test_date" config --type=expiry-date section.key
      else
        echo '-1'
      fi
    } >> baseline.git
}

# ============================================================================
# FIXED DATE FORMATS
# ============================================================================
# Following https://git-scm.com/docs/git-log#Documentation/git-log.txt---dateltformatgt

# Note: SHORT format (YYYY-MM-DD) is NOT included in baseline tests because
# Git fills in current time-of-day, making it non-reproducible for baseline comparison.
# SHORT format is tested separately in the unit tests.

# RFC2822 format: "Day, DD Mon YYYY HH:MM:SS +/-ZZZZ"
baseline 'Thu, 18 Aug 2022 12:45:06 +0800' 'RFC2822'
baseline 'Sat, 01 Jan 2000 00:00:00 +0000' 'RFC2822'
baseline 'Fri, 13 Feb 2009 23:31:30 +0000' 'RFC2822'  # Unix timestamp 1234567890

# GIT_RFC2822 format: like RFC2822 but with non-padded day
baseline 'Thu, 1 Aug 2022 12:45:06 +0800' ''
baseline 'Sat, 1 Jan 2000 00:00:00 +0000' ''

# ISO8601 format: "YYYY-MM-DD HH:MM:SS +/-ZZZZ"
baseline '2022-08-17 22:04:58 +0200' 'ISO8601'
baseline '2000-01-01 00:00:00 +0000' 'ISO8601'
baseline '1970-01-01 00:00:00 +0000' 'ISO8601'

# ISO8601_STRICT format: "YYYY-MM-DDTHH:MM:SS+ZZ:ZZ"
baseline '2022-08-17T21:43:13+08:00' 'ISO8601_STRICT'
baseline '2000-01-01T00:00:00+00:00' 'ISO8601_STRICT'
baseline '2009-02-13T23:31:30+00:00' 'ISO8601_STRICT'  # Unix timestamp 1234567890

# DEFAULT format (Git's default): "Day Mon D HH:MM:SS YYYY +/-ZZZZ"
baseline 'Thu Sep 04 2022 10:45:06 -0400' '' # cannot round-trip, incorrect day-of-week
baseline 'Sun Sep 04 2022 10:45:06 -0400' 'GITOXIDE'
baseline 'Thu Aug 18 12:45:06 2022 +0800' ''

# UNIX timestamp format
# Note: Git only treats numbers >= 100000000 as UNIX timestamps.
# Smaller numbers are interpreted as date components.
baseline '1234567890' 'UNIX'
baseline '100000000' 'UNIX'
baseline '946684800' 'UNIX'  # 2000-01-01 00:00:00 UTC

# RAW format: "SECONDS +/-ZZZZ"
# Note: Git only treats timestamps >= 100000000 as raw format.
# Smaller numbers are interpreted as date components.
baseline '1660874655 +0800' 'RAW'
baseline '1660874655 -0800' 'RAW'
baseline '100000000 +0000' 'RAW'
baseline '1234567890 +0000' 'RAW'
baseline '946684800 +0000' 'RAW'

# Note: Git does not support negative timestamps through --type=expiry-date
# gix-date does support them, but they can't be tested via the baseline.

# ============================================================================
# RELATIVE DATE FORMATS
# ============================================================================
# These tests use GIT_TEST_DATE_NOW=1000000000 (Sun Sep 9 01:46:40 UTC 2001)

# Seconds
baseline_relative '1 second ago' ''
baseline_relative '2 seconds ago' ''
baseline_relative '30 seconds ago' ''

# Minutes
baseline_relative '1 minute ago' ''
baseline_relative '2 minutes ago' ''
baseline_relative '30 minutes ago' ''

# Hours
baseline_relative '1 hour ago' ''
baseline_relative '2 hours ago' ''
baseline_relative '12 hours ago' ''

# Days
baseline_relative '1 day ago' ''
baseline_relative '2 days ago' ''
baseline_relative '7 days ago' ''

# Weeks
baseline_relative '1 week ago' ''
baseline_relative '2 weeks ago' ''
baseline_relative '4 weeks ago' ''

# Months
baseline_relative '1 month ago' ''
baseline_relative '2 months ago' ''
baseline_relative '6 months ago' ''

# Years
baseline_relative '1 year ago' ''
baseline_relative '2 years ago' ''
baseline_relative '10 years ago' ''

# Note that we can't necessarily put 64bit dates here yet as `git` on the system might not yet support it.
