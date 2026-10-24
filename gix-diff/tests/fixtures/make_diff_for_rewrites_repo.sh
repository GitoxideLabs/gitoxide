#!/usr/bin/env bash
set -eu -o pipefail

function store_tree() {
  local revspec="${1:?the commit to get the tree for}"
  local current_commit
  current_commit=$(git rev-parse HEAD)
  git rev-parse "${current_commit}^{tree}" > ../"$revspec".tree
}

git init -q

cat <<EOF >>.git/config

[diff "binary-true"]
  binary = true
[diff "binary-false"]
  binary = false
[diff ""]
  command = "empty is ignored"
[diff]
  command = "this is also ignored as sub-section name is missing"
  algorithm = histogram
[diff "all-but-binary"]
  command = command
  textconv = textconv
  algorithm = histogram
  binary = auto
EOF

git checkout -b main
mkdir dir
touch a b dir/c d
git add .
git commit -q -m "c1 - initial"
store_tree "c1 - initial"

echo a >> a
echo b >> b
echo dir/c >> dir/c
echo d >> d
git commit -q -am "c2"
store_tree "c2"

# Intent-to-add entries do not participate in the committed tree. Replacing tracked
# entries with them therefore stages deletions, including with skip-worktree set.
git rm --cached a d
git add --intent-to-add a d
echo new >will-ita-add
git add --intent-to-add will-ita-add
cp .git/index .git/index-intent-to-add
git diff --cached --no-renames --name-status >.git/intent-to-add.git
git update-index --skip-worktree a d
cp .git/index .git/index-intent-to-add-skip
git diff --cached --no-renames --name-status >.git/intent-to-add-skip.git
git update-index --no-skip-worktree a d
cp a a-renamed
git add a-renamed
cp .git/index .git/index-intent-to-add-rename
git diff --cached --no-renames --name-status >.git/intent-to-add-rename.git
git diff --cached --find-renames=100% --name-status >.git/intent-to-add-rename-with-renames.git
git reset --hard HEAD
rm -f will-ita-add a-renamed

echo a1 >> a
echo dir/c1 >> dir/c
git commit -q -am "c3-modification"
store_tree "c3-modification"

git mv a dir/a-moved
git commit -m "r1-identity"
store_tree "r1-identity"

touch s1 s2 s3
git add s* && git commit -m "c4 - add identical files"
store_tree "c4 - add identical files"

git mv s1 z && git mv s2 b2 && git mv s3 b1
git commit -m "r2-ambiguous"
store_tree "r2-ambiguous"

git mv dir/c dir/c-moved
echo dir/cn >> dir/c-moved
echo n >> b
git commit -am "r3-simple" # modified rename and normal modification
store_tree "r3-simple"

touch lt1 lt2
ln -s lt1 link-1
echo lt1 > no-link # a file that has content like a link and a similar name
ln -s ../lt2 dir/link-2
git add . && git commit -m "c5 - add links"
store_tree "c5 - add links"

git mv link-1 renamed-link-1
git rm no-link
git rm dir/link-2 && ln -s lt1 z-link-2 && git add .
git commit -m "r4-symlinks" # symlinks are only tracked by identity
store_tree "r4-symlinks"

seq 10 > f1
seq 11 > f2
git add . && git commit -m "c6 - two files with more content"
store_tree "c6"

echo n >> f1
echo n >> f2
git mv f1 f1-renamed
git mv f2 f2-renamed

git commit -am "r5" # two renames
store_tree "r5"


seq 9 > base
git add base
git commit -m "c7" # base has to be added
store_tree "c7"

echo 10 >> base
cp base c1
cp base c2
cp base dir/c3
git add . && git commit -m "tc1-identity"
store_tree "tc1-identity"

echo 11 >> base
cp base c4 # can be located by identity
cp base c5 && echo 12 >> c5
cp base dir/c6 && echo 13 >> dir/c6
git add . && git commit -m "tc2-similarity"
store_tree "tc2-similarity"

cp base c6 # can be located by identity, but base needs --find-copies-harder
cp base c7 && echo 13 >> c7 # modified copy, similarity and find copies harder
seq 15 > newly-added
echo nn >> b
git add .
git commit -m "tc3-find-harder"
store_tree "tc3-find-harder"

rm -Rf ./*
# from 92de081dc9ab5660cb18fa750452345dd63550ea~1 of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 4d3da1c26ebc6e0edf617ff3cc8087eab493f61a	git-index/CHANGELOG.md
100644 blob 921fafb8ad45eb9932b607cf51b9640b7a74f324	git-index/Cargo.toml
100644 blob f239be693c0106cbd46e7889f718c7e41eb51fb0	git-index/README.md
100644 blob c72fd75ad38b88f06e559f64f75f88d836678e6c	git-index/src/access.rs
100644 blob f33602637b28b008334fcc09cdd9f7085684158a	git-index/src/decode/entries.rs
100644 blob 097807dd511b5e087f465cac1ba8bb5983890a78	git-index/src/decode/header.rs
100644 blob 82a6229681818d2c70764adf1e64814e37b9eb9e	git-index/src/decode/mod.rs
100644 blob be72ef7281e4cdad10916f06e2ec5418202ef634	git-index/src/entry.rs
100644 blob 5bf6699a611725ee47ed781acf75af387c68fd6c	git-index/src/extension/decode.rs
100644 blob c44d15b295ceabb6d36fef7f2f0e69e4d8a8f28c	git-index/src/extension/end_of_index_entry.rs
100644 blob 117fd21dfcdb497d0ab840c16bbe34d4a12c01c9	git-index/src/extension/fs_monitor.rs
100644 blob 9d0cab23d7123417d81bfbf34b5ef8add4432f20	git-index/src/extension/index_entry_offset_table.rs
100644 blob ec7c559e23ecd6492f23d43d39d9be7b70b9b539	git-index/src/extension/iter.rs
100644 blob 507f60db60d6b6b54942cdd6e48b13e8e568ada4	git-index/src/extension/link.rs
100644 blob cbf7c0e313f00b5dae2bea5566c639d29a03701f	git-index/src/extension/mod.rs
100644 blob d263a47c0e9036a89a20b004e55ef37f1f6a8a18	git-index/src/extension/resolve_undo.rs
100644 blob ec0fa25f9e933c9a1e875f44017b9df12800f95b	git-index/src/extension/tree.rs
100644 blob abf223eeb3b773c8c65a6aae300c137bad93d991	git-index/src/extension/untracked_cache.rs
100644 blob 700c7934b39e652c362f71dac5431185fe51abf3	git-index/src/file.rs
100644 blob 3e06aa76960b11e1c91158a06de0fb11fb94921c	git-index/src/file/init.rs
100644 blob 29b76829ecdef3a3994e3bec52c0035b1bdb3ae2	git-index/src/file/verify.rs
100644 blob c1b9699ddf6c46d38cb52279a2d67cb06e76107e	git-index/src/lib.rs
100644 blob 998a49e3127518b1c13a6e7e95a1bce1397896fa	git-index/src/verify.rs
100644 blob 50def2768ea1c917f637d0c0db2046426c05944d	git-index/src/write.rs
100644 blob 85f637142bd20a3513b7d2ebfd60f99b21418507	git-index/tests/fixtures/generated-archives/V2_empty.tar.xz
100644 blob 6f22567cdd33e5af04eaf6534b59c02e23fb91bd	git-index/tests/fixtures/generated-archives/v2.tar.xz
100644 blob 902ebe2e2e9012d5d0f838bcd1ceaa3cc31e18f0	git-index/tests/fixtures/generated-archives/v2_more_files.tar.xz
100644 blob 53973a1923b39bbae435a9ebd10d37eee4521281	git-index/tests/fixtures/generated-archives/v2_split_index.tar.xz
100644 blob c0f22de0620e424ef44393745b35368ad0164851	git-index/tests/fixtures/generated-archives/v4_more_files_IEOT.tar.xz
100644 blob ec8c20111ec1ebbad5654ff9c7c9ac1f3f138e52	git-index/tests/fixtures/loose_index/FSMN.git-index
100644 blob 5aa997c6f8cf6660cbe7108654e1d495301e890b	git-index/tests/fixtures/loose_index/REUC.git-index
100644 blob d77985692ab14f02c70c3472b58d0cfd60c145ca	git-index/tests/fixtures/loose_index/REUC.on-90d242d36e248acfae0033274b524bfa55a947fd.git-patch
100644 blob b36ae38974754043ee46f7ca3d6b55fb977e9125	git-index/tests/fixtures/loose_index/UNTR-with-oids.git-index
100644 blob 410996317bfadd1cec7229be65887b25342a6136	git-index/tests/fixtures/loose_index/UNTR-with-oids.on-90d242d36e248acfae0033274b524bfa55a947fd.git-patch
100644 blob e5d1c3cba599ef1e1347f43283fa3991382a66c3	git-index/tests/fixtures/loose_index/UNTR.git-index
100644 blob 0e253a5955f2ec02b4764002d008db426ba15e1b	git-index/tests/fixtures/loose_index/conflicting-file.git-index
100644 blob f03713b684711d4a5aedab0bafd6b254137c9e5d	git-index/tests/fixtures/loose_index/extended-flags.git-index
100644 blob f2c7d2ab253a69e8c693f58b72e84d1145bbdf90	git-index/tests/fixtures/loose_index/very-long-path.git-index
100644 blob 1eb48b72b8c15924c86e21da4bccd48d80e7e3fc	git-index/tests/fixtures/make_index/V2_empty.sh
100644 blob a61c9527d01147eaddb846aa3e266e9b00cfa3e3	git-index/tests/fixtures/make_index/v2.sh
100644 blob d4cafddc09766f8b75d10bc45a41ffb60a40054a	git-index/tests/fixtures/make_index/v2_more_files.sh
100644 blob 0b5c98e5ed863df3c001ec4634e2baabc4ba5d39	git-index/tests/fixtures/make_index/v2_split_index.sh
100644 blob 3b22aaa66924d970117f6148ccf5a5cdf0c52121	git-index/tests/fixtures/make_index/v4_more_files_IEOT.sh
100644 blob 936bcb7f42836f75d96b57045786a2d48b9a4c56	git-index/tests/index-multi-threaded.rs
100644 blob 4dcec528c1e96ccac6f98458619ba403bac1af97	git-index/tests/index-single-threaded.rs
100644 blob f7cdac660789401af818c9e9ba44aa18404bbed6	git-index/tests/index/file/mod.rs
100644 blob c8c055934ac028ebc5f9952529f9b252fec00870	git-index/tests/index/file/read.rs
100644 blob aa43428167b4598138cc1b468ffc904381042e82	git-index/tests/index/file/write.rs
100644 blob bb32d174af2d06c31aacc880ea340e0748d2aa1a	git-index/tests/index/mod.rs
EOF
git add . && git commit -m "r1-base"
store_tree "r1-base"

rm -Rf ./*
# from 92de081dc9ab5660cb18fa750452345dd63550ea of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 4d3da1c26ebc6e0edf617ff3cc8087eab493f61a	git-index/CHANGELOG.md
100644 blob 921fafb8ad45eb9932b607cf51b9640b7a74f324	git-index/Cargo.toml
100644 blob f239be693c0106cbd46e7889f718c7e41eb51fb0	git-index/README.md
100644 blob c72fd75ad38b88f06e559f64f75f88d836678e6c	git-index/src/access.rs
100644 blob f33602637b28b008334fcc09cdd9f7085684158a	git-index/src/decode/entries.rs
100644 blob 097807dd511b5e087f465cac1ba8bb5983890a78	git-index/src/decode/header.rs
100644 blob 82a6229681818d2c70764adf1e64814e37b9eb9e	git-index/src/decode/mod.rs
100644 blob be72ef7281e4cdad10916f06e2ec5418202ef634	git-index/src/entry.rs
100644 blob 5bf6699a611725ee47ed781acf75af387c68fd6c	git-index/src/extension/decode.rs
100644 blob c44d15b295ceabb6d36fef7f2f0e69e4d8a8f28c	git-index/src/extension/end_of_index_entry.rs
100644 blob 117fd21dfcdb497d0ab840c16bbe34d4a12c01c9	git-index/src/extension/fs_monitor.rs
100644 blob 9d0cab23d7123417d81bfbf34b5ef8add4432f20	git-index/src/extension/index_entry_offset_table.rs
100644 blob ec7c559e23ecd6492f23d43d39d9be7b70b9b539	git-index/src/extension/iter.rs
100644 blob 507f60db60d6b6b54942cdd6e48b13e8e568ada4	git-index/src/extension/link.rs
100644 blob cbf7c0e313f00b5dae2bea5566c639d29a03701f	git-index/src/extension/mod.rs
100644 blob d263a47c0e9036a89a20b004e55ef37f1f6a8a18	git-index/src/extension/resolve_undo.rs
100644 blob ec0fa25f9e933c9a1e875f44017b9df12800f95b	git-index/src/extension/tree.rs
100644 blob abf223eeb3b773c8c65a6aae300c137bad93d991	git-index/src/extension/untracked_cache.rs
100644 blob 3e06aa76960b11e1c91158a06de0fb11fb94921c	git-index/src/file/init.rs
100644 blob 700c7934b39e652c362f71dac5431185fe51abf3	git-index/src/file/mod.rs
100644 blob 29b76829ecdef3a3994e3bec52c0035b1bdb3ae2	git-index/src/file/verify.rs
100644 blob c1b9699ddf6c46d38cb52279a2d67cb06e76107e	git-index/src/lib.rs
100644 blob 998a49e3127518b1c13a6e7e95a1bce1397896fa	git-index/src/verify.rs
100644 blob 50def2768ea1c917f637d0c0db2046426c05944d	git-index/src/write.rs
100644 blob 85f637142bd20a3513b7d2ebfd60f99b21418507	git-index/tests/fixtures/generated-archives/V2_empty.tar.xz
100644 blob 6f22567cdd33e5af04eaf6534b59c02e23fb91bd	git-index/tests/fixtures/generated-archives/v2.tar.xz
100644 blob 902ebe2e2e9012d5d0f838bcd1ceaa3cc31e18f0	git-index/tests/fixtures/generated-archives/v2_more_files.tar.xz
100644 blob 53973a1923b39bbae435a9ebd10d37eee4521281	git-index/tests/fixtures/generated-archives/v2_split_index.tar.xz
100644 blob c0f22de0620e424ef44393745b35368ad0164851	git-index/tests/fixtures/generated-archives/v4_more_files_IEOT.tar.xz
100644 blob ec8c20111ec1ebbad5654ff9c7c9ac1f3f138e52	git-index/tests/fixtures/loose_index/FSMN.git-index
100644 blob 5aa997c6f8cf6660cbe7108654e1d495301e890b	git-index/tests/fixtures/loose_index/REUC.git-index
100644 blob d77985692ab14f02c70c3472b58d0cfd60c145ca	git-index/tests/fixtures/loose_index/REUC.on-90d242d36e248acfae0033274b524bfa55a947fd.git-patch
100644 blob b36ae38974754043ee46f7ca3d6b55fb977e9125	git-index/tests/fixtures/loose_index/UNTR-with-oids.git-index
100644 blob 410996317bfadd1cec7229be65887b25342a6136	git-index/tests/fixtures/loose_index/UNTR-with-oids.on-90d242d36e248acfae0033274b524bfa55a947fd.git-patch
100644 blob e5d1c3cba599ef1e1347f43283fa3991382a66c3	git-index/tests/fixtures/loose_index/UNTR.git-index
100644 blob 0e253a5955f2ec02b4764002d008db426ba15e1b	git-index/tests/fixtures/loose_index/conflicting-file.git-index
100644 blob f03713b684711d4a5aedab0bafd6b254137c9e5d	git-index/tests/fixtures/loose_index/extended-flags.git-index
100644 blob f2c7d2ab253a69e8c693f58b72e84d1145bbdf90	git-index/tests/fixtures/loose_index/very-long-path.git-index
100644 blob 1eb48b72b8c15924c86e21da4bccd48d80e7e3fc	git-index/tests/fixtures/make_index/V2_empty.sh
100644 blob a61c9527d01147eaddb846aa3e266e9b00cfa3e3	git-index/tests/fixtures/make_index/v2.sh
100644 blob d4cafddc09766f8b75d10bc45a41ffb60a40054a	git-index/tests/fixtures/make_index/v2_more_files.sh
100644 blob 0b5c98e5ed863df3c001ec4634e2baabc4ba5d39	git-index/tests/fixtures/make_index/v2_split_index.sh
100644 blob 3b22aaa66924d970117f6148ccf5a5cdf0c52121	git-index/tests/fixtures/make_index/v4_more_files_IEOT.sh
100644 blob 936bcb7f42836f75d96b57045786a2d48b9a4c56	git-index/tests/index-multi-threaded.rs
100644 blob 4dcec528c1e96ccac6f98458619ba403bac1af97	git-index/tests/index-single-threaded.rs
100644 blob 6f83a5d9535564439afe91547562083f94e23885	git-index/tests/index/file/access.rs
100644 blob d7d956584364125b9e6a07b02a20c45ff22ee396	git-index/tests/index/file/mod.rs
100644 blob c8c055934ac028ebc5f9952529f9b252fec00870	git-index/tests/index/file/read.rs
100644 blob aa43428167b4598138cc1b468ffc904381042e82	git-index/tests/index/file/write.rs
100644 blob bb32d174af2d06c31aacc880ea340e0748d2aa1a	git-index/tests/index/mod.rs
EOF
echo n >  git-index/tests/index/file/mod.rs
git add . && git commit -m "r1-change"
store_tree "r1-change"

rm -Rf ./*
# from d7ad650d3~1 of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 4ecbcfe3b44969cd567f3514d84248d734acd6a3	git-revision/tests/spec/parse/navigate/colon_symbol.rs
100644 blob 5bab15914e09a595f9debbad8fa6b86b020c49c8	git-revision/tests/spec/parse/navigate/mod.rs
100644 blob 5f30877b015559fbb8d5058fcb005c43c49d3049	git-revision/tests/spec/parse/navigate/tilde_symbol.rs
100644 blob 2f5046508dafd8c25fe245c1314ee0cbf3e84788	git-sec/CHANGELOG.md
100644 blob 032d318537b0b8903d386b799be700261fc4ab41	git-sec/Cargo.toml
100644 blob 7c3df38bdd204a62929492f6afb66d647737f612	git-sec/src/identity.rs
100644 blob 2f35d98cf7f363873ae93efb40df349f62b8e295	git-sec/src/lib.rs
100644 blob 5bd5f2c325f64dadf313cae344fe8973dc2a5952	git-sec/src/permission.rs
100644 blob 274c5b7807d807c04b9a702944d436ad9d3b2a35	git-sec/src/trust.rs
100644 blob da0fef3bf49d4fc0381479850eafef2ed107ce53	git-sec/tests/identity/mod.rs
100644 blob ce9c24122ab3303b9a8cd08fbc4de4fa1e65c6a9	git-sec/tests/sec.rs
100644 blob 979de34893206c26034b0ef370e40b9241f6f376	git-sequencer/CHANGELOG.md
100644 blob 65279fd869b49ba69157a3caf5d3c8ea17f557a7	git-sequencer/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-sequencer/src/lib.rs
100644 blob cc220fcac635129c05bdd6b23f57ef1f3ef1650a	git-submodule/CHANGELOG.md
100644 blob a198ab48930dd9d0e7fc39abb36131167314c6b9	git-submodule/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-submodule/src/lib.rs
100644 blob 4a0c6e1d835e155a0652a0b536af0c9aa0cf9741	git-tempfile/CHANGELOG.md
100644 blob 3aecbefa04a091629de00dae370b83c0fe7de676	git-tempfile/Cargo.toml
120000 blob 965b606f331b51d566b46025f9ff311a7aad0c12	git-tempfile/LICENSE-APACHE
120000 blob 76219eb72e8524f15c21ec93b9b2592da49b5460	git-tempfile/LICENSE-MIT
100644 blob 0de38724b9f3660894cec1ba177f84539238ffb9	git-tempfile/README.md
100644 blob aae9e27984c1ae02d906949b0a1e386d3a731073	git-tempfile/examples/delete-tempfiles-on-sigterm-interactive.rs
100644 blob d51692756498b7b683c3a69e80cec9f291be35f6	git-tempfile/examples/delete-tempfiles-on-sigterm.rs
100644 blob 55550a7188e2085d82bb28bab4add362a57e5c90	git-tempfile/examples/try-deadlock-on-cleanup.rs
100644 blob 804db1ed63448af8157aff2c80bf56e1a84b152c	git-tempfile/src/forksafe.rs
100644 blob 65efdb2b8892e437e9f67ad7b4e22ee2f50db19a	git-tempfile/src/fs/create_dir.rs
100644 blob bf26d819b88c598769f011b76277c7cb2fcbd275	git-tempfile/src/fs/mod.rs
100644 blob ac7b212fadd4df8bebae73bfd426f5234ab84510	git-tempfile/src/fs/remove_dir.rs
100644 blob df857107439c348e73c3e3e63575027cb024d15a	git-tempfile/src/handle.rs
100644 blob 7b17b144ad21ef64d142f99f6c68c7fb10b949cb	git-tempfile/src/handler.rs
100644 blob f33c6c8946766315c233e8e3af590a9d7c9be47e	git-tempfile/src/lib.rs
100644 blob 6332850b3aa1c3c57adad12e972d345da21a3800	git-tempfile/tests/all.rs
100644 blob d8f68872fcfa3ec20a1594c62f8a238124f14084	git-tempfile/tests/tempfile/fs/create_dir.rs
100644 blob 9c028bc7ce02dbe56c5845676e98436b036ce114	git-tempfile/tests/tempfile/fs/mod.rs
100644 blob f9323604cae7993b235719ee0972cf2002a87b11	git-tempfile/tests/tempfile/fs/remove_dir.rs
100644 blob 48475baed44a5fe6dc4b6a41c15bcc27853a828a	git-tempfile/tests/tempfile/handle.rs
100644 blob da91c56c66553d87d0dd4aa3a71baf6f9ccafea7	git-tempfile/tests/tempfile/mod.rs
100644 blob 1241f04e63422675b83851a1ada4e95d1b2cb76f	git-tix/CHANGELOG.md
100644 blob ea9674febfdef592bb8a3ed8195117e683043523	git-tix/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-tix/src/lib.rs
100644 blob 731b548f114d44015e5d81747d7ee06654209555	git-transport/CHANGELOG.md
100644 blob 212fcfed31b3468c8e0c5fc21a62768512a5417d	git-transport/Cargo.toml
100644 blob b66a8fafae9f781b9b38cc85e8d1b8e5602e7d5b	git-transport/src/client/async_io/bufread_ext.rs
100644 blob 924a5d4962761635c9df6ceda344a993003183a7	git-transport/src/client/async_io/connect.rs
100644 blob 6cb1a500e183bdec876221c6987e776c789b9675	git-transport/src/client/async_io/mod.rs
100644 blob 127b086ef3628a61ff2bcfbe12d4cdaf8544d61b	git-transport/src/client/async_io/request.rs
100644 blob ea73f5e0943f8dd56407bd7317b80bae2888ebb3	git-transport/src/client/async_io/traits.rs
100644 blob b994b7ab504da613a2259992b41de4aef3e3969f	git-transport/src/client/blocking_io/bufread_ext.rs
100644 blob 8ed6cd5c03c6a0a464f0f427afb271a0d224e65e	git-transport/src/client/blocking_io/connect.rs
100644 blob 1a20cd6fadeca100793dc2522768cd3ad32cb54b	git-transport/src/client/blocking_io/file.rs
100644 blob 3312aa6b9c0193a596b3db2fc909e2261f66e2dc	git-transport/src/client/blocking_io/http/curl/mod.rs
100644 blob e5635d28a0c37d8b27dab21815a63235b8d94f92	git-transport/src/client/blocking_io/http/curl/remote.rs
100644 blob e5ef33d6e8cfa4597adad8ac4bcbb5ee55299a2a	git-transport/src/client/blocking_io/http/mod.rs
100644 blob f429961e0da151664349a455807cdaf6e1727ebe	git-transport/src/client/blocking_io/http/redirect.rs
100644 blob 7c68b166ea2f9ec69eba3d41b7bce4365b048b40	git-transport/src/client/blocking_io/http/reqwest/mod.rs
100644 blob e24d86dc20b91e8cca00de823edd9262378ae0d2	git-transport/src/client/blocking_io/http/reqwest/remote.rs
100644 blob 5b163f892a1ab4e03381fbe6d80eae199e192f05	git-transport/src/client/blocking_io/http/traits.rs
100644 blob dfb3752af95f81d12212c3bf7ca6a47f7c67a3e5	git-transport/src/client/blocking_io/mod.rs
100644 blob a1104b95a1361db0ede4eeea387d9715ff46c8ba	git-transport/src/client/blocking_io/request.rs
100644 blob 97ba4c192f7be204162e13149aa7f52cd6228c64	git-transport/src/client/blocking_io/ssh/mod.rs
100644 blob 9d0be16de08b6aafb16596f507cb1d7a3990b7c1	git-transport/src/client/blocking_io/ssh/program_kind.rs
100644 blob 971d95992293f22a45128b77d9c7281d8ccff753	git-transport/src/client/blocking_io/ssh/tests.rs
100644 blob 7b2eaebbe198e8209ca4566ffb00c22dd79f95ed	git-transport/src/client/blocking_io/traits.rs
100644 blob 3c1cbec7e7d0fd9a8ed5ea113f2efd7b2b0b720e	git-transport/src/client/capabilities.rs
100644 blob 704a3458763dea46b16c6c12175422394e2ce533	git-transport/src/client/git/async_io.rs
100644 blob e49fe9315a3d6eec9a937890d188edfcd7199674	git-transport/src/client/git/blocking_io.rs
100644 blob 3684f00873cdf82e1691d14927a8f0130c3d4b0d	git-transport/src/client/git/mod.rs
100644 blob 0eeb6f145d6b139d6c081e4ec2751905b6e046ba	git-transport/src/client/mod.rs
100644 blob c60f733e584a4c201646d08114527ea00e6a3f91	git-transport/src/client/non_io_types.rs
100644 blob 9d036cb4bfbdbc2f2b07a40c171cf983f483deb8	git-transport/src/client/traits.rs
100644 blob f87864f08145adcc33c5a10c29f76360d2f0fee6	git-transport/src/lib.rs
100644 blob 19a22801885d18067c3dab22c0017fff935bb1b1	git-transport/tests/async-transport.rs
100644 blob af94be7e9f4b861f68961f5c9fe4398401c02222	git-transport/tests/blocking-transport-http.rs
100644 blob ab5365afddece55200f8a30aed22f20f7b95fe49	git-transport/tests/blocking-transport.rs
100644 blob 5bbf6291ac7000e27a5062d26994a1138153c33b	git-transport/tests/client/blocking_io/http/mock.rs
100644 blob cd3b97cac1c93cc79129ce3cfb8b1657cfd210e5	git-transport/tests/client/blocking_io/http/mod.rs
100644 blob 57861695c5989af3de46ffa0c56d34d09b391ba0	git-transport/tests/client/blocking_io/mod.rs
100644 blob 5d300d1218ddf868ce51f3c6d8ecc05313e8edf4	git-transport/tests/client/capabilities.rs
100644 blob 4abab468654e94c2f4ce95c7d480ac03bd722548	git-transport/tests/client/git.rs
100644 blob 15ff276eadfd7a3b87de12a45ef4a457bb609a72	git-transport/tests/client/mod.rs
100644 blob f08593978612021a361886b8879cbcb3f539c9d2	git-transport/tests/fixtures/http-401.response
100644 blob 1d0780739288b899dd5f47b4db7fb746923fecb9	git-transport/tests/fixtures/http-404.response
100644 blob 0d38823e2a52050d61e6554a681e8e3b6b4b8e25	git-transport/tests/fixtures/http-500.response
100644 blob 9aeadfcc5622bc6c5b809381e8766ab920c97865	git-transport/tests/fixtures/v1/clone.request
100644 blob 014bd57aa08e26321886ee8b40edab4aaa6b491c	git-transport/tests/fixtures/v1/clone.response
100644 blob 07375e3ca2e41e77ca471f9e20747073bc32d9e4	git-transport/tests/fixtures/v1/http-clone.response
100644 blob 4b1d110410e779fc0c9b9d91fe22d437827c5a8f	git-transport/tests/fixtures/v1/http-handshake.response
100644 blob 20808368a49141542388c878e12e8bfdd3776b82	git-transport/tests/fixtures/v1/push.request
100644 blob 3a156d3f84e8bf501095ed4769287e4c87f25b55	git-transport/tests/fixtures/v1/push.response
100644 blob c68f30ff4a3fbff6f0876a9d45484dd1db7a476b	git-transport/tests/fixtures/v2/clone.request
100644 blob e536d55553f47218ba5d4380772b4d8a4e3d53cc	git-transport/tests/fixtures/v2/clone.response
100644 blob e5f36bd7cc9c0eb49fd076d7352dfd376bb8f4d2	git-transport/tests/fixtures/v2/http-fetch.response
100644 blob f22e506aba79247115f327925ac1eb4ddd5fc636	git-transport/tests/fixtures/v2/http-handshake-lowercase-headers.response
100644 blob 95fec47eaad1ae1dedc79d7998ea23eaf7c4fd4e	git-transport/tests/fixtures/v2/http-handshake-service-announced.response
100644 blob ebb41785d75decc4672b4b06857a42cafef5e2ad	git-transport/tests/fixtures/v2/http-handshake.response
100644 blob a11151fdc01f0289bcab9d29ac85e2199f2c57cb	git-transport/tests/fixtures/v2/http-lsrefs.response
100644 blob 745e1ea988fbde9e4697ad9f0907dc6272d95d4a	git-transport/tests/fixtures/v2/http-no-newlines-handshake.response
100644 blob e9b9992cd46fe7e9c6e521b2e899bacccf5e6ed0	git-traverse/CHANGELOG.md
100644 blob a2458e7754fba49133ce2f080c88750ee101d072	git-traverse/Cargo.toml
100644 blob de5a67e23048d15c2869ee4f7a9e7b62f12a7807	git-traverse/src/commit.rs
100644 blob 3cf6d2b3af7aa85b4bb24871ad86f473871d273c	git-traverse/src/lib.rs
100644 blob b8891b7e37151749b93e9d291d60d3af3c5e03df	git-traverse/src/tree/breadthfirst.rs
100644 blob ae5354198d92a8c454055c725e43af35827d09b8	git-traverse/src/tree/mod.rs
100644 blob e8e8705f8e9711786d4dbc465ff7820de0e7cc2f	git-traverse/src/tree/recorder.rs
100644 blob 1bee4f196e838c49996c87a503f0cd6898ec0c9c	git-traverse/tests/Cargo.toml
100644 blob 5bf10916b172794a750ffab0249bd9033cc5a10a	git-traverse/tests/commit/mod.rs
100644 blob 3b3aa46d92867e38deff18a997b6d74bdebd5489	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_commits.tar.xz
100644 blob 36ed2e1aaad3dab8335a3ec94ffc0321130605eb	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_commits_with_dates.tar.xz
100644 blob 47eb88b0ff8a58a708d4373093837409910ef758	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_trees.tar.xz
100755 blob fe7d265f0d35eae336a46101ef250e7b695b5dce	git-traverse/tests/fixtures/make_traversal_repo_for_commits.sh
100755 blob bb557a416c02d80d008fec9152752ecc2fb49d18	git-traverse/tests/fixtures/make_traversal_repo_for_commits_with_dates.sh
100755 blob 263ecf27f4779cb82fff799886d6d49c5f4f57bd	git-traverse/tests/fixtures/make_traversal_repo_for_trees.sh
100644 blob 5a61dde6515ed21f08d4c205b6327904625032bf	git-traverse/tests/traverse.rs
100644 blob 8dfee12de23c3ed2981b71d38a2221bb8b53acf1	git-traverse/tests/tree/mod.rs
100644 blob 8beaedfa8297379abc5653c30443e6b0036d6467	git-tui/Cargo.toml
100644 blob 86aec11b1489b268ba26bc74bb033b3994b16b14	git-tui/src/main.rs
100644 blob 5412dc380c46e86b99e3383a74173e886c877851	git-url/CHANGELOG.md
100644 blob e775ca4c301440f1aa1d08fe63142f3a108f4d9f	git-url/Cargo.toml
100644 blob 393ce9116c66098c4c86498c6bef14ae001a0c77	git-url/src/expand_path.rs
100644 blob d3c5780888799ed5fb321f709ef933f9bf1b759d	git-url/src/impls.rs
100644 blob 9e3de2b79aa937dc2429a03727de95a239fa576c	git-url/src/lib.rs
100644 blob 0878d954b1828cf76d1c74e3cea793f78bd1e3fe	git-url/src/parse.rs
100644 blob 5f491596af2d941b80c3f19900901a7d018f16fd	git-url/src/scheme.rs
100644 blob 58e83d9fb13bd87eee342279becad7d91e1c2f70	git-url/tests/access/mod.rs
100644 blob a93db8f2f0e6060615f750a5e2c3d161a7e37a75	git-url/tests/expand_path/mod.rs
100644 blob 082e0ef07a1e1c22e29e394723c64a2177a96ac1	git-url/tests/parse/file.rs
100644 blob b6190d9c49b03296aa5a7bb79c42be7245b41f31	git-url/tests/parse/invalid.rs
100644 blob 19f09c2ab9b9639de9df8e1749c1182b41fa6599	git-url/tests/parse/mod.rs
100644 blob 06001013994885422734a988447191a6682f37f6	git-url/tests/parse/ssh.rs
100644 blob 9efd0163648a9f5b994712c1712b5f6b101bff31	git-url/tests/url.rs
100644 blob 89cde36ac39547728f2a59cad8cd1cb06dcd2223	git-validate/CHANGELOG.md
100644 blob 9e7ea8dedb5e5cc72281b7b77cc0b4e7ec1c7638	git-validate/Cargo.toml
100644 blob fd603aeb8b8fadf9d651725f2e16aa86bd90b7e3	git-validate/src/lib.rs
100644 blob eb1f25a812d67915889536a678a9979696f37af1	git-validate/src/reference.rs
100644 blob 91ceec18549240d3cecf3a5482c72582bca3a435	git-validate/src/tag.rs
100644 blob 1b0bdf8378ba1127e3489ac4c6b0b70e5040e1ee	git-validate/tests/all.rs
100644 blob a75d2a0768a9602da34c548b1538c13bc9bdc467	git-validate/tests/validate/mod.rs
100644 blob 620572b12ea6ec39db393008604fd3b6886ef727	git-validate/tests/validate/reference.rs
100644 blob 5523e2f2181cc660104b9d8cb142781f05b42291	git-validate/tests/validate/tagname.rs
100644 blob 2ff545f32f928c5d337fdd390916e6703514f6e9	git-worktree/CHANGELOG.md
100644 blob 909c43ab0aa8369b8b47f8c7887ae71e5cbc899a	git-worktree/Cargo.toml
100644 blob b75ceb753dfb0bf8d4e45e97e62ae064197b4262	git-worktree/src/fs/cache/mod.rs
100644 blob 448464345cebca4e65b42ba7908f7d37fed9b7c2	git-worktree/src/fs/cache/platform.rs
100644 blob 30c8fa04eb24e36025840da379c226e9e840473f	git-worktree/src/fs/cache/state.rs
100644 blob 64daab9cea33b85d43d423f10b8fd3bde4fd219a	git-worktree/src/fs/capabilities.rs
100644 blob 2f0ac0f8f59be33ee8c8db406d54f9919be01874	git-worktree/src/fs/mod.rs
100644 blob 734a4988b40c873bdf63a7dd257617410f9d5d5b	git-worktree/src/fs/stack.rs
100644 blob 6f094777896d8cea62a9ca84c4fd463f3c0f57ad	git-worktree/src/index/checkout.rs
100644 blob 924f127a4b20b345d58cc09b60245263176c965a	git-worktree/src/index/entry.rs
100644 blob def2655345fa7e614aef38241d070272d11d7eda	git-worktree/src/index/mod.rs
100644 blob 9a67e0289ee850c72494771581402861ceee51fa	git-worktree/src/lib.rs
100644 blob a297e73cd876c9e91f9f7c634c95b6b4245e72a2	git-worktree/src/os.rs
100644 blob 16fa0a5d7c6e5b8f3b1fa9c0a91b615be94b29f7	git-worktree/tests/fixtures/generated-archives/make_dangerous_symlink.tar.xz
100644 blob 2eb265bd062868382f00c57576aff23d19682c9b	git-worktree/tests/fixtures/generated-archives/make_ignore_and_attributes_setup.tar.xz
100644 blob 970248cf7c4ba6379ae823d1cf7e644569de445b	git-worktree/tests/fixtures/generated-archives/make_ignorecase_collisions.tar.xz
100644 blob b9fd4d7e33e9b16b60563c65ca6fd16700cf26ef	git-worktree/tests/fixtures/generated-archives/make_mixed_without_submodules.tar.xz
100644 blob 56edd71ff4a6ba564f271dfea79e693afd3fdccd	git-worktree/tests/fixtures/generated-archives/make_special_exclude_case.tar.xz
100644 blob 31437285a37a396243633014ce41774fd54f8e5e	git-worktree/tests/fixtures/make_dangerous_symlink.sh
100644 blob e176e6c8140a4c838c06830403b7ab951d54022d	git-worktree/tests/fixtures/make_ignore_and_attributes_setup.sh
100644 blob c06d3926429b14ee8743a41372e3eb3c9fe386ef	git-worktree/tests/fixtures/make_ignore_setup.sh
100644 blob 4a20f2080c575a15118e728f5ba3c86b339276fd	git-worktree/tests/fixtures/make_ignorecase_collisions.sh
100755 blob 618c60ca54bf2dc1fbef1500454adfe481908698	git-worktree/tests/fixtures/make_mixed_without_submodules.sh
100644 blob 17a279eda9d6836d30d7d3fbd3cb64cf55876305	git-worktree/tests/fixtures/make_special_exclude_case.sh
100644 blob cdae7eb15e4ca0b89e2a80b3ebfd0bf1c50de126	git-worktree/tests/worktree-multi-threaded.rs
100644 blob 39b8dea85bec01b546bea47ef5a7ea33d8ef00eb	git-worktree/tests/worktree-single-threaded.rs
100644 blob 434e6efa34c2537d55b1881ae707a0e938a32ade	git-worktree/tests/worktree/fs/cache/create_directory.rs
100644 blob 896bca3cea0f9326cf15c3f3c2647d15c04477cc	git-worktree/tests/worktree/fs/cache/ignore_and_attributes.rs
100644 blob ed8b4a222657b64f7037ced269163d69c233d923	git-worktree/tests/worktree/fs/cache/mod.rs
100644 blob 65064b1a507d5443fe66163069aea598a11063b8	git-worktree/tests/worktree/fs/mod.rs
100644 blob 41e3406ceadaea3ce05e65717b1abd2844bbf6d8	git-worktree/tests/worktree/fs/stack/mod.rs
100644 blob 829b0f59698439edb20aeaf6d21030e436445208	git-worktree/tests/worktree/index/checkout.rs
100644 blob 24370dce48a9b09f6c376c518c2fcd11a737c804	git-worktree/tests/worktree/index/mod.rs
100644 blob bc68cc4be7b6ad76450839eeac9d55327985b759	git-worktree/tests/worktree/mod.rs
100644 blob 7197952e50aa77dead829d931c5803bc29367a06	gitoxide-core/CHANGELOG.md
100644 blob ce87494673145fcef3dbf5509269b50fec71429e	gitoxide-core/Cargo.toml
100644 blob a8118c56ae71a93cf7f09c7883d4ff16c7d51cef	gitoxide-core/src/commitgraph/mod.rs
100644 blob e79e502d43fdc4177e34588d912c49a7a9991d3b	gitoxide-core/src/commitgraph/verify.rs
100644 blob f33c28b5b8a9e2958204d26530a74ac3a90f8c1e	gitoxide-core/src/hours/core.rs
100644 blob 38258de0f8212f8a3a09cc019246cabe87c3e812	gitoxide-core/src/hours/mod.rs
100644 blob 5738efd8ac3c5ba7e7055773ed6d195e5a2eee00	gitoxide-core/src/hours/util.rs
100644 blob 0bccdf127bfd807e6bffbef6e815561519b67c78	gitoxide-core/src/index/checkout.rs
100644 blob e6ca66aee51c3f06fc333a0920bff7b771151b1f	gitoxide-core/src/index/entries.rs
100644 blob d1e543608d1c8fda88d91d3b43095fd2e084e73f	gitoxide-core/src/index/information.rs
100644 blob c8556e6e01f6197036e16640cce6e975d04dff01	gitoxide-core/src/index/mod.rs
100644 blob 0d8dac5a892c89fc68ece3bce9e0ca504dce1c82	gitoxide-core/src/lib.rs
100644 blob 85c1e51c96a7df2d1121027ad1af862ed995e148	gitoxide-core/src/mailmap.rs
100644 blob 86455dd671297fc59d057df81762205990476c5f	gitoxide-core/src/net.rs
100644 blob c58aca146c94ea9b47fc8ae562a220f26c59706d	gitoxide-core/src/organize.rs
100644 blob 098cca5df58e13837fc3f4240906629d7b2682c2	gitoxide-core/src/pack/create.rs
100644 blob ae9aa972a45fd08d73e025ee1956bc67f2ae56a9	gitoxide-core/src/pack/explode.rs
100644 blob aec6343380d544bb9e1ee541e9453c6ea13d9b8f	gitoxide-core/src/pack/index.rs
100644 blob 0805418356cf47aaa5a477ebdad2eebb00b4f2db	gitoxide-core/src/pack/mod.rs
100644 blob eb847870e9d57acdceaa205370e5d517df242ace	gitoxide-core/src/pack/multi_index.rs
100644 blob fa512bba534f6aa5a17fd4516d2cfc6afcc762a8	gitoxide-core/src/pack/receive.rs
100644 blob c56afcefc389ae326d0d7b78186c7d20755ffaff	gitoxide-core/src/pack/verify.rs
100644 blob 59d521b3f192bbb0e0ac5d3e1cd57ecb72b72c46	gitoxide-core/src/repository/clone.rs
100644 blob d1f4c400e9f7a321af93a4b972d6d478776d4126	gitoxide-core/src/repository/commit.rs
100644 blob efeb2c53ef8be868d69a7f8e07dd719bb6858177	gitoxide-core/src/repository/config.rs
100644 blob b9b93b9d057da76abc3ba0cc5744994025f631c7	gitoxide-core/src/repository/credential.rs
100644 blob c6f620b9ff9149b0a3ed18cccc7804849863a28a	gitoxide-core/src/repository/exclude.rs
100644 blob 1b980965487d2b0edcfaad60a11709f881fccb58	gitoxide-core/src/repository/fetch.rs
100644 blob d14fca2227eeb09ff610fef995a78e02de43dea4	gitoxide-core/src/repository/index.rs
100644 blob 14531df7a6c757d7fafee05e29ab3dbf483e79c7	gitoxide-core/src/repository/mailmap.rs
100644 blob 48e3ad87d13a5206b30633bbafb8c24e0bfd822a	gitoxide-core/src/repository/mod.rs
100644 blob e41d95439b438012a42d37adf39c193c0b3a2a89	gitoxide-core/src/repository/odb.rs
100644 blob 78c8acc0b86c19e04fa3a0a7a4cdc8c68fbb83c1	gitoxide-core/src/repository/remote.rs
100644 blob 05c8b2833525c4da2ac725971b9cba35cbd8d5ab	gitoxide-core/src/repository/revision/explain.rs
100644 blob e87590f5d9d8bc0167f182ec56604c50675ec63e	gitoxide-core/src/repository/revision/list.rs
100644 blob 5e5dda98af523783a2811cfc3c0e783948a37ca7	gitoxide-core/src/repository/revision/mod.rs
100644 blob 727b56cf4b77623ea226d2e768297d2007343d2d	gitoxide-core/src/repository/revision/previous_branches.rs
100644 blob a66d6b8a914589f72a752bca7d61b86e4f11f673	gitoxide-core/src/repository/revision/resolve.rs
100644 blob cbe9b913352d3d4162e2a09232b32ea6e79e237c	gitoxide-core/src/repository/tree.rs
100644 blob 3b8c2579250e628dc1b141b876f71a689ae48952	gitoxide-core/src/repository/verify.rs
100644 blob fc8f25ace4a901bf771d31dac604cdf866db3261	gix-config-value/CHANGELOG.md
100644 blob e4f4aa4a2126c83404a2eacfefb9cdfc94905aa7	gix-config-value/Cargo.toml
100644 blob 908e11a300a637d5e15ec3776dda2709d981c5e1	gix-config-value/src/boolean.rs
100644 blob 558e81a769381d971d7b1f5b42df693db5b2523b	gix-config-value/src/color.rs
100644 blob 7214b76e4c82ab1b4402a9a8721bf948cc828196	gix-config-value/src/integer.rs
100644 blob 5b4f3fa76c34101c422de7a2dd61e88b0ff0279e	gix-config-value/src/lib.rs
100644 blob 99ee5cf2efef1d3781393a26f59b3bfa6c89db04	gix-config-value/src/path.rs
100644 blob 239679c703d6cb5131ad664312914f8a50312581	gix-config-value/src/types.rs
100644 blob 8fe8b43f0d43b5c675574e6d763cdeb171008220	gix-config-value/tests/value/boolean.rs
100644 blob 1db7fe2e03124687ec6d754e81714d429bd14a07	gix-config-value/tests/value/color.rs
100644 blob 9de2ab1ca2a8bc7d2698794b125f2c49cd722573	gix-config-value/tests/value/integer.rs
100644 blob 13a33c9d8f6b0f7d4b0929e0863000c8871b09d4	gix-config-value/tests/value/main.rs
100644 blob 10970296dbfd77368f8428a5ca3622962f747103	gix-config-value/tests/value/path.rs
100644 blob 76f58d7a187155013b02aa11a0903de46e8af0fe	gix-path/CHANGELOG.md
100644 blob 6a17963b4a7fecdfdd7d38ea0fff8ab4d3030838	gix-path/Cargo.toml
100644 blob 6a949529f588978cb5feb35ea7034c703565e062	gix-path/src/convert.rs
100644 blob 70a9bc53f0704bc3e4a14263501c71355785cfaf	gix-path/src/lib.rs
100644 blob 807cb74ccf62957bb4645e7d5d0b8020e3764696	gix-path/src/realpath.rs
100644 blob ff878bde141a040d3c65b9672feb612a5bb54b9a	gix-path/src/spec.rs
100644 blob 7920910d725bcdbea486e340c490c76a115dfd8c	gix-path/src/util.rs
100644 blob bf67c26241b3a56c1a1fa32f5d64c2057712bd83	gix-path/tests/convert/mod.rs
100644 blob 65a8885eb5722781d01941767d4a6667b4b63c3c	gix-path/tests/convert/normalize.rs
100644 blob 95ebafc5e32d846cf4c980b6a6fbd8ac84809de9	gix-path/tests/path.rs
100644 blob df5a59090d7fdcb1f91d1ef2543b5dcd84266e07	gix-path/tests/realpath/mod.rs
100644 blob 7d78de7e70bfa44cb30347204b24f228a4b5ac27	gix-path/tests/util/mod.rs
100644 blob e8a55463ab7ec43354595b236df56a3e30235691	gix/CHANGELOG.md
100644 blob 2e4f5ab099d6d05403c46714e561fd1a95c4cdc8	gix/Cargo.toml
100644 blob fb5cbf0c14bc2216e794f459a1c173061fe4980a	gix/examples/clone.rs
100644 blob aad2db759084cc51206967c56ed8e4f4417ede97	gix/examples/init-repo-and-commit.rs
100644 blob e75a50beea5cb93ba49d4c95db84837db91ac953	gix/examples/interrupt-handler-allows-graceful-shutdown.rs
100644 blob fcd02a2a38e76659682a57ed3f06349aa828b921	gix/examples/reversible-interrupt-handlers.rs
100644 blob 8c41a40fcefcff7b00bec539fd012b906b8a781a	gix/examples/stats.rs
EOF
git add . && git commit -m "r2-base"
store_tree "r2-base"

rm -Rf ./*
# from d7ad650d3 of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 4ecbcfe3b44969cd567f3514d84248d734acd6a3	git-revision/tests/spec/parse/navigate/colon_symbol.rs
100644 blob 5bab15914e09a595f9debbad8fa6b86b020c49c8	git-revision/tests/spec/parse/navigate/mod.rs
100644 blob 5f30877b015559fbb8d5058fcb005c43c49d3049	git-revision/tests/spec/parse/navigate/tilde_symbol.rs
100644 blob 979de34893206c26034b0ef370e40b9241f6f376	git-sequencer/CHANGELOG.md
100644 blob 65279fd869b49ba69157a3caf5d3c8ea17f557a7	git-sequencer/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-sequencer/src/lib.rs
100644 blob cc220fcac635129c05bdd6b23f57ef1f3ef1650a	git-submodule/CHANGELOG.md
100644 blob a198ab48930dd9d0e7fc39abb36131167314c6b9	git-submodule/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-submodule/src/lib.rs
100644 blob 4a0c6e1d835e155a0652a0b536af0c9aa0cf9741	git-tempfile/CHANGELOG.md
100644 blob 3aecbefa04a091629de00dae370b83c0fe7de676	git-tempfile/Cargo.toml
120000 blob 965b606f331b51d566b46025f9ff311a7aad0c12	git-tempfile/LICENSE-APACHE
120000 blob 76219eb72e8524f15c21ec93b9b2592da49b5460	git-tempfile/LICENSE-MIT
100644 blob 0de38724b9f3660894cec1ba177f84539238ffb9	git-tempfile/README.md
100644 blob aae9e27984c1ae02d906949b0a1e386d3a731073	git-tempfile/examples/delete-tempfiles-on-sigterm-interactive.rs
100644 blob d51692756498b7b683c3a69e80cec9f291be35f6	git-tempfile/examples/delete-tempfiles-on-sigterm.rs
100644 blob 55550a7188e2085d82bb28bab4add362a57e5c90	git-tempfile/examples/try-deadlock-on-cleanup.rs
100644 blob 804db1ed63448af8157aff2c80bf56e1a84b152c	git-tempfile/src/forksafe.rs
100644 blob 65efdb2b8892e437e9f67ad7b4e22ee2f50db19a	git-tempfile/src/fs/create_dir.rs
100644 blob bf26d819b88c598769f011b76277c7cb2fcbd275	git-tempfile/src/fs/mod.rs
100644 blob ac7b212fadd4df8bebae73bfd426f5234ab84510	git-tempfile/src/fs/remove_dir.rs
100644 blob df857107439c348e73c3e3e63575027cb024d15a	git-tempfile/src/handle.rs
100644 blob 7b17b144ad21ef64d142f99f6c68c7fb10b949cb	git-tempfile/src/handler.rs
100644 blob f33c6c8946766315c233e8e3af590a9d7c9be47e	git-tempfile/src/lib.rs
100644 blob 6332850b3aa1c3c57adad12e972d345da21a3800	git-tempfile/tests/all.rs
100644 blob d8f68872fcfa3ec20a1594c62f8a238124f14084	git-tempfile/tests/tempfile/fs/create_dir.rs
100644 blob 9c028bc7ce02dbe56c5845676e98436b036ce114	git-tempfile/tests/tempfile/fs/mod.rs
100644 blob f9323604cae7993b235719ee0972cf2002a87b11	git-tempfile/tests/tempfile/fs/remove_dir.rs
100644 blob 48475baed44a5fe6dc4b6a41c15bcc27853a828a	git-tempfile/tests/tempfile/handle.rs
100644 blob da91c56c66553d87d0dd4aa3a71baf6f9ccafea7	git-tempfile/tests/tempfile/mod.rs
100644 blob 1241f04e63422675b83851a1ada4e95d1b2cb76f	git-tix/CHANGELOG.md
100644 blob ea9674febfdef592bb8a3ed8195117e683043523	git-tix/Cargo.toml
100644 blob 3a6cd994a5363fa97f600b539de71bb8b5191bf1	git-tix/src/lib.rs
100644 blob 731b548f114d44015e5d81747d7ee06654209555	git-transport/CHANGELOG.md
100644 blob 212fcfed31b3468c8e0c5fc21a62768512a5417d	git-transport/Cargo.toml
100644 blob b66a8fafae9f781b9b38cc85e8d1b8e5602e7d5b	git-transport/src/client/async_io/bufread_ext.rs
100644 blob 924a5d4962761635c9df6ceda344a993003183a7	git-transport/src/client/async_io/connect.rs
100644 blob 6cb1a500e183bdec876221c6987e776c789b9675	git-transport/src/client/async_io/mod.rs
100644 blob 127b086ef3628a61ff2bcfbe12d4cdaf8544d61b	git-transport/src/client/async_io/request.rs
100644 blob ea73f5e0943f8dd56407bd7317b80bae2888ebb3	git-transport/src/client/async_io/traits.rs
100644 blob b994b7ab504da613a2259992b41de4aef3e3969f	git-transport/src/client/blocking_io/bufread_ext.rs
100644 blob 8ed6cd5c03c6a0a464f0f427afb271a0d224e65e	git-transport/src/client/blocking_io/connect.rs
100644 blob 1a20cd6fadeca100793dc2522768cd3ad32cb54b	git-transport/src/client/blocking_io/file.rs
100644 blob 3312aa6b9c0193a596b3db2fc909e2261f66e2dc	git-transport/src/client/blocking_io/http/curl/mod.rs
100644 blob e5635d28a0c37d8b27dab21815a63235b8d94f92	git-transport/src/client/blocking_io/http/curl/remote.rs
100644 blob e5ef33d6e8cfa4597adad8ac4bcbb5ee55299a2a	git-transport/src/client/blocking_io/http/mod.rs
100644 blob f429961e0da151664349a455807cdaf6e1727ebe	git-transport/src/client/blocking_io/http/redirect.rs
100644 blob 7c68b166ea2f9ec69eba3d41b7bce4365b048b40	git-transport/src/client/blocking_io/http/reqwest/mod.rs
100644 blob e24d86dc20b91e8cca00de823edd9262378ae0d2	git-transport/src/client/blocking_io/http/reqwest/remote.rs
100644 blob 5b163f892a1ab4e03381fbe6d80eae199e192f05	git-transport/src/client/blocking_io/http/traits.rs
100644 blob dfb3752af95f81d12212c3bf7ca6a47f7c67a3e5	git-transport/src/client/blocking_io/mod.rs
100644 blob a1104b95a1361db0ede4eeea387d9715ff46c8ba	git-transport/src/client/blocking_io/request.rs
100644 blob 97ba4c192f7be204162e13149aa7f52cd6228c64	git-transport/src/client/blocking_io/ssh/mod.rs
100644 blob 9d0be16de08b6aafb16596f507cb1d7a3990b7c1	git-transport/src/client/blocking_io/ssh/program_kind.rs
100644 blob 971d95992293f22a45128b77d9c7281d8ccff753	git-transport/src/client/blocking_io/ssh/tests.rs
100644 blob 7b2eaebbe198e8209ca4566ffb00c22dd79f95ed	git-transport/src/client/blocking_io/traits.rs
100644 blob 3c1cbec7e7d0fd9a8ed5ea113f2efd7b2b0b720e	git-transport/src/client/capabilities.rs
100644 blob 704a3458763dea46b16c6c12175422394e2ce533	git-transport/src/client/git/async_io.rs
100644 blob e49fe9315a3d6eec9a937890d188edfcd7199674	git-transport/src/client/git/blocking_io.rs
100644 blob 3684f00873cdf82e1691d14927a8f0130c3d4b0d	git-transport/src/client/git/mod.rs
100644 blob 0eeb6f145d6b139d6c081e4ec2751905b6e046ba	git-transport/src/client/mod.rs
100644 blob c60f733e584a4c201646d08114527ea00e6a3f91	git-transport/src/client/non_io_types.rs
100644 blob 9d036cb4bfbdbc2f2b07a40c171cf983f483deb8	git-transport/src/client/traits.rs
100644 blob f87864f08145adcc33c5a10c29f76360d2f0fee6	git-transport/src/lib.rs
100644 blob 19a22801885d18067c3dab22c0017fff935bb1b1	git-transport/tests/async-transport.rs
100644 blob af94be7e9f4b861f68961f5c9fe4398401c02222	git-transport/tests/blocking-transport-http.rs
100644 blob ab5365afddece55200f8a30aed22f20f7b95fe49	git-transport/tests/blocking-transport.rs
100644 blob 5bbf6291ac7000e27a5062d26994a1138153c33b	git-transport/tests/client/blocking_io/http/mock.rs
100644 blob cd3b97cac1c93cc79129ce3cfb8b1657cfd210e5	git-transport/tests/client/blocking_io/http/mod.rs
100644 blob 57861695c5989af3de46ffa0c56d34d09b391ba0	git-transport/tests/client/blocking_io/mod.rs
100644 blob 5d300d1218ddf868ce51f3c6d8ecc05313e8edf4	git-transport/tests/client/capabilities.rs
100644 blob 4abab468654e94c2f4ce95c7d480ac03bd722548	git-transport/tests/client/git.rs
100644 blob 15ff276eadfd7a3b87de12a45ef4a457bb609a72	git-transport/tests/client/mod.rs
100644 blob f08593978612021a361886b8879cbcb3f539c9d2	git-transport/tests/fixtures/http-401.response
100644 blob 1d0780739288b899dd5f47b4db7fb746923fecb9	git-transport/tests/fixtures/http-404.response
100644 blob 0d38823e2a52050d61e6554a681e8e3b6b4b8e25	git-transport/tests/fixtures/http-500.response
100644 blob 9aeadfcc5622bc6c5b809381e8766ab920c97865	git-transport/tests/fixtures/v1/clone.request
100644 blob 014bd57aa08e26321886ee8b40edab4aaa6b491c	git-transport/tests/fixtures/v1/clone.response
100644 blob 07375e3ca2e41e77ca471f9e20747073bc32d9e4	git-transport/tests/fixtures/v1/http-clone.response
100644 blob 4b1d110410e779fc0c9b9d91fe22d437827c5a8f	git-transport/tests/fixtures/v1/http-handshake.response
100644 blob 20808368a49141542388c878e12e8bfdd3776b82	git-transport/tests/fixtures/v1/push.request
100644 blob 3a156d3f84e8bf501095ed4769287e4c87f25b55	git-transport/tests/fixtures/v1/push.response
100644 blob c68f30ff4a3fbff6f0876a9d45484dd1db7a476b	git-transport/tests/fixtures/v2/clone.request
100644 blob e536d55553f47218ba5d4380772b4d8a4e3d53cc	git-transport/tests/fixtures/v2/clone.response
100644 blob e5f36bd7cc9c0eb49fd076d7352dfd376bb8f4d2	git-transport/tests/fixtures/v2/http-fetch.response
100644 blob f22e506aba79247115f327925ac1eb4ddd5fc636	git-transport/tests/fixtures/v2/http-handshake-lowercase-headers.response
100644 blob 95fec47eaad1ae1dedc79d7998ea23eaf7c4fd4e	git-transport/tests/fixtures/v2/http-handshake-service-announced.response
100644 blob ebb41785d75decc4672b4b06857a42cafef5e2ad	git-transport/tests/fixtures/v2/http-handshake.response
100644 blob a11151fdc01f0289bcab9d29ac85e2199f2c57cb	git-transport/tests/fixtures/v2/http-lsrefs.response
100644 blob 745e1ea988fbde9e4697ad9f0907dc6272d95d4a	git-transport/tests/fixtures/v2/http-no-newlines-handshake.response
100644 blob e9b9992cd46fe7e9c6e521b2e899bacccf5e6ed0	git-traverse/CHANGELOG.md
100644 blob a2458e7754fba49133ce2f080c88750ee101d072	git-traverse/Cargo.toml
100644 blob de5a67e23048d15c2869ee4f7a9e7b62f12a7807	git-traverse/src/commit.rs
100644 blob 3cf6d2b3af7aa85b4bb24871ad86f473871d273c	git-traverse/src/lib.rs
100644 blob b8891b7e37151749b93e9d291d60d3af3c5e03df	git-traverse/src/tree/breadthfirst.rs
100644 blob ae5354198d92a8c454055c725e43af35827d09b8	git-traverse/src/tree/mod.rs
100644 blob e8e8705f8e9711786d4dbc465ff7820de0e7cc2f	git-traverse/src/tree/recorder.rs
100644 blob 1bee4f196e838c49996c87a503f0cd6898ec0c9c	git-traverse/tests/Cargo.toml
100644 blob 5bf10916b172794a750ffab0249bd9033cc5a10a	git-traverse/tests/commit/mod.rs
100644 blob 3b3aa46d92867e38deff18a997b6d74bdebd5489	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_commits.tar.xz
100644 blob 36ed2e1aaad3dab8335a3ec94ffc0321130605eb	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_commits_with_dates.tar.xz
100644 blob 47eb88b0ff8a58a708d4373093837409910ef758	git-traverse/tests/fixtures/generated-archives/make_traversal_repo_for_trees.tar.xz
100755 blob fe7d265f0d35eae336a46101ef250e7b695b5dce	git-traverse/tests/fixtures/make_traversal_repo_for_commits.sh
100755 blob bb557a416c02d80d008fec9152752ecc2fb49d18	git-traverse/tests/fixtures/make_traversal_repo_for_commits_with_dates.sh
100755 blob 263ecf27f4779cb82fff799886d6d49c5f4f57bd	git-traverse/tests/fixtures/make_traversal_repo_for_trees.sh
100644 blob 5a61dde6515ed21f08d4c205b6327904625032bf	git-traverse/tests/traverse.rs
100644 blob 8dfee12de23c3ed2981b71d38a2221bb8b53acf1	git-traverse/tests/tree/mod.rs
100644 blob 8beaedfa8297379abc5653c30443e6b0036d6467	git-tui/Cargo.toml
100644 blob 86aec11b1489b268ba26bc74bb033b3994b16b14	git-tui/src/main.rs
100644 blob 5412dc380c46e86b99e3383a74173e886c877851	git-url/CHANGELOG.md
100644 blob e775ca4c301440f1aa1d08fe63142f3a108f4d9f	git-url/Cargo.toml
100644 blob 393ce9116c66098c4c86498c6bef14ae001a0c77	git-url/src/expand_path.rs
100644 blob d3c5780888799ed5fb321f709ef933f9bf1b759d	git-url/src/impls.rs
100644 blob 9e3de2b79aa937dc2429a03727de95a239fa576c	git-url/src/lib.rs
100644 blob 0878d954b1828cf76d1c74e3cea793f78bd1e3fe	git-url/src/parse.rs
100644 blob 5f491596af2d941b80c3f19900901a7d018f16fd	git-url/src/scheme.rs
100644 blob 58e83d9fb13bd87eee342279becad7d91e1c2f70	git-url/tests/access/mod.rs
100644 blob a93db8f2f0e6060615f750a5e2c3d161a7e37a75	git-url/tests/expand_path/mod.rs
100644 blob 082e0ef07a1e1c22e29e394723c64a2177a96ac1	git-url/tests/parse/file.rs
100644 blob b6190d9c49b03296aa5a7bb79c42be7245b41f31	git-url/tests/parse/invalid.rs
100644 blob 19f09c2ab9b9639de9df8e1749c1182b41fa6599	git-url/tests/parse/mod.rs
100644 blob 06001013994885422734a988447191a6682f37f6	git-url/tests/parse/ssh.rs
100644 blob 9efd0163648a9f5b994712c1712b5f6b101bff31	git-url/tests/url.rs
100644 blob 89cde36ac39547728f2a59cad8cd1cb06dcd2223	git-validate/CHANGELOG.md
100644 blob 9e7ea8dedb5e5cc72281b7b77cc0b4e7ec1c7638	git-validate/Cargo.toml
100644 blob fd603aeb8b8fadf9d651725f2e16aa86bd90b7e3	git-validate/src/lib.rs
100644 blob eb1f25a812d67915889536a678a9979696f37af1	git-validate/src/reference.rs
100644 blob 91ceec18549240d3cecf3a5482c72582bca3a435	git-validate/src/tag.rs
100644 blob 1b0bdf8378ba1127e3489ac4c6b0b70e5040e1ee	git-validate/tests/all.rs
100644 blob a75d2a0768a9602da34c548b1538c13bc9bdc467	git-validate/tests/validate/mod.rs
100644 blob 620572b12ea6ec39db393008604fd3b6886ef727	git-validate/tests/validate/reference.rs
100644 blob 5523e2f2181cc660104b9d8cb142781f05b42291	git-validate/tests/validate/tagname.rs
100644 blob 2ff545f32f928c5d337fdd390916e6703514f6e9	git-worktree/CHANGELOG.md
100644 blob 909c43ab0aa8369b8b47f8c7887ae71e5cbc899a	git-worktree/Cargo.toml
100644 blob b75ceb753dfb0bf8d4e45e97e62ae064197b4262	git-worktree/src/fs/cache/mod.rs
100644 blob 448464345cebca4e65b42ba7908f7d37fed9b7c2	git-worktree/src/fs/cache/platform.rs
100644 blob 30c8fa04eb24e36025840da379c226e9e840473f	git-worktree/src/fs/cache/state.rs
100644 blob 64daab9cea33b85d43d423f10b8fd3bde4fd219a	git-worktree/src/fs/capabilities.rs
100644 blob 2f0ac0f8f59be33ee8c8db406d54f9919be01874	git-worktree/src/fs/mod.rs
100644 blob 734a4988b40c873bdf63a7dd257617410f9d5d5b	git-worktree/src/fs/stack.rs
100644 blob 6f094777896d8cea62a9ca84c4fd463f3c0f57ad	git-worktree/src/index/checkout.rs
100644 blob 924f127a4b20b345d58cc09b60245263176c965a	git-worktree/src/index/entry.rs
100644 blob def2655345fa7e614aef38241d070272d11d7eda	git-worktree/src/index/mod.rs
100644 blob 9a67e0289ee850c72494771581402861ceee51fa	git-worktree/src/lib.rs
100644 blob a297e73cd876c9e91f9f7c634c95b6b4245e72a2	git-worktree/src/os.rs
100644 blob 16fa0a5d7c6e5b8f3b1fa9c0a91b615be94b29f7	git-worktree/tests/fixtures/generated-archives/make_dangerous_symlink.tar.xz
100644 blob 2eb265bd062868382f00c57576aff23d19682c9b	git-worktree/tests/fixtures/generated-archives/make_ignore_and_attributes_setup.tar.xz
100644 blob 970248cf7c4ba6379ae823d1cf7e644569de445b	git-worktree/tests/fixtures/generated-archives/make_ignorecase_collisions.tar.xz
100644 blob b9fd4d7e33e9b16b60563c65ca6fd16700cf26ef	git-worktree/tests/fixtures/generated-archives/make_mixed_without_submodules.tar.xz
100644 blob 56edd71ff4a6ba564f271dfea79e693afd3fdccd	git-worktree/tests/fixtures/generated-archives/make_special_exclude_case.tar.xz
100644 blob 31437285a37a396243633014ce41774fd54f8e5e	git-worktree/tests/fixtures/make_dangerous_symlink.sh
100644 blob e176e6c8140a4c838c06830403b7ab951d54022d	git-worktree/tests/fixtures/make_ignore_and_attributes_setup.sh
100644 blob c06d3926429b14ee8743a41372e3eb3c9fe386ef	git-worktree/tests/fixtures/make_ignore_setup.sh
100644 blob 4a20f2080c575a15118e728f5ba3c86b339276fd	git-worktree/tests/fixtures/make_ignorecase_collisions.sh
100755 blob 618c60ca54bf2dc1fbef1500454adfe481908698	git-worktree/tests/fixtures/make_mixed_without_submodules.sh
100644 blob 17a279eda9d6836d30d7d3fbd3cb64cf55876305	git-worktree/tests/fixtures/make_special_exclude_case.sh
100644 blob cdae7eb15e4ca0b89e2a80b3ebfd0bf1c50de126	git-worktree/tests/worktree-multi-threaded.rs
100644 blob 39b8dea85bec01b546bea47ef5a7ea33d8ef00eb	git-worktree/tests/worktree-single-threaded.rs
100644 blob 434e6efa34c2537d55b1881ae707a0e938a32ade	git-worktree/tests/worktree/fs/cache/create_directory.rs
100644 blob 896bca3cea0f9326cf15c3f3c2647d15c04477cc	git-worktree/tests/worktree/fs/cache/ignore_and_attributes.rs
100644 blob ed8b4a222657b64f7037ced269163d69c233d923	git-worktree/tests/worktree/fs/cache/mod.rs
100644 blob 65064b1a507d5443fe66163069aea598a11063b8	git-worktree/tests/worktree/fs/mod.rs
100644 blob 41e3406ceadaea3ce05e65717b1abd2844bbf6d8	git-worktree/tests/worktree/fs/stack/mod.rs
100644 blob 829b0f59698439edb20aeaf6d21030e436445208	git-worktree/tests/worktree/index/checkout.rs
100644 blob 24370dce48a9b09f6c376c518c2fcd11a737c804	git-worktree/tests/worktree/index/mod.rs
100644 blob bc68cc4be7b6ad76450839eeac9d55327985b759	git-worktree/tests/worktree/mod.rs
100644 blob 7197952e50aa77dead829d931c5803bc29367a06	gitoxide-core/CHANGELOG.md
100644 blob ce87494673145fcef3dbf5509269b50fec71429e	gitoxide-core/Cargo.toml
100644 blob a8118c56ae71a93cf7f09c7883d4ff16c7d51cef	gitoxide-core/src/commitgraph/mod.rs
100644 blob e79e502d43fdc4177e34588d912c49a7a9991d3b	gitoxide-core/src/commitgraph/verify.rs
100644 blob f33c28b5b8a9e2958204d26530a74ac3a90f8c1e	gitoxide-core/src/hours/core.rs
100644 blob 38258de0f8212f8a3a09cc019246cabe87c3e812	gitoxide-core/src/hours/mod.rs
100644 blob 5738efd8ac3c5ba7e7055773ed6d195e5a2eee00	gitoxide-core/src/hours/util.rs
100644 blob 0bccdf127bfd807e6bffbef6e815561519b67c78	gitoxide-core/src/index/checkout.rs
100644 blob e6ca66aee51c3f06fc333a0920bff7b771151b1f	gitoxide-core/src/index/entries.rs
100644 blob d1e543608d1c8fda88d91d3b43095fd2e084e73f	gitoxide-core/src/index/information.rs
100644 blob c8556e6e01f6197036e16640cce6e975d04dff01	gitoxide-core/src/index/mod.rs
100644 blob 0d8dac5a892c89fc68ece3bce9e0ca504dce1c82	gitoxide-core/src/lib.rs
100644 blob 85c1e51c96a7df2d1121027ad1af862ed995e148	gitoxide-core/src/mailmap.rs
100644 blob 86455dd671297fc59d057df81762205990476c5f	gitoxide-core/src/net.rs
100644 blob c58aca146c94ea9b47fc8ae562a220f26c59706d	gitoxide-core/src/organize.rs
100644 blob 098cca5df58e13837fc3f4240906629d7b2682c2	gitoxide-core/src/pack/create.rs
100644 blob ae9aa972a45fd08d73e025ee1956bc67f2ae56a9	gitoxide-core/src/pack/explode.rs
100644 blob aec6343380d544bb9e1ee541e9453c6ea13d9b8f	gitoxide-core/src/pack/index.rs
100644 blob 0805418356cf47aaa5a477ebdad2eebb00b4f2db	gitoxide-core/src/pack/mod.rs
100644 blob eb847870e9d57acdceaa205370e5d517df242ace	gitoxide-core/src/pack/multi_index.rs
100644 blob fa512bba534f6aa5a17fd4516d2cfc6afcc762a8	gitoxide-core/src/pack/receive.rs
100644 blob c56afcefc389ae326d0d7b78186c7d20755ffaff	gitoxide-core/src/pack/verify.rs
100644 blob 59d521b3f192bbb0e0ac5d3e1cd57ecb72b72c46	gitoxide-core/src/repository/clone.rs
100644 blob d1f4c400e9f7a321af93a4b972d6d478776d4126	gitoxide-core/src/repository/commit.rs
100644 blob efeb2c53ef8be868d69a7f8e07dd719bb6858177	gitoxide-core/src/repository/config.rs
100644 blob b9b93b9d057da76abc3ba0cc5744994025f631c7	gitoxide-core/src/repository/credential.rs
100644 blob c6f620b9ff9149b0a3ed18cccc7804849863a28a	gitoxide-core/src/repository/exclude.rs
100644 blob 1b980965487d2b0edcfaad60a11709f881fccb58	gitoxide-core/src/repository/fetch.rs
100644 blob d14fca2227eeb09ff610fef995a78e02de43dea4	gitoxide-core/src/repository/index.rs
100644 blob 14531df7a6c757d7fafee05e29ab3dbf483e79c7	gitoxide-core/src/repository/mailmap.rs
100644 blob 48e3ad87d13a5206b30633bbafb8c24e0bfd822a	gitoxide-core/src/repository/mod.rs
100644 blob e41d95439b438012a42d37adf39c193c0b3a2a89	gitoxide-core/src/repository/odb.rs
100644 blob 78c8acc0b86c19e04fa3a0a7a4cdc8c68fbb83c1	gitoxide-core/src/repository/remote.rs
100644 blob 05c8b2833525c4da2ac725971b9cba35cbd8d5ab	gitoxide-core/src/repository/revision/explain.rs
100644 blob e87590f5d9d8bc0167f182ec56604c50675ec63e	gitoxide-core/src/repository/revision/list.rs
100644 blob 5e5dda98af523783a2811cfc3c0e783948a37ca7	gitoxide-core/src/repository/revision/mod.rs
100644 blob 727b56cf4b77623ea226d2e768297d2007343d2d	gitoxide-core/src/repository/revision/previous_branches.rs
100644 blob a66d6b8a914589f72a752bca7d61b86e4f11f673	gitoxide-core/src/repository/revision/resolve.rs
100644 blob cbe9b913352d3d4162e2a09232b32ea6e79e237c	gitoxide-core/src/repository/tree.rs
100644 blob 3b8c2579250e628dc1b141b876f71a689ae48952	gitoxide-core/src/repository/verify.rs
100644 blob fc8f25ace4a901bf771d31dac604cdf866db3261	gix-config-value/CHANGELOG.md
100644 blob e4f4aa4a2126c83404a2eacfefb9cdfc94905aa7	gix-config-value/Cargo.toml
100644 blob 908e11a300a637d5e15ec3776dda2709d981c5e1	gix-config-value/src/boolean.rs
100644 blob 558e81a769381d971d7b1f5b42df693db5b2523b	gix-config-value/src/color.rs
100644 blob 7214b76e4c82ab1b4402a9a8721bf948cc828196	gix-config-value/src/integer.rs
100644 blob 5b4f3fa76c34101c422de7a2dd61e88b0ff0279e	gix-config-value/src/lib.rs
100644 blob 99ee5cf2efef1d3781393a26f59b3bfa6c89db04	gix-config-value/src/path.rs
100644 blob 239679c703d6cb5131ad664312914f8a50312581	gix-config-value/src/types.rs
100644 blob 8fe8b43f0d43b5c675574e6d763cdeb171008220	gix-config-value/tests/value/boolean.rs
100644 blob 1db7fe2e03124687ec6d754e81714d429bd14a07	gix-config-value/tests/value/color.rs
100644 blob 9de2ab1ca2a8bc7d2698794b125f2c49cd722573	gix-config-value/tests/value/integer.rs
100644 blob 13a33c9d8f6b0f7d4b0929e0863000c8871b09d4	gix-config-value/tests/value/main.rs
100644 blob 10970296dbfd77368f8428a5ca3622962f747103	gix-config-value/tests/value/path.rs
100644 blob 76f58d7a187155013b02aa11a0903de46e8af0fe	gix-path/CHANGELOG.md
100644 blob 6a17963b4a7fecdfdd7d38ea0fff8ab4d3030838	gix-path/Cargo.toml
100644 blob 6a949529f588978cb5feb35ea7034c703565e062	gix-path/src/convert.rs
100644 blob 70a9bc53f0704bc3e4a14263501c71355785cfaf	gix-path/src/lib.rs
100644 blob 807cb74ccf62957bb4645e7d5d0b8020e3764696	gix-path/src/realpath.rs
100644 blob ff878bde141a040d3c65b9672feb612a5bb54b9a	gix-path/src/spec.rs
100644 blob 7920910d725bcdbea486e340c490c76a115dfd8c	gix-path/src/util.rs
100644 blob bf67c26241b3a56c1a1fa32f5d64c2057712bd83	gix-path/tests/convert/mod.rs
100644 blob 65a8885eb5722781d01941767d4a6667b4b63c3c	gix-path/tests/convert/normalize.rs
100644 blob 95ebafc5e32d846cf4c980b6a6fbd8ac84809de9	gix-path/tests/path.rs
100644 blob df5a59090d7fdcb1f91d1ef2543b5dcd84266e07	gix-path/tests/realpath/mod.rs
100644 blob 7d78de7e70bfa44cb30347204b24f228a4b5ac27	gix-path/tests/util/mod.rs
100644 blob 2f5046508dafd8c25fe245c1314ee0cbf3e84788	gix-sec/CHANGELOG.md
100644 blob 032d318537b0b8903d386b799be700261fc4ab41	gix-sec/Cargo.toml
100644 blob 7c3df38bdd204a62929492f6afb66d647737f612	gix-sec/src/identity.rs
100644 blob 2f35d98cf7f363873ae93efb40df349f62b8e295	gix-sec/src/lib.rs
100644 blob 5bd5f2c325f64dadf313cae344fe8973dc2a5952	gix-sec/src/permission.rs
100644 blob 274c5b7807d807c04b9a702944d436ad9d3b2a35	gix-sec/src/trust.rs
100644 blob da0fef3bf49d4fc0381479850eafef2ed107ce53	gix-sec/tests/identity/mod.rs
100644 blob ce9c24122ab3303b9a8cd08fbc4de4fa1e65c6a9	gix-sec/tests/sec.rs
100644 blob e8a55463ab7ec43354595b236df56a3e30235691	gix/CHANGELOG.md
100644 blob 2e4f5ab099d6d05403c46714e561fd1a95c4cdc8	gix/Cargo.toml
100644 blob fb5cbf0c14bc2216e794f459a1c173061fe4980a	gix/examples/clone.rs
100644 blob aad2db759084cc51206967c56ed8e4f4417ede97	gix/examples/init-repo-and-commit.rs
100644 blob e75a50beea5cb93ba49d4c95db84837db91ac953	gix/examples/interrupt-handler-allows-graceful-shutdown.rs
100644 blob fcd02a2a38e76659682a57ed3f06349aa828b921	gix/examples/reversible-interrupt-handlers.rs
100644 blob 8c41a40fcefcff7b00bec539fd012b906b8a781a	gix/examples/stats.rs
EOF
git add . && git commit -m "r2-change"
store_tree "r2-change"

rm -Rf ./*
# from faaf791cc of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 75306517965a54731b94dca8bace640e856af115	rustfmt.toml
100644 blob 653db61c27bc8676db65293799ee37b900d21cd1	src/plumbing-cli.rs
100644 blob b30a19098c101818cb46aa1730681bb439827a0a	src/plumbing/main.rs
100644 blob 2e32346bdd9dd1092020cc8e8294adaa7c8019fc	src/plumbing/mod.rs
100644 blob 7a0f73b3e46b1dbfe518aed268098722d26b4ce1	src/plumbing/options.rs
100644 blob 1414704becf8540873778861a10dc1d04a3cb04a	src/porcelain-cli.rs
100644 blob af1c7d8b9a565a8c952b5680948a1ca689f1b267	src/porcelain/main.rs
100644 blob 68886bba05f53a7b1e4597a243a4e513092cd4db	src/porcelain/mod.rs
100644 blob 9d3a25aaab6ad92ff8781655690a3421bbd1d6de	src/porcelain/options.rs
100644 blob d9b5ae05af9bcb05e83b7296e11505cce7b3c9bf	src/shared.rs
100644 blob 8236acaa18c95b5e1de26d6e50f54989c43c83df	tasks.md
EOF
git add . && git commit -m "r3-base"
store_tree "r3-base"

rm -Rf ./*
# from faaf791cc of `gitoxide`
while read -r _ _ _ path; do
  mkdir -p ${path%/*} && touch $path
done <<EOF
100644 blob 75306517965a54731b94dca8bace640e856af115	rustfmt.toml
100644 blob 1414704becf8540873778861a10dc1d04a3cb04a	src/ein.rs
100644 blob 653db61c27bc8676db65293799ee37b900d21cd1	src/gix.rs
100644 blob b30a19098c101818cb46aa1730681bb439827a0a	src/plumbing/main.rs
100644 blob 2e32346bdd9dd1092020cc8e8294adaa7c8019fc	src/plumbing/mod.rs
100644 blob 7a0f73b3e46b1dbfe518aed268098722d26b4ce1	src/plumbing/options.rs
100644 blob af1c7d8b9a565a8c952b5680948a1ca689f1b267	src/porcelain/main.rs
100644 blob 68886bba05f53a7b1e4597a243a4e513092cd4db	src/porcelain/mod.rs
100644 blob 9d3a25aaab6ad92ff8781655690a3421bbd1d6de	src/porcelain/options.rs
100644 blob d9b5ae05af9bcb05e83b7296e11505cce7b3c9bf	src/shared.rs
100644 blob 8236acaa18c95b5e1de26d6e50f54989c43c83df	tasks.md
EOF
git add . && git commit -m "r3-change"
store_tree "r3-change"

git -c diff.renames=0 show > baseline-3.no-renames
git -c diff.renames=1 show > baseline-3.with-renames
git -c diff.renames=0 show HEAD~2 > baseline-2.no-renames
git -c diff.renames=1 show HEAD~2 > baseline-2.with-renames
git -c diff.renames=0 show HEAD~4 > baseline.no-renames
git -c diff.renames=1 show HEAD~4 > baseline.with-renames

echo 1 >src/plumbing/main.rs
echo 2 >src/plumbing/mod.rs
echo 3 >src/plumbing/options.rs
git add src/plumbing && git commit -m "r4-base"
store_tree "r4-base"

mkdir src/plumbing/options
git mv src/plumbing/options.rs src/plumbing/options/mod.rs
git mv src/plumbing src/plumbing-renamed
git commit -m "r4-dir-rename-non-identity"
store_tree "r4-dir-rename-non-identity"

git checkout -b conflicting @~1
git rm src/plumbing/main.rs
git commit -m "remove main.rs"

git checkout main
git merge conflicting || :

echo not-empty >will-add
git add --intent-to-add will-add

# a file with skip-worktree flag, which has no bearing on tree/index diffs.
git update-index --skip-worktree src/shared.rs
rm src/shared.rs

mv ../*.tree .
