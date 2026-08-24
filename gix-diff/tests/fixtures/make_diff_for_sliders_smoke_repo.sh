#!/usr/bin/env bash
set -eu -o pipefail

mkdir assets

cat >assets/before.blob <<'EOF'
fn example() {
    before();
}
EOF

cat >assets/after.blob <<'EOF'
fn example() {
    after();
}
EOF

cat >before-after.myers.baseline <<'EOF'
diff --git a/before b/after
index 0000000..1111111 100644
--- a/before
+++ b/after
@@ -1,3 +1,3 @@ fn example() {
 fn example() {
-    before();
+    after();
 }
EOF

cp before-after.myers.baseline before-after.histogram.baseline
