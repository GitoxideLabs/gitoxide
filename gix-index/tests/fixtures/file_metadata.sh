#!/usr/bin/env bash
set -eu -o pipefail

# Attempt to create files with the latest and earliest possible 64-bit dates/times for ext4.
# Although nanoseconds are stored in ext4, specifying fractions of a second does not seem to make
# this work better, and omitting them allows the commands that attempt to set these dates to
# succeed on more systems. While we use a portable format, if the system rejects a future date as
# out of range with an error (and touch does not automatically retry with an allowed date) then it
# can fail. In this case, we try again with a much more moderate date: the greatest value that can
# in practice always parse to fit within a 32-bit signed time_t. This is subject to change to
# support changed or new tests. It will also become less useful in the near future (after 2038).
TZ=UTC touch -d '2446-05-10 22:38:55' future || TZ=UTC touch -d '2038-01-19 03:14:07' future
TZ=UTC touch -d '1901-12-13 20:45:52' past
