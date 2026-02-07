#!/usr/bin/env bash
set -eu

# A simple fixture script that creates a basic file structure
echo "created by script" > script_file.txt
mkdir subdir
echo "nested" > subdir/nested.txt
