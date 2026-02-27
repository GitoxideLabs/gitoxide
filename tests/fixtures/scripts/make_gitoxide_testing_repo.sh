#!/bin/bash
# Reproduces the history and content of https://github.com/staehle/gitoxide-testing
# for use in journey tests without requiring network access.
#
# Note: This script is designed to be run via `jtt run-script` which sets up
# a controlled Git environment with fixed author/committer dates. The resulting
# commit hashes will differ from the original repository but the content and
# structure will be semantically equivalent.
set -eu -o pipefail

git init -q

# Configure the repository to allow partial clone filters when served via file://
git config uploadpack.allowFilter true
git config uploadpack.allowAnySHA1InWant true

# Initial commit (simulating GitHub's initial commit)
cat > LICENSE << 'EOF'
MIT License

Copyright (c) 2026 Jake Staehle

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

cat > README.md << 'EOF'
# gitoxide-testing
example repo to use in gitoxide functional tests
EOF

git add LICENSE README.md
git commit -q -m "Initial commit"

# Version 1 commit
cat > README.md << 'EOF'
# gitoxide-testing
Example repo to use in gitoxide functional tests

For use with journey tests in: https://github.com/GitoxideLabs/gitoxide

EOF

echo "This is version 1.0" > version.txt
git add README.md version.txt
git commit -q -m "version 1 commit"
git tag v1

# Version 2 commit
echo "This is version 2.0" > version.txt
touch new-file-in-v2
git add version.txt new-file-in-v2
git commit -q -m "version 2"
git tag v2

# Commit between versions - removes new-file-in-v2, adds another-new-file, updates version
echo "This is version 2.1" > version.txt
echo "This should exist starting in version 2" > another-new-file
git rm -q new-file-in-v2
git add version.txt another-new-file
git commit -q -m "a commit between versions"

# Adds a script (non-executable)
cat > a_script.sh << 'EOF'
#!/bin/sh

echo "This is a script!"
echo "It does things!"

exit 0
EOF
git add a_script.sh
git commit -q -m "adds a script (non-executable)"

# Version 3 commit
echo "This is version 3.0" > version.txt
git add version.txt
git commit -q -m "version 3"
git tag v3

# Make script executable
chmod +x a_script.sh
git add a_script.sh
git commit -q -m "just changes the file mode for a_script.sh from 644 to 755"

# Version 4 commit
echo "This is version 4.0" > version.txt

cat > CHANGELOG.md << 'EOF'
# Summary of commits in this repo and what to test for:

1. `v1`: Just adds `version.txt` and updates the README
2. `v2`: Immediately after v1, adds `new-file-in-v2`, and updates `version.txt` (all tags should have bumped versions of this)
3. `v3`: Has three commits after v2, adds `another-new-file` and a non-executable script `a_script.sh`
4. `v4`: Fixes the script to be executable
EOF

git add version.txt CHANGELOG.md
git commit -q -m "version 4"
git tag v4

# Version 5 commit
cat > README.md << 'EOF'
# gitoxide-testing
Example repo to use in gitoxide functional tests

For use with journey tests in: https://github.com/GitoxideLabs/gitoxide

# Summary of commits in this repo and what to test for:

1. `v1`: Just adds `version.txt` and updates the README
2. `v2`: Immediately after v1, adds `new-file-in-v2`, and updates `version.txt` (all tags should have bumped versions of this)
3. `v3`: Has three commits after v2, adds `another-new-file` and a non-executable script `a_script.sh`
4. `v4`: Fixes the script to be executable
5. `v5`: Removes `another-new-file` and moves the changelog to readme
EOF

# Note: Despite the README saying v5 removes another-new-file, the original repo still has it at v5
git rm -q CHANGELOG.md
git add README.md
git commit -q -m "version 5"
git tag v5
