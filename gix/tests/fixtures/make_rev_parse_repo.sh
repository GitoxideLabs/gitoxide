#!/usr/bin/env bash
set -eu -o pipefail

# This fixture embeds historical SHA-1 reflog entries verbatim.
export GIT_DEFAULT_HASH=sha1

git init -q
touch new && git add new && git commit -m "init"

git branch old

cat <<EOF >.git/logs/refs/heads/old
be2f093f0588eaeb71e1eff7451b18c2a9b1d765 e5e8178a701fefed30096dab2077a85301a83236 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013175 +0200	rebase (pick): Add content to file for blame test
e5e8178a701fefed30096dab2077a85301a83236 1f669d5a11d15a027cedd59133c98c329b4ac835 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013175 +0200	rebase (pick): Start exploring gix APIs in gix-blame
1f669d5a11d15a027cedd59133c98c329b4ac835 6147adf463958a075d303256535c744c66044217 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013175 +0200	rebase (pick): Use gix-traverse for graph traversal
6147adf463958a075d303256535c744c66044217 030d31303b02708b58e842adc05bea82195277fd Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Get diff between trees
030d31303b02708b58e842adc05bea82195277fd 06531798076f91e8b20774421574663584447531 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Get diff between two files
06531798076f91e8b20774421574663584447531 1ec1f705acf577ff7078319288fe221342775fce Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Start to keep track of lines to blame
1ec1f705acf577ff7078319288fe221342775fce b672807d77c10c98cae932fd3de7216c7f01c041 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Start to keep track of blamed lines
b672807d77c10c98cae932fd3de7216c7f01c041 b6f991f7142ceb0f3c827539451fed5455f3baa9 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Add Blame
b6f991f7142ceb0f3c827539451fed5455f3baa9 bd3ddce5a2a14d28cb6ca6cdcd42c5684ad1a7cb Sebastian Thiel <sebastian.thiel@icloud.com> 1727013176 +0200	rebase (pick): Wrap diffing in loop
bd3ddce5a2a14d28cb6ca6cdcd42c5684ad1a7cb aa60a14edda90aa6f6449db9bd637f7a9ff3b705 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Run loop more than once
aa60a14edda90aa6f6449db9bd637f7a9ff3b705 d142441a200f39d202cfea47d7934c69300dd214 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Record commit ids instead of blob ids
d142441a200f39d202cfea47d7934c69300dd214 4ef253f53fd9bb001f0cee5b101b7f83a1b200a4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Compare result against git blame
4ef253f53fd9bb001f0cee5b101b7f83a1b200a4 f26c70961807e5bd2d4577fdb889f56f9b3851da Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Turn for into loop
f26c70961807e5bd2d4577fdb889f56f9b3851da b50e3fe71d2b5b113039cbc9ea3e2e0b352cdc80 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Move new_lines_to_blame out of closure
b50e3fe71d2b5b113039cbc9ea3e2e0b352cdc80 737633a61c938ff66b392246bec26876405c30f1 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Remove unnecessary code
737633a61c938ff66b392246bec26876405c30f1 2f9920cfe076de238e55c6fc57ca8c8afba3c04d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013177 +0200	rebase (pick): Assign remaining lines to last suspect before break
2f9920cfe076de238e55c6fc57ca8c8afba3c04d 99a43992db091942127ed1dea116fe89477f0717 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Add comment
99a43992db091942127ed1dea116fe89477f0717 743c1bec4ab89973541a601f320c1c130427ab9e Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Extract diffing into function
743c1bec4ab89973541a601f320c1c130427ab9e 8717f4f3a3dcc6672b4b5e155147d88ed1f61e3c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Rename file in fixture
8717f4f3a3dcc6672b4b5e155147d88ed1f61e3c 4ec9eaee85e6633146d7368b507ad1e00cd6ee13 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Skip commits that don’t affect file
4ec9eaee85e6633146d7368b507ad1e00cd6ee13 bfe8156989715d65715689bb4490136f15ffaeb4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Add first test for multiline hunk blames
bfe8156989715d65715689bb4490136f15ffaeb4 cdb90ebbea5161bb2577ef4724621950481ccc16 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013178 +0200	rebase (pick): Fix clippy issues
cdb90ebbea5161bb2577ef4724621950481ccc16 964a987c440338316dee5d99c49de3745244dd5c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Add first test for history with deleted lines
964a987c440338316dee5d99c49de3745244dd5c 6b1de47c0c3c07583b7368de40b3acd524f659eb Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Fix clippy issues
6b1de47c0c3c07583b7368de40b3acd524f659eb 7ea0d6b72f1e0b7f8d4d85ffffd3dd23f49d8eac Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Add test for more than one unchanged section
7ea0d6b72f1e0b7f8d4d85ffffd3dd23f49d8eac 52a6e015f8b171fec1d7a33ec736313f4c2a2cb7 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Add test for changed lines
52a6e015f8b171fec1d7a33ec736313f4c2a2cb7 1b57d6eac19a1b9bdc443b794570b9df500a580a Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Add test for single changed line between unchanged ones
1b57d6eac19a1b9bdc443b794570b9df500a580a a99044cf551999c99a8832cc61a82e6ac3d6783c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013179 +0200	rebase (pick): Add missing test setup
a99044cf551999c99a8832cc61a82e6ac3d6783c 448ec1cfe4591fb0681605af996534e42c619a0c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Add test for lines added before other line
448ec1cfe4591fb0681605af996534e42c619a0c c0714af30b4435f676e412af7bc56149c1eef42c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Extract diffing into DiffRecorder
c0714af30b4435f676e412af7bc56149c1eef42c 343bc7013d5465ec3f55ac02e5bf19b7efeae4d8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Split DiffRecorder into ChangeRecorder and process_changes
343bc7013d5465ec3f55ac02e5bf19b7efeae4d8 e7c17ace42c4d6fd4ef4c35626dff377530e66ea Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Add test for lines added around other line
e7c17ace42c4d6fd4ef4c35626dff377530e66ea 2a1a08000a7cb7ce805f2f60ad19b1c0aa7fc416 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Replace platform-dependent sed by echo
2a1a08000a7cb7ce805f2f60ad19b1c0aa7fc416 ab582dc55798479dba96f32d174a5bec63deca13 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013180 +0200	rebase (pick): Add semicolon recommended by clippy
ab582dc55798479dba96f32d174a5bec63deca13 5208add5eb3e2c27d2dfa1dad741b789238f1f26 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Annotate type
5208add5eb3e2c27d2dfa1dad741b789238f1f26 2fafaffe51c795a613795af341a456e150b8738a Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Turn if into match
2fafaffe51c795a613795af341a456e150b8738a ccff0e7080d8dc2f24e15e9e08f56834096d03c7 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Add assert_hunk_valid!
ccff0e7080d8dc2f24e15e9e08f56834096d03c7 148e5e912002669a14ce6200722453b4b1147943 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Extend test for delete line
148e5e912002669a14ce6200722453b4b1147943 81595fc1f80ca56d4ee4a16638caec3bdbe6080a Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Add test for switched lines
81595fc1f80ca56d4ee4a16638caec3bdbe6080a 800be3c011a57e03fe680f9ecdabef96aea1ab86 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013181 +0200	rebase (pick): Condense empty lines
800be3c011a57e03fe680f9ecdabef96aea1ab86 14b0953cb69be4eea64159b0b505535b8fc8f30f Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Take worktree_path as argument
14b0953cb69be4eea64159b0b505535b8fc8f30f 83a72ebe7982d2c61199eb5173dd5239e10063f8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Simplify tests through macro
83a72ebe7982d2c61199eb5173dd5239e10063f8 e0c2d9c23958d846fefd939e37ddf0fb2035794d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Add first tests for process_changes
e0c2d9c23958d846fefd939e37ddf0fb2035794d a0d3e0c00e68281242c1cca8a91955a5e614f6eb Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Replace PathBuf by Path
a0d3e0c00e68281242c1cca8a91955a5e614f6eb 8450e3e7e20556e381ad691bc14acb3d28ce5d6c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Add UnblamedHunk to be able to track offset
8450e3e7e20556e381ad691bc14acb3d28ce5d6c 107cd874ac0bcca3f8bd92ccb8e7978bd67fd8cc Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Track offset in process_changes
107cd874ac0bcca3f8bd92ccb8e7978bd67fd8cc 9642df2aca1f567d485439a2376585e6e788f874 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013182 +0200	rebase (pick): Fix clippy issues
9642df2aca1f567d485439a2376585e6e788f874 6ccd04637c76fd66ba0252b90ec38790a03af9fe Sebastian Thiel <sebastian.thiel@icloud.com> 1727013183 +0200	rebase (pick): Add BlameEntry::new
6ccd04637c76fd66ba0252b90ec38790a03af9fe fff3acb902fe98b1fc34c4ac6636c917f437abe2 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013183 +0200	rebase (pick): Correctly handle non-inclusive end
fff3acb902fe98b1fc34c4ac6636c917f437abe2 62ab6872e0b30f3cece6b53aff853fb3c9cd2ce7 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013183 +0200	rebase (pick): Add UnblamedHunk::new
62ab6872e0b30f3cece6b53aff853fb3c9cd2ce7 d729de0c4082eba0b5f7e4102c59a576bf5386ac Sebastian Thiel <sebastian.thiel@icloud.com> 1727013183 +0200	rebase (pick): Remove obsolete comment
d729de0c4082eba0b5f7e4102c59a576bf5386ac ebb4608b5b0f32f8ca9f2f5ef43f0ea6769c1c15 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013183 +0200	rebase (pick): Keep two ranges in UnblamedHunk for clarity
ebb4608b5b0f32f8ca9f2f5ef43f0ea6769c1c15 4b83df072f06ccca3714b783d5def13293777e6c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013184 +0200	rebase (pick): Better separate offset and offset_in_destination
4b83df072f06ccca3714b783d5def13293777e6c 9aa1150052b8c52a80ac8143b41e2efa6f4546a6 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013184 +0200	rebase (pick): Better handle offset when no changes left
9aa1150052b8c52a80ac8143b41e2efa6f4546a6 88b0ae37ee4de001c3a6e0065196e0abcb90dc4b Sebastian Thiel <sebastian.thiel@icloud.com> 1727013184 +0200	rebase (pick): Better handle offset when no changes left
88b0ae37ee4de001c3a6e0065196e0abcb90dc4b 57f449306f4c9407b9a12b88620b7f67e47554d7 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013184 +0200	rebase (pick): Add UnblamedHunk::offset
57f449306f4c9407b9a12b88620b7f67e47554d7 eacb6dbf4c2fb6cfe4b07754ebca375985443b55 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013185 +0200	rebase (pick): Add test for change before addition
eacb6dbf4c2fb6cfe4b07754ebca375985443b55 49d109c9d0119b5ea2a8d943aaa1e8673ebf4326 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013185 +0200	rebase (pick): Add more test for process_changes
49d109c9d0119b5ea2a8d943aaa1e8673ebf4326 452eb2c0e50ce0d7f787d8168c43c19c0520b031 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013185 +0200	rebase (pick): More reliably detect group header
452eb2c0e50ce0d7f787d8168c43c19c0520b031 2a44b49073bc7b04dcdb558ef342e1e5d49d1287 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013185 +0200	rebase (pick): Remove unnecessary clone
2a44b49073bc7b04dcdb558ef342e1e5d49d1287 8a17dc8d43a6683cda8fa947ed2480937f358876 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013185 +0200	rebase (pick): Record unchanged hunk at end of file
8a17dc8d43a6683cda8fa947ed2480937f358876 7ec3723ea9e2c8936ce032e99d4d537ae6e3cb5a Sebastian Thiel <sebastian.thiel@icloud.com> 1727013186 +0200	rebase (pick): Add test for same line changed twice
7ec3723ea9e2c8936ce032e99d4d537ae6e3cb5a 8c2bb0c67b0ecb8c89fc13e7d9ac7114e945910d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013186 +0200	rebase (pick): Take offset into account
8c2bb0c67b0ecb8c89fc13e7d9ac7114e945910d 5f9c0dc79334414fcc6376f409057a342df824f8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013186 +0200	rebase (pick): Add Offset
5f9c0dc79334414fcc6376f409057a342df824f8 46050fb227ade4cf0fcfc3ee06d3570710b7411e Sebastian Thiel <sebastian.thiel@icloud.com> 1727013186 +0200	rebase (pick): Add LineRange
46050fb227ade4cf0fcfc3ee06d3570710b7411e b8356c2a18732c08e2b860b2f17e8ca96d689eae Sebastian Thiel <sebastian.thiel@icloud.com> 1727013186 +0200	rebase (pick): Add BlameEntry::with_offset
b8356c2a18732c08e2b860b2f17e8ca96d689eae ec6b1fc791a7f86c3de540e8a2f5a1fb82b21bbd Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Add Offset::Deleted
ec6b1fc791a7f86c3de540e8a2f5a1fb82b21bbd 4115b3031e7833fee5766d045a234f35c49602eb Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Count line numbers in destination
4115b3031e7833fee5766d045a234f35c49602eb 08ccbcc2069282c0a7e7254a23724171508809e3 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Take hunks with deletions only into account
08ccbcc2069282c0a7e7254a23724171508809e3 7b114132d03c468a9cd97836901553658c9792de Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Sort result in test
7b114132d03c468a9cd97836901553658c9792de 306cdbab5457c323d1201aa8a59b3639f600a758 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Replace FIXED THIS LINE WHILE WE DON"T PARSE RIGHT ANGLE BRACKETS IN COMMENTS CORRECTLY
306cdbab5457c323d1201aa8a59b3639f600a758 2bc1920f88be1e1a86ee42da5678aaba3eae0b62 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013187 +0200	rebase (pick): Add match arm for unchanged hunks
2bc1920f88be1e1a86ee42da5678aaba3eae0b62 0f8acdff1bd41b225695d73df77145d845abe52e Sebastian Thiel <sebastian.thiel@icloud.com> 1727013188 +0200	rebase (pick): Extract process_change
0f8acdff1bd41b225695d73df77145d845abe52e 71fa0a3d6e6006b7cdaa47882a1b768a6df57510 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013188 +0200	rebase (pick): Start adding tests for process_change
71fa0a3d6e6006b7cdaa47882a1b768a6df57510 e3e637916f78d98805c5674fc77ea985a78c49b5 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013188 +0200	rebase (pick): Take hunk offset into account for new hunk
e3e637916f78d98805c5674fc77ea985a78c49b5 3ad406c5bc83bccff1e138518e07c28add2889dd Sebastian Thiel <sebastian.thiel@icloud.com> 1727013188 +0200	rebase (pick): Fill match arms
3ad406c5bc83bccff1e138518e07c28add2889dd d3283d3065d0979872b96916688cdef77b07bb40 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013188 +0200	rebase (pick): Add more tests for unchanged lines
d3283d3065d0979872b96916688cdef77b07bb40 e3496f4a75fbf59da917e9fab835dfe87e1f04bd Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Add test for deleted hunk
e3496f4a75fbf59da917e9fab835dfe87e1f04bd c957b4066b651211095872f976475247b1e5ccba Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Add more tests for added lines
c957b4066b651211095872f976475247b1e5ccba 5024d2883d9952ed91104390ce40f533197adc90 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Fix offset of new hunk
5024d2883d9952ed91104390ce40f533197adc90 28cad2fb5cbc5f1a524edac228a92a1cbf79acaf Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Fix offset when no overlap
28cad2fb5cbc5f1a524edac228a92a1cbf79acaf 4f09a34c3e654981ff9583332e9e2e84a97c0c10 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Consume addition when before hunk
4f09a34c3e654981ff9583332e9e2e84a97c0c10 cf472a7d74bb4e42e9ebfae02bd7d1125169a244 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013189 +0200	rebase (pick): Add semicolons recommended by clippy
cf472a7d74bb4e42e9ebfae02bd7d1125169a244 3b8e0a8b835f5f0750d67bdb5cfe0e17d5665a43 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Fix offset of new hunk
3b8e0a8b835f5f0750d67bdb5cfe0e17d5665a43 cd8a75bef58553881a117b83bde606e893c8f520 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Fix expectation in test
cd8a75bef58553881a117b83bde606e893c8f520 1151d562fc9ee0f4046de5b7500cecbff3ddab52 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Apply offset to chunk after deletion
1151d562fc9ee0f4046de5b7500cecbff3ddab52 758b0f0bc7aa8c7af8916626b9b071916bfdd25e Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Split hunk that contains deletion
758b0f0bc7aa8c7af8916626b9b071916bfdd25e 492ae43596ed79cb5ec932f84535fd47adbc4c48 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Split addition related to more than one hunk
492ae43596ed79cb5ec932f84535fd47adbc4c48 f89244dd117e164d4c8a5649db03be2154901bfe Sebastian Thiel <sebastian.thiel@icloud.com> 1727013190 +0200	rebase (pick): Rename range to range_in_blamed_file
f89244dd117e164d4c8a5649db03be2154901bfe 043f52a34333083f1ff1fe4c9866e5af8d225d21 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013191 +0200	rebase (pick): Add range_in_original_file to BlameEntry
043f52a34333083f1ff1fe4c9866e5af8d225d21 5f2317245826c90affc15197d034d2c64c75d74d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013191 +0200	rebase (pick): Assert baseline length matches result's length
5f2317245826c90affc15197d034d2c64c75d74d fa0d99807f829f01963ade102d78203e6ea03a98 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013191 +0200	rebase (pick): Coalesce adjacent blame entries
fa0d99807f829f01963ade102d78203e6ea03a98 2add197d8add4794e3782d6c497a7bfb0878cbd6 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013191 +0200	rebase (pick): Add more context to comment
2add197d8add4794e3782d6c497a7bfb0878cbd6 b1d63db34c1fdcb6158614a6c20d73c3585fe04d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013191 +0200	rebase (pick): Fix added lines overlapping unblamed hunk's start
b1d63db34c1fdcb6158614a6c20d73c3585fe04d b38124fc61152c3c08b73a0348ed5750d62d307b Sebastian Thiel <sebastian.thiel@icloud.com> 1727013192 +0200	rebase (pick): Use LineRange::with_offset
b38124fc61152c3c08b73a0348ed5750d62d307b 42a94dfbe767dfffe5fdbd8fff167c0d4c20de2d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013192 +0200	rebase (pick): Don't consume addition preceding unblamed hunk
42a94dfbe767dfffe5fdbd8fff167c0d4c20de2d a38b0dce3129575f1a8dadc0bd01d99bf90a036c Sebastian Thiel <sebastian.thiel@icloud.com> 1727013192 +0200	rebase (pick): Don't consume unchanged lines preceding unblamed hunk
a38b0dce3129575f1a8dadc0bd01d99bf90a036c 0ecba206895c6b891420bd7697f574845ba84392 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013192 +0200	rebase (pick): Change offset for changes when there is no hunk
0ecba206895c6b891420bd7697f574845ba84392 960d7ba6b3041f69df9ae3e808390a1ff3182014 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013192 +0200	rebase (pick): Don't consume deletion preceding unblamed hunk
960d7ba6b3041f69df9ae3e808390a1ff3182014 f88ee99fe751b27baf550321c8c0821f9a6cab1d Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Don't consume unblamed hunk following deletion
f88ee99fe751b27baf550321c8c0821f9a6cab1d 363e20c16cdc57b8e6fef7b6b4bd0f54818babb0 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Remove leftover dbg!
363e20c16cdc57b8e6fef7b6b4bd0f54818babb0 5f50657ff8038d16997b91e5532c113c27368b52 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Handle addition enclosing unblamed hunk
5f50657ff8038d16997b91e5532c113c27368b52 4ad458c321ebe0382d85d9d8fe133ec69ac93145 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Handle unchanged lines extending past unblamed hunk
4ad458c321ebe0382d85d9d8fe133ec69ac93145 8e51500552812254718602c8bdd1fb84b5b15850 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Add test for unblamed hunk enclosing deletion
8e51500552812254718602c8bdd1fb84b5b15850 5fa546bb4c52656084c54dea55031c8c1f384998 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013193 +0200	rebase (pick): Take offset into account only once for addition
5fa546bb4c52656084c54dea55031c8c1f384998 04ad9a8f18b9a3d13c875b36adb93f5ecd0bbd05 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013194 +0200	rebase (pick): Simplify a branch that processes unchanged lines
04ad9a8f18b9a3d13c875b36adb93f5ecd0bbd05 04ad9a8f18b9a3d13c875b36adb93f5ecd0bbd05 Sebastian Thiel <sebastian.thiel@icloud.com> 1727013194 +0200	rebase (finish): returning to refs/heads/gix-blame
04ad9a8f18b9a3d13c875b36adb93f5ecd0bbd05 34ce2fb4bdd85a77b9c7b1d6b10eb4f7e8323bfb Sebastian Thiel <sebastian.thiel@icloud.com> 1727014492 +0200	commit: update lock file to match manifests
34ce2fb4bdd85a77b9c7b1d6b10eb4f7e8323bfb a6678f9233315a5126abc19e69b1ed2a11aedb1e Sebastian Thiel <sebastian.thiel@icloud.com> 1727014506 +0200	checkout: moving from gix-blame to merge
b09092c545f35555d806ce69d54fda7da9b9e9b8 90ffb1c62d2903469a131d841d5044df8c5a20cc Sebastian Thiel <sebastian.thiel@icloud.com> 1727014618 +0200	checkout: moving from merge to reports
90ffb1c62d2903469a131d841d5044df8c5a20cc be2f093f0588eaeb71e1eff7451b18c2a9b1d765 Sebastian Thiel <sebastian.thiel@icloud.com> 1727014626 +0200	checkout: moving from reports to main
be2f093f0588eaeb71e1eff7451b18c2a9b1d765 35c7213710d253387a3d7c1cc7ad80546bf782d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727014630 +0200	pull --ff-only: Fast-forward
35c7213710d253387a3d7c1cc7ad80546bf782d4 34ce2fb4bdd85a77b9c7b1d6b10eb4f7e8323bfb Sebastian Thiel <sebastian.thiel@icloud.com> 1727014638 +0200	checkout: moving from main to gix-blame
34ce2fb4bdd85a77b9c7b1d6b10eb4f7e8323bfb eea819870326cf8e8ba32d6b92d32f66b8ec40bf Sebastian Thiel <sebastian.thiel@icloud.com> 1727075720 +0200	commit: remove gix-blame changelog while the crate isn't published
eea819870326cf8e8ba32d6b92d32f66b8ec40bf b13a62feb5d538aa50c2661c407711684bcac09b Sebastian Thiel <sebastian.thiel@icloud.com> 1727078620 +0200	commit: refactor: separate production code from tests
0cab0f0b7ee03d2c7d40bd5b4a8a8da2c8ffd1a5 35c7213710d253387a3d7c1cc7ad80546bf782d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727096420 +0200	checkout: moving from gix-blame to main
35c7213710d253387a3d7c1cc7ad80546bf782d4 04845cae0b72cc88801ea58e2ddf3d2826a4f8dc Sebastian Thiel <sebastian.thiel@icloud.com> 1727096469 +0200	checkout: moving from main to freelist
5ef4d5de3733648f5376a6f53fad378847eead53 35c7213710d253387a3d7c1cc7ad80546bf782d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727096473 +0200	reset: moving to 35c7213710d253387a3d7c1cc7ad80546bf782d4
0cac69077e738cb22914e77a9a9dd3fd712d5670 35c7213710d253387a3d7c1cc7ad80546bf782d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727103008 +0200	checkout: moving from freelist to main
35c7213710d253387a3d7c1cc7ad80546bf782d4 73a7d15fb9030081a64803aacfd8d2ac7babf904 Sebastian Thiel <sebastian.thiel@icloud.com> 1727103009 +0200	pull --ff-only: Fast-forward
73a7d15fb9030081a64803aacfd8d2ac7babf904 73a7d15fb9030081a64803aacfd8d2ac7babf904 Sebastian Thiel <sebastian.thiel@icloud.com> 1727104030 +0200	checkout: moving from main to protocol-shallow-v1
b723a67809395e6d6bc4e2b7d624fe880af57d46 73a7d15fb9030081a64803aacfd8d2ac7babf904 Sebastian Thiel <sebastian.thiel@icloud.com> 1727106179 +0200	checkout: moving from protocol-shallow-v1 to main
73a7d15fb9030081a64803aacfd8d2ac7babf904 b723a67809395e6d6bc4e2b7d624fe880af57d46 Sebastian Thiel <sebastian.thiel@icloud.com> 1727106185 +0200	checkout: moving from main to protocol-shallow-v1
0d3b480e5e7d27c308fb5f76f36831dfc7af3e8f 73a7d15fb9030081a64803aacfd8d2ac7babf904 Sebastian Thiel <sebastian.thiel@icloud.com> 1727119593 +0200	checkout: moving from protocol-shallow-v1 to main
73a7d15fb9030081a64803aacfd8d2ac7babf904 612896d7ec15c153d3d48012c75aaf85d10a5abe Sebastian Thiel <sebastian.thiel@icloud.com> 1727119595 +0200	pull --ff-only: Fast-forward
612896d7ec15c153d3d48012c75aaf85d10a5abe 01722e908ab0676add52b21c69f349f2cada8bae Sebastian Thiel <sebastian.thiel@icloud.com> 1727182021 +0200	checkout: moving from main to gix-blame
01722e908ab0676add52b21c69f349f2cada8bae b09092c545f35555d806ce69d54fda7da9b9e9b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727183690 +0200	checkout: moving from gix-blame to merge
b09092c545f35555d806ce69d54fda7da9b9e9b8 da8965bffb04935a88d58a56545986da88d51524 Sebastian Thiel <sebastian.thiel@icloud.com> 1727188216 +0200	checkout: moving from merge to count-sh
dd94f57db82a9bf5833e290b3300a747001bb1eb 612896d7ec15c153d3d48012c75aaf85d10a5abe Sebastian Thiel <sebastian.thiel@icloud.com> 1727199134 +0200	checkout: moving from count-sh to main
612896d7ec15c153d3d48012c75aaf85d10a5abe 01722e908ab0676add52b21c69f349f2cada8bae Sebastian Thiel <sebastian.thiel@icloud.com> 1727199140 +0200	checkout: moving from main to gix-blame
01722e908ab0676add52b21c69f349f2cada8bae 78bc6e002ca2fbf883fbbf1d65ccab4351ef214f Sebastian Thiel <sebastian.thiel@icloud.com> 1727199142 +0200	pull --ff-only: Fast-forward
78bc6e002ca2fbf883fbbf1d65ccab4351ef214f 78bc6e002ca2fbf883fbbf1d65ccab4351ef214f Sebastian Thiel <sebastian.thiel@icloud.com> 1727199395 +0200	reset: moving to HEAD
cf9c23a2400cdcbc06cf47b7369128b4254328c6 b09092c545f35555d806ce69d54fda7da9b9e9b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727247201 +0200	checkout: moving from gix-blame to merge
46a86e9f23d366891a7f0d2219e03aaae68f3292 f99175e539f8f3e580f908aa9bfd92d74edda453 Sebastian Thiel <sebastian.thiel@icloud.com> 1727332125 +0200	checkout: moving from merge to traverse/oldest-first
3f0bcef04dde9935f5613a4e86d75023120b6b87 3f0bcef04dde9935f5613a4e86d75023120b6b87 Sebastian Thiel <sebastian.thiel@icloud.com> 1727339483 +0200	reset: moving to HEAD
6862c27e671cbfd8caae549813ea01eeb753bd0b 612896d7ec15c153d3d48012c75aaf85d10a5abe Sebastian Thiel <sebastian.thiel@icloud.com> 1727354273 +0200	checkout: moving from traverse/oldest-first to main
612896d7ec15c153d3d48012c75aaf85d10a5abe 20f9b3f361b46226be102a065cbb0fbaa83ae2db Sebastian Thiel <sebastian.thiel@icloud.com> 1727354276 +0200	pull --ff-only: Fast-forward
20f9b3f361b46226be102a065cbb0fbaa83ae2db 46a86e9f23d366891a7f0d2219e03aaae68f3292 Sebastian Thiel <sebastian.thiel@icloud.com> 1727354278 +0200	checkout: moving from main to merge
4f92140febf4e9a13d7490b36c04fbf3fc63a5ad 20f9b3f361b46226be102a065cbb0fbaa83ae2db Sebastian Thiel <sebastian.thiel@icloud.com> 1727354282 +0200	reset: moving to 20f9b3f361b46226be102a065cbb0fbaa83ae2db
e18e2042cf9e8c056052b6b3c53f169e89fd2cc8 e18e2042cf9e8c056052b6b3c53f169e89fd2cc8 Sebastian Thiel <sebastian.thiel@icloud.com> 1727425833 +0200	reset: moving to HEAD
fe8ef4d7fd000703cc8269b3501de728d1b676aa fe8ef4d7fd000703cc8269b3501de728d1b676aa Sebastian Thiel <sebastian.thiel@icloud.com> 1727446294 +0200	reset: moving to HEAD
feab227571893518c6e1ebc8a843539b85688642 feab227571893518c6e1ebc8a843539b85688642 Sebastian Thiel <sebastian.thiel@icloud.com> 1727458473 +0200	reset: moving to HEAD
bdf9e8990315cf8b50a953f06ef66efa7c794ee4 bdf9e8990315cf8b50a953f06ef66efa7c794ee4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727513179 +0200	reset: moving to HEAD
bdf9e8990315cf8b50a953f06ef66efa7c794ee4 bdf9e8990315cf8b50a953f06ef66efa7c794ee4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727513241 +0200	checkout: moving from merge to @
bdf9e8990315cf8b50a953f06ef66efa7c794ee4 bdf9e8990315cf8b50a953f06ef66efa7c794ee4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727513250 +0200	checkout: moving from bdf9e8990315cf8b50a953f06ef66efa7c794ee4 to merge
eb37dc36d8c42f5a7714c641244ce4a13111b0a1 20f9b3f361b46226be102a065cbb0fbaa83ae2db Sebastian Thiel <sebastian.thiel@icloud.com> 1727701276 +0200	checkout: moving from merge to main
20f9b3f361b46226be102a065cbb0fbaa83ae2db 2261de470aeb77be080f9e423e1513bde85d9cc0 Sebastian Thiel <sebastian.thiel@icloud.com> 1727701277 +0200	pull --ff-only: Fast-forward
2261de470aeb77be080f9e423e1513bde85d9cc0 2261de470aeb77be080f9e423e1513bde85d9cc0 Sebastian Thiel <sebastian.thiel@icloud.com> 1727701286 +0200	checkout: moving from main to merge
a4d590b5b1afa60b8c320bf65ff393dea0362b42 a4d590b5b1afa60b8c320bf65ff393dea0362b42 Sebastian Thiel <sebastian.thiel@icloud.com> 1727714726 +0200	reset: moving to HEAD
e0b09d2764fd02a2b69340d9b3aef9773ae899ce e0b09d2764fd02a2b69340d9b3aef9773ae899ce Sebastian Thiel <sebastian.thiel@icloud.com> 1727720439 +0200	reset: moving to HEAD
c6019028d488965b50451ca0fffe0ac8e3a0d0c2 c6019028d488965b50451ca0fffe0ac8e3a0d0c2 Sebastian Thiel <sebastian.thiel@icloud.com> 1727725328 +0200	reset: moving to HEAD
90399698b87019d115a86897d3eea5d75da30745 2261de470aeb77be080f9e423e1513bde85d9cc0 Sebastian Thiel <sebastian.thiel@icloud.com> 1727758831 +0200	checkout: moving from merge to main
2261de470aeb77be080f9e423e1513bde85d9cc0 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1727758833 +0200	pull --ff-only: Fast-forward
5ffccd2f08d70576347e3ae17a66ca5a60f1d81c 90399698b87019d115a86897d3eea5d75da30745 Sebastian Thiel <sebastian.thiel@icloud.com> 1727758837 +0200	checkout: moving from main to merge
90399698b87019d115a86897d3eea5d75da30745 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1727758840 +0200	merge main: Fast-forward
16890d41b27956f3b655ed3fa5169d4879abd6c9 16890d41b27956f3b655ed3fa5169d4879abd6c9 Sebastian Thiel <sebastian.thiel@icloud.com> 1727868289 +0200	reset: moving to HEAD
f0ce2e568d1a00d200935e5cf0daaca26e9a430c f0ce2e568d1a00d200935e5cf0daaca26e9a430c Sebastian Thiel <sebastian.thiel@icloud.com> 1727868991 +0200	reset: moving to HEAD
ca2b2f8f378f17ecf466ab0a5960bfb191f0ee90 cf9c23a2400cdcbc06cf47b7369128b4254328c6 Sebastian Thiel <sebastian.thiel@icloud.com> 1727869287 +0200	checkout: moving from merge to gix-blame
cf9c23a2400cdcbc06cf47b7369128b4254328c6 acfb3c7a960a44aee1d9f965e729a24aa38d4927 Sebastian Thiel <sebastian.thiel@icloud.com> 1727869291 +0200	reset: moving to FETCH_HEAD
acfb3c7a960a44aee1d9f965e729a24aa38d4927 61c9768eb02c1414cb5c164bd76f7fe668ab0a39 Sebastian Thiel <sebastian.thiel@icloud.com> 1727869716 +0200	commit: remove unnecessary gitoxide-core/gitoxide 'blame' feature.
61c9768eb02c1414cb5c164bd76f7fe668ab0a39 16b5058679061642bc78a3de440313e5e080a09d Sebastian Thiel <sebastian.thiel@icloud.com> 1727869933 +0200	commit: Add missing crate-metdata in documentation
fa76de61880b6c04d8e1f7379c2f653efe56dd72 ca2b2f8f378f17ecf466ab0a5960bfb191f0ee90 Sebastian Thiel <sebastian.thiel@icloud.com> 1727875968 +0200	checkout: moving from gix-blame to merge
aded67480e139f49131c526ee1cdefbd9d4bdb28 aded67480e139f49131c526ee1cdefbd9d4bdb28 Sebastian Thiel <sebastian.thiel@icloud.com> 1727896122 +0200	reset: moving to HEAD
41481e9f72b847985135715b02905fb0286ac8e5 fa76de61880b6c04d8e1f7379c2f653efe56dd72 Sebastian Thiel <sebastian.thiel@icloud.com> 1727937888 +0200	checkout: moving from merge to gix-blame
fa76de61880b6c04d8e1f7379c2f653efe56dd72 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1727938014 +0200	checkout: moving from gix-blame to gix-blame-rewritten
ae7012f93e50a302b79715aa533b980e516909d4 41481e9f72b847985135715b02905fb0286ac8e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1727943482 +0200	checkout: moving from gix-blame-rewritten to merge
1ab2bfd11c4b1d1de74243ff42f40b62837c55d4 1ab2bfd11c4b1d1de74243ff42f40b62837c55d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1727948058 +0200	reset: moving to HEAD
f41b4b862cee36b3b9b81b6b178e6cf182f2957d f41b4b862cee36b3b9b81b6b178e6cf182f2957d Sebastian Thiel <sebastian.thiel@icloud.com> 1727976446 +0200	reset: moving to HEAD
fc6eef2f8b833dcd7138277f1c8c27798b2294bb fc6eef2f8b833dcd7138277f1c8c27798b2294bb Sebastian Thiel <sebastian.thiel@icloud.com> 1727978199 +0200	reset: moving to HEAD
2c64702061fcd314f4e5267cde630536504facf8 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1728030064 +0200	checkout: moving from merge to main
5ffccd2f08d70576347e3ae17a66ca5a60f1d81c 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1728030076 +0200	checkout: moving from main to progress
06f84de3256812b40d8aede53e8b07b5e24f88e3 5ffccd2f08d70576347e3ae17a66ca5a60f1d81c Sebastian Thiel <sebastian.thiel@icloud.com> 1728031207 +0200	checkout: moving from progress to main
5ffccd2f08d70576347e3ae17a66ca5a60f1d81c c76e6b4249d7f821b0abd916a2366d42de6d3db5 Sebastian Thiel <sebastian.thiel@icloud.com> 1728031209 +0200	pull --ff-only: Fast-forward
c76e6b4249d7f821b0abd916a2366d42de6d3db5 2c64702061fcd314f4e5267cde630536504facf8 Sebastian Thiel <sebastian.thiel@icloud.com> 1728031213 +0200	checkout: moving from main to merge
5ffccd2f08d70576347e3ae17a66ca5a60f1d81c c76e6b4249d7f821b0abd916a2366d42de6d3db5 Sebastian Thiel <sebastian.thiel@icloud.com> 1728031575 +0200	reset: moving to c76e6b4249d7f821b0abd916a2366d42de6d3db5
a4c687daefa36775c032d6c57a4c3826c2b73657 c76e6b4249d7f821b0abd916a2366d42de6d3db5 Sebastian Thiel <sebastian.thiel@icloud.com> 1728246924 +0200	checkout: moving from merge to main
c76e6b4249d7f821b0abd916a2366d42de6d3db5 c76e6b4249d7f821b0abd916a2366d42de6d3db5 Sebastian Thiel <sebastian.thiel@icloud.com> 1728246935 +0200	checkout: moving from main to git-cat
9c8bc03de99e6494abd9755deef7e7be5577bce2 a4c687daefa36775c032d6c57a4c3826c2b73657 Sebastian Thiel <sebastian.thiel@icloud.com> 1728248372 +0200	checkout: moving from git-cat to merge
bb29cdb89dc42fc0851384ca55c80f52716d0756 c76e6b4249d7f821b0abd916a2366d42de6d3db5 Sebastian Thiel <sebastian.thiel@icloud.com> 1728373621 +0200	checkout: moving from merge to main
c76e6b4249d7f821b0abd916a2366d42de6d3db5 31bdd2ecc6c800dc57faedc9250be6d5fbcc1133 Sebastian Thiel <sebastian.thiel@icloud.com> 1728373625 +0200	pull --ff-only: Fast-forward
31bdd2ecc6c800dc57faedc9250be6d5fbcc1133 31bdd2ecc6c800dc57faedc9250be6d5fbcc1133 Sebastian Thiel <sebastian.thiel@icloud.com> 1728373724 +0200	checkout: moving from main to commit-roundtrip
d29b158635ad2150de04b1de37cb801a23b33e3d bb29cdb89dc42fc0851384ca55c80f52716d0756 Sebastian Thiel <sebastian.thiel@icloud.com> 1728379687 +0200	checkout: moving from commit-roundtrip to merge
bb29cdb89dc42fc0851384ca55c80f52716d0756 d29b158635ad2150de04b1de37cb801a23b33e3d Sebastian Thiel <sebastian.thiel@icloud.com> 1728379694 +0200	checkout: moving from merge to commit-roundtrip
528f549a3572eabb2ad137707a2ef5051d6414a4 bb29cdb89dc42fc0851384ca55c80f52716d0756 Sebastian Thiel <sebastian.thiel@icloud.com> 1728385786 +0200	checkout: moving from commit-roundtrip to merge
bb29cdb89dc42fc0851384ca55c80f52716d0756 31bdd2ecc6c800dc57faedc9250be6d5fbcc1133 Sebastian Thiel <sebastian.thiel@icloud.com> 1728385790 +0200	checkout: moving from merge to main
31bdd2ecc6c800dc57faedc9250be6d5fbcc1133 f35b1096c6db73842a55e089187d27d1287075ad Sebastian Thiel <sebastian.thiel@icloud.com> 1728385791 +0200	pull --ff-only: Fast-forward
f35b1096c6db73842a55e089187d27d1287075ad bb29cdb89dc42fc0851384ca55c80f52716d0756 Sebastian Thiel <sebastian.thiel@icloud.com> 1728385791 +0200	checkout: moving from main to merge
c76e6b4249d7f821b0abd916a2366d42de6d3db5 f35b1096c6db73842a55e089187d27d1287075ad Sebastian Thiel <sebastian.thiel@icloud.com> 1728385794 +0200	reset: moving to f35b1096c6db73842a55e089187d27d1287075ad
434439883635e3c453428deac08172eeb4500eda 434439883635e3c453428deac08172eeb4500eda Sebastian Thiel <sebastian.thiel@icloud.com> 1728386497 +0200	reset: moving to HEAD
a33a1ec558f503f1bc65717980e30492e6413cb9 a33a1ec558f503f1bc65717980e30492e6413cb9 Sebastian Thiel <sebastian.thiel@icloud.com> 1728386554 +0200	reset: moving to HEAD
3745212abf0353f15fec41556c55ee1d30d69f0a f35b1096c6db73842a55e089187d27d1287075ad Sebastian Thiel <sebastian.thiel@icloud.com> 1728459260 +0200	checkout: moving from merge to main
f35b1096c6db73842a55e089187d27d1287075ad 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728459263 +0200	pull --ff-only: Fast-forward
37c1e4c919382c9d213bd5ca299ed659d63ab45d 3745212abf0353f15fec41556c55ee1d30d69f0a Sebastian Thiel <sebastian.thiel@icloud.com> 1728459341 +0200	checkout: moving from main to merge
3745212abf0353f15fec41556c55ee1d30d69f0a 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728459462 +0200	reset: moving to 37c1e4c919382c9d213bd5ca299ed659d63ab45d
02d10e1ec0960e3dedafad82bf23b47da8bb5818 eef0fe07eff1ff7cfb9eb00bf1ee45868bfe4caf Sebastian Thiel <sebastian.thiel@icloud.com> 1728460068 +0200	checkout: moving from merge to status
7dd58b845a7bf55a0aced5cca075a22fbebec978 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728460088 +0200	reset: moving to 37c1e4c919382c9d213bd5ca299ed659d63ab45d
e9c200e9c772f22974999e9fd906e4ebd94c6572 e9c200e9c772f22974999e9fd906e4ebd94c6572 Sebastian Thiel <sebastian.thiel@icloud.com> 1728487669 +0200	reset: moving to HEAD
e9c200e9c772f22974999e9fd906e4ebd94c6572 02d10e1ec0960e3dedafad82bf23b47da8bb5818 Sebastian Thiel <sebastian.thiel@icloud.com> 1728487670 +0200	checkout: moving from status to merge
45b3557dac8b417e3af85917393c20dc1ce93755 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728626078 +0200	checkout: moving from merge to main
37c1e4c919382c9d213bd5ca299ed659d63ab45d 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728626091 +0200	checkout: moving from main to fix-discovery
1c073c2462eacfeddabd33b1178a55482693a404 1c073c2462eacfeddabd33b1178a55482693a404 Sebastian Thiel <sebastian.thiel@icloud.com> 1728639320 +0200	reset: moving to HEAD
2df1f3b1956bb69e404862d3372d3ffbb99193c7 2df1f3b1956bb69e404862d3372d3ffbb99193c7 Sebastian Thiel <sebastian.thiel@icloud.com> 1728639360 +0200	reset: moving to HEAD
c4cb9a966dccf04ba2a241a1cabe8de09bb4d87f c4cb9a966dccf04ba2a241a1cabe8de09bb4d87f Sebastian Thiel <sebastian.thiel@icloud.com> 1728639512 +0200	reset: moving to HEAD
f8952e4cbfaf9ab7ddc12a028a1cdb821ac9a3b1 45b3557dac8b417e3af85917393c20dc1ce93755 Sebastian Thiel <sebastian.thiel@icloud.com> 1728662142 +0200	checkout: moving from fix-discovery to merge
37c1e4c919382c9d213bd5ca299ed659d63ab45d f8952e4cbfaf9ab7ddc12a028a1cdb821ac9a3b1 Sebastian Thiel <sebastian.thiel@icloud.com> 1728662189 +0200	checkout: moving from merge to fix-discovery
c18ebbeabb3e4bd775cf59bd90e6672749ce9549 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728665901 +0200	checkout: moving from fix-discovery to main
37c1e4c919382c9d213bd5ca299ed659d63ab45d 64872690e60efdd9267d517f4d9971eecd3b875c Sebastian Thiel <sebastian.thiel@icloud.com> 1728665903 +0200	pull --ff-only: Fast-forward
64872690e60efdd9267d517f4d9971eecd3b875c 37c1e4c919382c9d213bd5ca299ed659d63ab45d Sebastian Thiel <sebastian.thiel@icloud.com> 1728721544 +0200	checkout: moving from main to merge
402b04655a89ba00c24516df9cbc7cfc7e671041 64872690e60efdd9267d517f4d9971eecd3b875c Sebastian Thiel <sebastian.thiel@icloud.com> 1728721654 +0200	checkout: moving from merge to main
64872690e60efdd9267d517f4d9971eecd3b875c 402b04655a89ba00c24516df9cbc7cfc7e671041 Sebastian Thiel <sebastian.thiel@icloud.com> 1728721658 +0200	checkout: moving from main to merge
37c1e4c919382c9d213bd5ca299ed659d63ab45d 64872690e60efdd9267d517f4d9971eecd3b875c Sebastian Thiel <sebastian.thiel@icloud.com> 1728721659 +0200	reset: moving to 64872690e60efdd9267d517f4d9971eecd3b875c
bf41e127b31784594ae87dbdda237d420d68f449 bf41e127b31784594ae87dbdda237d420d68f449 Sebastian Thiel <sebastian.thiel@icloud.com> 1728721849 +0200	reset: moving to HEAD
64e0f78a3ff061837ff647da96110be432cc7228 64872690e60efdd9267d517f4d9971eecd3b875c Sebastian Thiel <sebastian.thiel@icloud.com> 1728882001 +0200	checkout: moving from merge to main
64872690e60efdd9267d517f4d9971eecd3b875c 70c4df5418c4018549cfbd48e374f46e112c4d6c Sebastian Thiel <sebastian.thiel@icloud.com> 1728882003 +0200	pull --ff-only: Fast-forward
70c4df5418c4018549cfbd48e374f46e112c4d6c 64e0f78a3ff061837ff647da96110be432cc7228 Sebastian Thiel <sebastian.thiel@icloud.com> 1728882004 +0200	checkout: moving from main to merge
64872690e60efdd9267d517f4d9971eecd3b875c 70c4df5418c4018549cfbd48e374f46e112c4d6c Sebastian Thiel <sebastian.thiel@icloud.com> 1728882006 +0200	reset: moving to 70c4df5418c4018549cfbd48e374f46e112c4d6c
995198d04df0f3a694645669ac25a8172206a359 f9ae557aa5815bbd0ed0272c0f9d9007700d45fb Sebastian Thiel <sebastian.thiel@icloud.com> 1728983088 +0200	checkout: moving from merge to add-gix-diff
795962b107d86f58b1f7c75006da256d19cc80ad 70c4df5418c4018549cfbd48e374f46e112c4d6c Sebastian Thiel <sebastian.thiel@icloud.com> 1728984462 +0200	reset: moving to 70c4df5418c4018549cfbd48e374f46e112c4d6c
6777ecb99306830a3353a0db24caaa69e348ca74 70c4df5418c4018549cfbd48e374f46e112c4d6c Sebastian Thiel <sebastian.thiel@icloud.com> 1728987657 +0200	checkout: moving from add-gix-diff to main
70c4df5418c4018549cfbd48e374f46e112c4d6c f186c2381b91f350813076927bf988d253fe1ad0 Sebastian Thiel <sebastian.thiel@icloud.com> 1728987658 +0200	pull --ff-only: Fast-forward
f186c2381b91f350813076927bf988d253fe1ad0 f186c2381b91f350813076927bf988d253fe1ad0 Sebastian Thiel <sebastian.thiel@icloud.com> 1728987922 +0200	checkout: moving from main to diff-fix
99a553bf4f5abe1a14eb663e65534d4e48cceaea 99a553bf4f5abe1a14eb663e65534d4e48cceaea Sebastian Thiel <sebastian.thiel@icloud.com> 1728989476 +0200	reset: moving to HEAD
99a553bf4f5abe1a14eb663e65534d4e48cceaea 99a553bf4f5abe1a14eb663e65534d4e48cceaea Sebastian Thiel <sebastian.thiel@icloud.com> 1728989583 +0200	reset: moving to HEAD
d42544c3991b1f65e5df1e6280059123e9ca122f d42544c3991b1f65e5df1e6280059123e9ca122f Sebastian Thiel <sebastian.thiel@icloud.com> 1729001961 +0200	reset: moving to HEAD
05c0c868897ccc0352bac5c40e79e619a280e9ad 05c0c868897ccc0352bac5c40e79e619a280e9ad Sebastian Thiel <sebastian.thiel@icloud.com> 1729002538 +0200	reset: moving to HEAD
1a02abe0c9f1c4891848ce00bb288b09f8ae02b0 1a02abe0c9f1c4891848ce00bb288b09f8ae02b0 Sebastian Thiel <sebastian.thiel@icloud.com> 1729005222 +0200	reset: moving to HEAD
53fa8abda6cf96e2afd8082db0d7a9f686d82752 f186c2381b91f350813076927bf988d253fe1ad0 Sebastian Thiel <sebastian.thiel@icloud.com> 1729013254 +0200	checkout: moving from diff-fix to main
f186c2381b91f350813076927bf988d253fe1ad0 155b5e1c3691852b08dc81241423597dc34fa2dc Sebastian Thiel <sebastian.thiel@icloud.com> 1729013256 +0200	pull --ff-only: Fast-forward
155b5e1c3691852b08dc81241423597dc34fa2dc 995198d04df0f3a694645669ac25a8172206a359 Sebastian Thiel <sebastian.thiel@icloud.com> 1729013257 +0200	checkout: moving from main to merge
70c4df5418c4018549cfbd48e374f46e112c4d6c 155b5e1c3691852b08dc81241423597dc34fa2dc Sebastian Thiel <sebastian.thiel@icloud.com> 1729013278 +0200	reset: moving to 155b5e1c3691852b08dc81241423597dc34fa2dc
d23270870e18c306b3cac82e689a97170fa5c013 155b5e1c3691852b08dc81241423597dc34fa2dc Sebastian Thiel <sebastian.thiel@icloud.com> 1729058698 +0200	checkout: moving from merge to main
155b5e1c3691852b08dc81241423597dc34fa2dc b835ea7512d82fe323cfba9ce7d80364b62cf235 Sebastian Thiel <sebastian.thiel@icloud.com> 1729058700 +0200	pull --ff-only: Fast-forward
b835ea7512d82fe323cfba9ce7d80364b62cf235 d23270870e18c306b3cac82e689a97170fa5c013 Sebastian Thiel <sebastian.thiel@icloud.com> 1729061546 +0200	checkout: moving from main to merge
6dba04510b063bbe30ad8fff899c3bd5061400df 6dba04510b063bbe30ad8fff899c3bd5061400df Sebastian Thiel <sebastian.thiel@icloud.com> 1729176653 +0200	reset: moving to HEAD
9e97c724d6ae0a31031f501a8f7075f3ef018ce1 9e97c724d6ae0a31031f501a8f7075f3ef018ce1 Sebastian Thiel <sebastian.thiel@icloud.com> 1729177803 +0200	reset: moving to HEAD
d0067a2e13b7d1f48c7623f60e7b665e76f2a2cc b835ea7512d82fe323cfba9ce7d80364b62cf235 Sebastian Thiel <sebastian.thiel@icloud.com> 1729262502 +0200	checkout: moving from merge to main
b835ea7512d82fe323cfba9ce7d80364b62cf235 c081114ff885ca07032cad994970ed027a62a0cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729262505 +0200	pull --ff-only: Fast-forward
c081114ff885ca07032cad994970ed027a62a0cf c081114ff885ca07032cad994970ed027a62a0cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729262676 +0200	checkout: moving from main to remove-delegates
d1717c3e4335a0e90136447dc42ad555d8e754da c081114ff885ca07032cad994970ed027a62a0cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729316469 +0200	checkout: moving from remove-delegates to main
c081114ff885ca07032cad994970ed027a62a0cf c081114ff885ca07032cad994970ed027a62a0cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729316474 +0200	checkout: moving from main to fix-ci
c9490300b116cf468cca82d87c65c9190e9a6696 c081114ff885ca07032cad994970ed027a62a0cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729322302 +0200	checkout: moving from fix-ci to main
c081114ff885ca07032cad994970ed027a62a0cf 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729322303 +0200	pull --ff-only: Fast-forward
2622936e77d938d6cb441b4e7001dd55374328cd d0067a2e13b7d1f48c7623f60e7b665e76f2a2cc Sebastian Thiel <sebastian.thiel@icloud.com> 1729322328 +0200	checkout: moving from main to merge
155b5e1c3691852b08dc81241423597dc34fa2dc 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729322332 +0200	reset: moving to 2622936e77d938d6cb441b4e7001dd55374328cd
53a40f4afc325d25a6812a443368dab45227b4ce 53a40f4afc325d25a6812a443368dab45227b4ce Sebastian Thiel <sebastian.thiel@icloud.com> 1729361036 +0200	reset: moving to HEAD
53a40f4afc325d25a6812a443368dab45227b4ce 53a40f4afc325d25a6812a443368dab45227b4ce Sebastian Thiel <sebastian.thiel@icloud.com> 1729361140 +0200	reset: moving to HEAD
25a303edcd07bd21c4875dce21fd2148d9c10c05 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729405440 +0200	checkout: moving from merge to main
2622936e77d938d6cb441b4e7001dd55374328cd 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729405453 +0200	checkout: moving from main to improve-error-message
206f5d70fa74c23c56c6cbecc5625234fde930fc 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729407438 +0200	checkout: moving from improve-error-message to main
2622936e77d938d6cb441b4e7001dd55374328cd 206f5d70fa74c23c56c6cbecc5625234fde930fc Sebastian Thiel <sebastian.thiel@icloud.com> 1729407442 +0200	checkout: moving from main to improve-error-message
31f14a18b737cae929767f0e2a2e5e31aaaa1185 25a303edcd07bd21c4875dce21fd2148d9c10c05 Sebastian Thiel <sebastian.thiel@icloud.com> 1729407491 +0200	checkout: moving from improve-error-message to merge
25a303edcd07bd21c4875dce21fd2148d9c10c05 31f14a18b737cae929767f0e2a2e5e31aaaa1185 Sebastian Thiel <sebastian.thiel@icloud.com> 1729407497 +0200	checkout: moving from merge to improve-error-message
2d1fbce2877e0d4c64557f7feb5327d695d114a3 25a303edcd07bd21c4875dce21fd2148d9c10c05 Sebastian Thiel <sebastian.thiel@icloud.com> 1729407506 +0200	checkout: moving from improve-error-message to merge
25a303edcd07bd21c4875dce21fd2148d9c10c05 2d1fbce2877e0d4c64557f7feb5327d695d114a3 Sebastian Thiel <sebastian.thiel@icloud.com> 1729407510 +0200	checkout: moving from merge to improve-error-message
2d1fbce2877e0d4c64557f7feb5327d695d114a3 25a303edcd07bd21c4875dce21fd2148d9c10c05 Sebastian Thiel <sebastian.thiel@icloud.com> 1729407734 +0200	checkout: moving from improve-error-message to merge
3dcf70d1874797752cd29a12b736f0fe2caa8ccf 2d1fbce2877e0d4c64557f7feb5327d695d114a3 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409611 +0200	checkout: moving from merge to improve-error-message
2d1fbce2877e0d4c64557f7feb5327d695d114a3 2622936e77d938d6cb441b4e7001dd55374328cd Sebastian Thiel <sebastian.thiel@icloud.com> 1729409625 +0200	checkout: moving from improve-error-message to main
2622936e77d938d6cb441b4e7001dd55374328cd b36d7efb9743766338ac7bb7fb2399a06fae5e60 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409626 +0200	pull --ff-only: Fast-forward
b36d7efb9743766338ac7bb7fb2399a06fae5e60 3dcf70d1874797752cd29a12b736f0fe2caa8ccf Sebastian Thiel <sebastian.thiel@icloud.com> 1729409646 +0200	checkout: moving from main to merge
2622936e77d938d6cb441b4e7001dd55374328cd b36d7efb9743766338ac7bb7fb2399a06fae5e60 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409649 +0200	reset: moving to b36d7efb9743766338ac7bb7fb2399a06fae5e60
bd948af9d12b006192587939855f438dde2d3bb9 b36d7efb9743766338ac7bb7fb2399a06fae5e60 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409653 +0200	checkout: moving from merge to main
b36d7efb9743766338ac7bb7fb2399a06fae5e60 b36d7efb9743766338ac7bb7fb2399a06fae5e60 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409776 +0200	checkout: moving from main to main
b36d7efb9743766338ac7bb7fb2399a06fae5e60 bd948af9d12b006192587939855f438dde2d3bb9 Sebastian Thiel <sebastian.thiel@icloud.com> 1729409782 +0200	checkout: moving from main to merge
fee8f1b4fe1d659abc073dbcd38f4d4175f1be43 c3d04fa3e1ab3d2e69280f48fd1f00cdf2de676f Sebastian Thiel <sebastian.thiel@icloud.com> 1729496861 +0200	checkout: moving from merge to allow-contructCustomFormat
4910912e2b4957350a7ab8169ba9de956e8d8325 b36d7efb9743766338ac7bb7fb2399a06fae5e60 Sebastian Thiel <sebastian.thiel@icloud.com> 1729499947 +0200	checkout: moving from allow-contructCustomFormat to main
b36d7efb9743766338ac7bb7fb2399a06fae5e60 bcdce6e873904e4dd77070d7b4e75f969b9f0bea Sebastian Thiel <sebastian.thiel@icloud.com> 1729499949 +0200	pull --ff-only: Fast-forward
bcdce6e873904e4dd77070d7b4e75f969b9f0bea 4910912e2b4957350a7ab8169ba9de956e8d8325 Sebastian Thiel <sebastian.thiel@icloud.com> 1729499954 +0200	checkout: moving from main to allow-contructCustomFormat
4910912e2b4957350a7ab8169ba9de956e8d8325 fee8f1b4fe1d659abc073dbcd38f4d4175f1be43 Sebastian Thiel <sebastian.thiel@icloud.com> 1729499960 +0200	checkout: moving from allow-contructCustomFormat to merge
ff7d92ed1a4733fa868de74a27ce2f558a88e1e4 ff7d92ed1a4733fa868de74a27ce2f558a88e1e4 Sebastian Thiel <sebastian.thiel@icloud.com> 1729504521 +0200	reset: moving to HEAD
505a6fdaf58de05ffd2316e94fe23d6d4fcc061d 0bebe524b75346edca219d13c10b52dee3273643 Sebastian Thiel <sebastian.thiel@icloud.com> 1729529020 +0200	checkout: moving from merge to respect-env-variables
a8c0f8b55cf9be630237cb4c2832fcab4714042b a8c0f8b55cf9be630237cb4c2832fcab4714042b Sebastian Thiel <sebastian.thiel@icloud.com> 1729533629 +0200	reset: moving to HEAD
e9b3db8021ad1f8bf7b2ee6ffecd5b1b1c8a38b9 d1717c3e4335a0e90136447dc42ad555d8e754da Sebastian Thiel <sebastian.thiel@icloud.com> 1729535529 +0200	checkout: moving from respect-env-variables to remove-delegates
c081114ff885ca07032cad994970ed027a62a0cf bcdce6e873904e4dd77070d7b4e75f969b9f0bea Sebastian Thiel <sebastian.thiel@icloud.com> 1729535625 +0200	reset: moving to bcdce6e873904e4dd77070d7b4e75f969b9f0bea
c7d477dc6f37d30bc5f0871081b08f7931c43ffa 505a6fdaf58de05ffd2316e94fe23d6d4fcc061d Sebastian Thiel <sebastian.thiel@icloud.com> 1729538643 +0200	checkout: moving from remove-delegates to merge
0fce40d3113b4711c18a9b6cb22782dd2da5727d bcdce6e873904e4dd77070d7b4e75f969b9f0bea Sebastian Thiel <sebastian.thiel@icloud.com> 1729608388 +0200	checkout: moving from merge to main
bcdce6e873904e4dd77070d7b4e75f969b9f0bea 48aa74b911fb874986c244712b7fd5b5cc10070b Sebastian Thiel <sebastian.thiel@icloud.com> 1729608392 +0200	pull --ff-only: Fast-forward
48aa74b911fb874986c244712b7fd5b5cc10070b 0fce40d3113b4711c18a9b6cb22782dd2da5727d Sebastian Thiel <sebastian.thiel@icloud.com> 1729608398 +0200	checkout: moving from main to merge
b36d7efb9743766338ac7bb7fb2399a06fae5e60 48aa74b911fb874986c244712b7fd5b5cc10070b Sebastian Thiel <sebastian.thiel@icloud.com> 1729608402 +0200	reset: moving to 48aa74b911fb874986c244712b7fd5b5cc10070b
3e95c3aff2aaf76be09ece41949213bc261beb22 48aa74b911fb874986c244712b7fd5b5cc10070b Sebastian Thiel <sebastian.thiel@icloud.com> 1729608412 +0200	checkout: moving from merge to main
48aa74b911fb874986c244712b7fd5b5cc10070b 48aa74b911fb874986c244712b7fd5b5cc10070b Sebastian Thiel <sebastian.thiel@icloud.com> 1729608419 +0200	checkout: moving from main to progress-report
d3489cdf35aa38a6df86bb95d4b1e4014b42da94 3e95c3aff2aaf76be09ece41949213bc261beb22 Sebastian Thiel <sebastian.thiel@icloud.com> 1729608451 +0200	checkout: moving from progress-report to merge
3e95c3aff2aaf76be09ece41949213bc261beb22 d3489cdf35aa38a6df86bb95d4b1e4014b42da94 Sebastian Thiel <sebastian.thiel@icloud.com> 1729608500 +0200	checkout: moving from merge to progress-report
0439fc769b282e9475231a9e1c3be2cff46447f4 48aa74b911fb874986c244712b7fd5b5cc10070b Sebastian Thiel <sebastian.thiel@icloud.com> 1729621703 +0200	checkout: moving from progress-report to main
48aa74b911fb874986c244712b7fd5b5cc10070b 435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 Sebastian Thiel <sebastian.thiel@icloud.com> 1729621704 +0200	pull --ff-only: Fast-forward
435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 3f7e8ee2c5107aec009eada1a05af7941da9cb4d Sebastian Thiel <sebastian.thiel@icloud.com> 1729623106 +0200	commit: Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates
3f7e8ee2c5107aec009eada1a05af7941da9cb4d fa3e2600d7e39011f1d7f410249ebd0426a348a8 Sebastian Thiel <sebastian.thiel@icloud.com> 1729623710 +0200	commit: add new changelog for gix-merge
fa3e2600d7e39011f1d7f410249ebd0426a348a8 fa3e2600d7e39011f1d7f410249ebd0426a348a8 Sebastian Thiel <sebastian.thiel@icloud.com> 1729623767 +0200	checkout: moving from main to new-release
fa3e2600d7e39011f1d7f410249ebd0426a348a8 f1364dcb8aa66e3d8730e38445b045c5b63c56e6 Sebastian Thiel <sebastian.thiel@icloud.com> 1729623786 +0200	commit: Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0
f1364dcb8aa66e3d8730e38445b045c5b63c56e6 fa3e2600d7e39011f1d7f410249ebd0426a348a8 Sebastian Thiel <sebastian.thiel@icloud.com> 1729624277 +0200	checkout: moving from new-release to main
fa3e2600d7e39011f1d7f410249ebd0426a348a8 435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 Sebastian Thiel <sebastian.thiel@icloud.com> 1729624287 +0200	reset: moving to origin/main
435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 f1364dcb8aa66e3d8730e38445b045c5b63c56e6 Sebastian Thiel <sebastian.thiel@icloud.com> 1729624291 +0200	checkout: moving from main to new-release
f1364dcb8aa66e3d8730e38445b045c5b63c56e6 435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660383 +0200	checkout: moving from new-release to main
435b30d4021ae9e621af6ac22c6f6e8ed54dabd0 db5c9cfce93713b4b3e249cff1f8cc1ef146f470 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660385 +0200	pull --ff-only: Fast-forward
db5c9cfce93713b4b3e249cff1f8cc1ef146f470 416116145e0712d75d143710c06fcf021ef4bc96 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660451 +0200	rebase (pick): Add initial implementation and tests for `gix-blame`.
416116145e0712d75d143710c06fcf021ef4bc96 1039e99d97c431ba6bc3bebbc04db477581e2ea6 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660464 +0200	rebase (pick): Update meta-data to include `gix-blame` crate
1039e99d97c431ba6bc3bebbc04db477581e2ea6 b9462cf3ff8a4a3e7ced7921e336a834dacc26ec Sebastian Thiel <sebastian.thiel@icloud.com> 1729660539 +0200	rebase (continue): feat: Add `blame` plumbing crate to the top-level.
b9462cf3ff8a4a3e7ced7921e336a834dacc26ec a16129223a4f7d62bd537879997bf73c5fce9a95 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660595 +0200	rebase (continue): feat: add `gix blame` to the CLI
a16129223a4f7d62bd537879997bf73c5fce9a95 c143820d4365474aa619d6327d38673f96b0fa1a Sebastian Thiel <sebastian.thiel@icloud.com> 1729660596 +0200	rebase (pick): Pass blame to more than one parent
c143820d4365474aa619d6327d38673f96b0fa1a abe98ab578bd9d5e2aeabce346e07eb074b76d5a Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Add ignored test for resolved merge conflict
abe98ab578bd9d5e2aeabce346e07eb074b76d5a 5d6d7c7d793633ef697eb00bf620f24fa7c4bc83 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Replace expect by ?
5d6d7c7d793633ef697eb00bf620f24fa7c4bc83 606651d21242f34819d1bf5ff484361eb66f72d4 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Correctly pass blame for some merge commits
606651d21242f34819d1bf5ff484361eb66f72d4 f660a4513abc3c47c619634f04410395f1458ba1 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Adapt to changes in gix-diff
f660a4513abc3c47c619634f04410395f1458ba1 855f12528e2bd661a5f8b4fd521527f11d8a4b07 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Add failing test
855f12528e2bd661a5f8b4fd521527f11d8a4b07 886c0f626b2e39e9c6681fd19d3e905fac7bed26 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660597 +0200	rebase (pick): Add shortcut when oid is identical to parent's
886c0f626b2e39e9c6681fd19d3e905fac7bed26 01f309ebd482904b4622b9db67d4b24b6ed009cf Sebastian Thiel <sebastian.thiel@icloud.com> 1729660598 +0200	rebase (pick): Walk commits in topological order
01f309ebd482904b4622b9db67d4b24b6ed009cf 0db1db3a0a258dba835a8790d46b28c406b73c74 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660598 +0200	rebase (pick): Add shortcut when oid is identical to parent's
0db1db3a0a258dba835a8790d46b28c406b73c74 0db1db3a0a258dba835a8790d46b28c406b73c74 Sebastian Thiel <sebastian.thiel@icloud.com> 1729660598 +0200	rebase (finish): returning to refs/heads/gix-blame
c5e0261b64078f848beb5c3bc6a4e3b06d1f0939 3e95c3aff2aaf76be09ece41949213bc261beb22 Sebastian Thiel <sebastian.thiel@icloud.com> 1729662221 +0200	checkout: moving from gix-blame to merge
3e95c3aff2aaf76be09ece41949213bc261beb22 db5c9cfce93713b4b3e249cff1f8cc1ef146f470 Sebastian Thiel <sebastian.thiel@icloud.com> 1729685054 +0200	checkout: moving from merge to main
db5c9cfce93713b4b3e249cff1f8cc1ef146f470 3e95c3aff2aaf76be09ece41949213bc261beb22 Sebastian Thiel <sebastian.thiel@icloud.com> 1729685058 +0200	checkout: moving from main to merge
48aa74b911fb874986c244712b7fd5b5cc10070b db5c9cfce93713b4b3e249cff1f8cc1ef146f470 Sebastian Thiel <sebastian.thiel@icloud.com> 1729685064 +0200	reset: moving to db5c9cfce93713b4b3e249cff1f8cc1ef146f470
d04464690233d939996d51c68af511233052d605 d04464690233d939996d51c68af511233052d605 Sebastian Thiel <sebastian.thiel@icloud.com> 1729696856 +0200	reset: moving to HEAD
d04464690233d939996d51c68af511233052d605 7af598e814612cb17047229f863afe973abc8cfc Sebastian Thiel <sebastian.thiel@icloud.com> 1729696861 +0200	checkout: moving from merge to add-gix-log
7af598e814612cb17047229f863afe973abc8cfc d04464690233d939996d51c68af511233052d605 Sebastian Thiel <sebastian.thiel@icloud.com> 1729699867 +0200	checkout: moving from add-gix-log to merge
a69c24d87854fa01a2f994be20a98ad365a81d88 a69c24d87854fa01a2f994be20a98ad365a81d88 Sebastian Thiel <sebastian.thiel@icloud.com> 1729799436 +0200	reset: moving to HEAD
b0cd60d4faa3f877bea83629d71cf711c8077b8c b0cd60d4faa3f877bea83629d71cf711c8077b8c Sebastian Thiel <sebastian.thiel@icloud.com> 1729877505 +0200	reset: moving to HEAD
4f092faa794cb9b7712fcfee1e63d7d229fe09cf 4f092faa794cb9b7712fcfee1e63d7d229fe09cf Sebastian Thiel <sebastian.thiel@icloud.com> 1730303755 +0100	reset: moving to HEAD
d9ea38c58ff57fcae982722e9d2dc9aa7d9fe869 d9ea38c58ff57fcae982722e9d2dc9aa7d9fe869 Sebastian Thiel <sebastian.thiel@icloud.com> 1730371186 +0100	reset: moving to HEAD
bd2327c3077b536dfb87507e90d98d2bc874c260 bd2327c3077b536dfb87507e90d98d2bc874c260 Sebastian Thiel <sebastian.thiel@icloud.com> 1730456932 +0100	reset: moving to HEAD
21258863fc56036cc858bc15da7d8b876685427f 21258863fc56036cc858bc15da7d8b876685427f Sebastian Thiel <sebastian.thiel@icloud.com> 1730490523 +0100	reset: moving to HEAD
0e0ef2f74657ac6b64b9557a1c3a903e7faba214 0e0ef2f74657ac6b64b9557a1c3a903e7faba214 Sebastian Thiel <sebastian.thiel@icloud.com> 1730491980 +0100	reset: moving to HEAD
84707c2b7540f9a73cc3f0cde74dabd9822cd809 db5c9cfce93713b4b3e249cff1f8cc1ef146f470 Sebastian Thiel <sebastian.thiel@icloud.com> 1730555684 +0100	checkout: moving from merge to main
db5c9cfce93713b4b3e249cff1f8cc1ef146f470 3fb989be21c739bbfeac93953c1685e7c6cd2106 Sebastian Thiel <sebastian.thiel@icloud.com> 1730555688 +0100	pull --ff-only: Fast-forward
3fb989be21c739bbfeac93953c1685e7c6cd2106 84707c2b7540f9a73cc3f0cde74dabd9822cd809 Sebastian Thiel <sebastian.thiel@icloud.com> 1730555694 +0100	checkout: moving from main to merge
84707c2b7540f9a73cc3f0cde74dabd9822cd809 3fb989be21c739bbfeac93953c1685e7c6cd2106 Sebastian Thiel <sebastian.thiel@icloud.com> 1730555697 +0100	merge main: Fast-forward
3fd4cb2036fd5c6f6d9da2dc83d86084031ce459 3fb989be21c739bbfeac93953c1685e7c6cd2106 Sebastian Thiel <sebastian.thiel@icloud.com> 1730557171 +0100	checkout: moving from merge to main
3fb989be21c739bbfeac93953c1685e7c6cd2106 3fd4cb2036fd5c6f6d9da2dc83d86084031ce459 Sebastian Thiel <sebastian.thiel@icloud.com> 1730557173 +0100	checkout: moving from main to merge
4767eceb019fe66cd219a5531cddf327c883ef16 c5e0261b64078f848beb5c3bc6a4e3b06d1f0939 Sebastian Thiel <sebastian.thiel@icloud.com> 1730639461 +0100	checkout: moving from merge to gix-blame
12e79c99fdd0ade13a38aecb20b6ea1b763fcb75 12e79c99fdd0ade13a38aecb20b6ea1b763fcb75 Sebastian Thiel <sebastian.thiel@icloud.com> 1730817442 +0100	reset: moving to HEAD
fab342c598f0656dc5158a69a6e9826c0df643d5 fab342c598f0656dc5158a69a6e9826c0df643d5 Sebastian Thiel <sebastian.thiel@icloud.com> 1730822316 +0100	reset: moving to HEAD
254793581a135553e555f0bcc815154bb0951324 254793581a135553e555f0bcc815154bb0951324 Sebastian Thiel <sebastian.thiel@icloud.com> 1730822628 +0100	reset: moving to HEAD
8d590f33f49b556de1748818e0bbec610566842f 3fb989be21c739bbfeac93953c1685e7c6cd2106 Sebastian Thiel <sebastian.thiel@icloud.com> 1730833449 +0100	checkout: moving from merge to main
3fb989be21c739bbfeac93953c1685e7c6cd2106 a8765330fc16997dee275866b18a128dec1c5d55 Sebastian Thiel <sebastian.thiel@icloud.com> 1730833453 +0100	pull --ff-only: Fast-forward
a8765330fc16997dee275866b18a128dec1c5d55 8d590f33f49b556de1748818e0bbec610566842f Sebastian Thiel <sebastian.thiel@icloud.com> 1730833471 +0100	checkout: moving from main to merge
8d590f33f49b556de1748818e0bbec610566842f a8765330fc16997dee275866b18a128dec1c5d55 Sebastian Thiel <sebastian.thiel@icloud.com> 1730833480 +0100	merge main: Fast-forward
4079519e7e292ee193248e3acea6587788c6b884 a8765330fc16997dee275866b18a128dec1c5d55 Sebastian Thiel <sebastian.thiel@icloud.com> 1730882722 +0100	checkout: moving from merge to main
a8765330fc16997dee275866b18a128dec1c5d55 697a6320c7664845590e3e8251015085b6cc5d81 Sebastian Thiel <sebastian.thiel@icloud.com> 1730882725 +0100	pull --ff-only: Fast-forward
697a6320c7664845590e3e8251015085b6cc5d81 4079519e7e292ee193248e3acea6587788c6b884 Sebastian Thiel <sebastian.thiel@icloud.com> 1730900266 +0100	checkout: moving from main to merge
4079519e7e292ee193248e3acea6587788c6b884 697a6320c7664845590e3e8251015085b6cc5d81 Sebastian Thiel <sebastian.thiel@icloud.com> 1730900268 +0100	merge main: Fast-forward
697a6320c7664845590e3e8251015085b6cc5d81 697a6320c7664845590e3e8251015085b6cc5d81 Sebastian Thiel <sebastian.thiel@icloud.com> 1730900321 +0100	checkout: moving from merge to hasconfig
d51aec95588fee219dee62438d26e4574d38a497 697a6320c7664845590e3e8251015085b6cc5d81 Sebastian Thiel <sebastian.thiel@icloud.com> 1730962199 +0100	checkout: moving from hasconfig to main
697a6320c7664845590e3e8251015085b6cc5d81 c5955fc4ad1064c7e4b4c57de32a661e693fbe49 Sebastian Thiel <sebastian.thiel@icloud.com> 1730962201 +0100	pull --ff-only: Fast-forward
c5955fc4ad1064c7e4b4c57de32a661e693fbe49 697a6320c7664845590e3e8251015085b6cc5d81 Sebastian Thiel <sebastian.thiel@icloud.com> 1730973938 +0100	checkout: moving from main to merge
697a6320c7664845590e3e8251015085b6cc5d81 c5955fc4ad1064c7e4b4c57de32a661e693fbe49 Sebastian Thiel <sebastian.thiel@icloud.com> 1730973945 +0100	merge main: Fast-forward
4a5afc7524fc96213385454079ebf9baf302ad4b c5955fc4ad1064c7e4b4c57de32a661e693fbe49 Sebastian Thiel <sebastian.thiel@icloud.com> 1731002416 +0100	checkout: moving from merge to main
c5955fc4ad1064c7e4b4c57de32a661e693fbe49 905e5b42a6163f92edef8fab82d97aeb6f17cf06 Sebastian Thiel <sebastian.thiel@icloud.com> 1731002417 +0100	pull --ff-only: Fast-forward
905e5b42a6163f92edef8fab82d97aeb6f17cf06 4a5afc7524fc96213385454079ebf9baf302ad4b Sebastian Thiel <sebastian.thiel@icloud.com> 1731002419 +0100	checkout: moving from main to merge
4a5afc7524fc96213385454079ebf9baf302ad4b 905e5b42a6163f92edef8fab82d97aeb6f17cf06 Sebastian Thiel <sebastian.thiel@icloud.com> 1731002422 +0100	merge main: Fast-forward
2ac56732b59aced2ebb58d730b1c8fed458cea7f 2ac56732b59aced2ebb58d730b1c8fed458cea7f Sebastian Thiel <sebastian.thiel@icloud.com> 1731008700 +0100	reset: moving to HEAD
65ae68eac6b77d12ca804927090da5bb80551eae 905e5b42a6163f92edef8fab82d97aeb6f17cf06 Sebastian Thiel <sebastian.thiel@icloud.com> 1731010935 +0100	checkout: moving from merge to main
905e5b42a6163f92edef8fab82d97aeb6f17cf06 cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731010940 +0100	pull --ff-only: Fast-forward
cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 65ae68eac6b77d12ca804927090da5bb80551eae Sebastian Thiel <sebastian.thiel@icloud.com> 1731010953 +0100	checkout: moving from main to merge
65ae68eac6b77d12ca804927090da5bb80551eae cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731010954 +0100	merge main: Fast-forward
cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731010958 +0100	checkout: moving from merge to main
cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731068079 +0100	checkout: moving from main to merge
72202f95d1e6bb1ee3fe091d00d183da6183fc1b cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731135990 +0100	checkout: moving from merge to main
cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 7a406481b072728cec089d7c05364f9dbba335a2 Sebastian Thiel <sebastian.thiel@icloud.com> 1731135993 +0100	pull --ff-only: Fast-forward
7a406481b072728cec089d7c05364f9dbba335a2 72202f95d1e6bb1ee3fe091d00d183da6183fc1b Sebastian Thiel <sebastian.thiel@icloud.com> 1731135995 +0100	checkout: moving from main to merge
cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5 7a406481b072728cec089d7c05364f9dbba335a2 Sebastian Thiel <sebastian.thiel@icloud.com> 1731135999 +0100	reset: moving to 7a406481b072728cec089d7c05364f9dbba335a2
5f7d26e8a4c4b1639eb35921e6f517ee48dc50e2 4564a641e26146d3b908c2d44af74991d3c25e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1731395387 +0100	checkout: moving from merge to run-ci/gha-permissions
5173e9a2d8464beb7b18de56ac31c04491108381 132696dce95ce8d79e279978d82f9f038f41a9a4 Sebastian Thiel <sebastian.thiel@icloud.com> 1731407531 +0100	commit: Also clear the target before running journey tests.
132696dce95ce8d79e279978d82f9f038f41a9a4 7a406481b072728cec089d7c05364f9dbba335a2 Sebastian Thiel <sebastian.thiel@icloud.com> 1731410436 +0100	checkout: moving from run-ci/gha-permissions to main
7a406481b072728cec089d7c05364f9dbba335a2 d47263bed4ac38d175fcc206f2df5d711dc633ac Sebastian Thiel <sebastian.thiel@icloud.com> 1731410438 +0100	pull --ff-only: Fast-forward
d47263bed4ac38d175fcc206f2df5d711dc633ac 5f7d26e8a4c4b1639eb35921e6f517ee48dc50e2 Sebastian Thiel <sebastian.thiel@icloud.com> 1731410439 +0100	checkout: moving from main to merge
7a406481b072728cec089d7c05364f9dbba335a2 d47263bed4ac38d175fcc206f2df5d711dc633ac Sebastian Thiel <sebastian.thiel@icloud.com> 1731410442 +0100	reset: moving to d47263bed4ac38d175fcc206f2df5d711dc633ac
9ac0a04eda9330f7367344d5ec3e51cd03d95bf8 d47263bed4ac38d175fcc206f2df5d711dc633ac Sebastian Thiel <sebastian.thiel@icloud.com> 1731652477 +0100	checkout: moving from merge to main
d47263bed4ac38d175fcc206f2df5d711dc633ac 66c222c255b05ef8ff9b43609dcbf8b4ca00e01a Sebastian Thiel <sebastian.thiel@icloud.com> 1731652479 +0100	pull --ff-only: Fast-forward
66c222c255b05ef8ff9b43609dcbf8b4ca00e01a 66c222c255b05ef8ff9b43609dcbf8b4ca00e01a Sebastian Thiel <sebastian.thiel@icloud.com> 1731652495 +0100	checkout: moving from main to fix-1678
dc3d8bf79e90733172a2c3796995cdfbed438355 66c222c255b05ef8ff9b43609dcbf8b4ca00e01a Sebastian Thiel <sebastian.thiel@icloud.com> 1731673455 +0100	checkout: moving from fix-1678 to main
66c222c255b05ef8ff9b43609dcbf8b4ca00e01a 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b Sebastian Thiel <sebastian.thiel@icloud.com> 1731673457 +0100	pull --ff-only: Fast-forward
275a0c55ac074e5a1004c188b87f8fc8aa9adc5b 9ac0a04eda9330f7367344d5ec3e51cd03d95bf8 Sebastian Thiel <sebastian.thiel@icloud.com> 1731673471 +0100	checkout: moving from main to merge
d47263bed4ac38d175fcc206f2df5d711dc633ac 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b Sebastian Thiel <sebastian.thiel@icloud.com> 1731673478 +0100	reset: moving to 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b
d4b718baedda615e600c55894ff4c103955fa632 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b Sebastian Thiel <sebastian.thiel@icloud.com> 1731826630 +0100	checkout: moving from merge to main
275a0c55ac074e5a1004c188b87f8fc8aa9adc5b 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b Sebastian Thiel <sebastian.thiel@icloud.com> 1731826637 +0100	checkout: moving from main to max-purer
275a0c55ac074e5a1004c188b87f8fc8aa9adc5b d4b718baedda615e600c55894ff4c103955fa632 Sebastian Thiel <sebastian.thiel@icloud.com> 1731826703 +0100	checkout: moving from max-purer to merge
d4b718baedda615e600c55894ff4c103955fa632 275a0c55ac074e5a1004c188b87f8fc8aa9adc5b Sebastian Thiel <sebastian.thiel@icloud.com> 1731851953 +0100	checkout: moving from merge to main
275a0c55ac074e5a1004c188b87f8fc8aa9adc5b 906acd3625e98cbf2c38cc3678ad81a57a58b33e Sebastian Thiel <sebastian.thiel@icloud.com> 1731851955 +0100	pull --ff-only: Fast-forward
906acd3625e98cbf2c38cc3678ad81a57a58b33e 877f4d2091a24d691f2c88a5841a6e4eb357aca3 Sebastian Thiel <sebastian.thiel@icloud.com> 1731851961 +0100	checkout: moving from main to fixes
877f4d2091a24d691f2c88a5841a6e4eb357aca3 906acd3625e98cbf2c38cc3678ad81a57a58b33e Sebastian Thiel <sebastian.thiel@icloud.com> 1731851977 +0100	reset: moving to 906acd3625e98cbf2c38cc3678ad81a57a58b33e
88d9d4387287b7540a0f42b26c6a4adb4cd769a9 906acd3625e98cbf2c38cc3678ad81a57a58b33e Sebastian Thiel <sebastian.thiel@icloud.com> 1731861900 +0100	checkout: moving from fixes to main
906acd3625e98cbf2c38cc3678ad81a57a58b33e 9ab86a23d45941c4f0a3239e0cb57d4161dd279c Sebastian Thiel <sebastian.thiel@icloud.com> 1731861902 +0100	pull --ff-only: Fast-forward
9ab86a23d45941c4f0a3239e0cb57d4161dd279c d4b718baedda615e600c55894ff4c103955fa632 Sebastian Thiel <sebastian.thiel@icloud.com> 1731867373 +0100	checkout: moving from main to merge
f4b679d989c3ded95e415731d6f0b08e54315b8b 1c4a910518a749ff53a38f1719a302659dd96f7d Sebastian Thiel <sebastian.thiel@icloud.com> 1731925651 +0100	checkout: moving from merge to move-lookup-entry-to-gix-object
1c4a910518a749ff53a38f1719a302659dd96f7d f4b679d989c3ded95e415731d6f0b08e54315b8b Sebastian Thiel <sebastian.thiel@icloud.com> 1731933071 +0100	checkout: moving from move-lookup-entry-to-gix-object to merge
acb069a476281dacac8ed5a9abaf948dce92ad98 9ab86a23d45941c4f0a3239e0cb57d4161dd279c Sebastian Thiel <sebastian.thiel@icloud.com> 1731997636 +0100	checkout: moving from merge to main
9ab86a23d45941c4f0a3239e0cb57d4161dd279c 700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 Sebastian Thiel <sebastian.thiel@icloud.com> 1731997637 +0100	pull --ff-only: Fast-forward
700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 acb069a476281dacac8ed5a9abaf948dce92ad98 Sebastian Thiel <sebastian.thiel@icloud.com> 1732010020 +0100	checkout: moving from main to merge
10ae556eb91e341bca21c5595bb076324c8bfdbf 700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 Sebastian Thiel <sebastian.thiel@icloud.com> 1732090392 +0100	checkout: moving from merge to main
700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 Sebastian Thiel <sebastian.thiel@icloud.com> 1732090396 +0100	checkout: moving from main to report
af447c046932779d9912f3eea7381b75531e2fbd 7e0974f90bb9a89065e90dcb30dd42b52246a3fe Sebastian Thiel <sebastian.thiel@icloud.com> 1732107837 +0100	reset: moving to origin/report
a92ec00758c69eae0df5732242c9d7838717b764 10ae556eb91e341bca21c5595bb076324c8bfdbf Sebastian Thiel <sebastian.thiel@icloud.com> 1732117363 +0100	checkout: moving from report to merge
665e54848b8cff8e11e1b0f4779896d5f4d2a865 a92ec00758c69eae0df5732242c9d7838717b764 Sebastian Thiel <sebastian.thiel@icloud.com> 1732175669 +0100	checkout: moving from merge to report
ac3143632f0c0d14374b721ad1b53c98dfd8ae90 b29405fe9147a3a366c4048fbe295ea04de40fa6 Sebastian Thiel <sebastian.thiel@icloud.com> 1732184844 +0100	checkout: moving from report to move-lookup-entry-to-gix-object
b29405fe9147a3a366c4048fbe295ea04de40fa6 665e54848b8cff8e11e1b0f4779896d5f4d2a865 Sebastian Thiel <sebastian.thiel@icloud.com> 1732208119 +0100	checkout: moving from move-lookup-entry-to-gix-object to merge
665e54848b8cff8e11e1b0f4779896d5f4d2a865 ac3143632f0c0d14374b721ad1b53c98dfd8ae90 Sebastian Thiel <sebastian.thiel@icloud.com> 1732211573 +0100	checkout: moving from merge to report
ac3143632f0c0d14374b721ad1b53c98dfd8ae90 700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 Sebastian Thiel <sebastian.thiel@icloud.com> 1732211578 +0100	checkout: moving from report to main
700cfa52e7f3008036881a99fbdeb04c9ab1f2f5 9738191145f889a529917fb4b7d1a645c58a6636 Sebastian Thiel <sebastian.thiel@icloud.com> 1732211579 +0100	pull --ff-only: Fast-forward
9738191145f889a529917fb4b7d1a645c58a6636 197d31aff5a602c3107c32661340e89781ad0b33 Sebastian Thiel <sebastian.thiel@icloud.com> 1732211750 +0100	merge report: Merge made by the 'ort' strategy.
197d31aff5a602c3107c32661340e89781ad0b33 665e54848b8cff8e11e1b0f4779896d5f4d2a865 Sebastian Thiel <sebastian.thiel@icloud.com> 1732264014 +0100	checkout: moving from main to merge
275a0c55ac074e5a1004c188b87f8fc8aa9adc5b 197d31aff5a602c3107c32661340e89781ad0b33 Sebastian Thiel <sebastian.thiel@icloud.com> 1732264028 +0100	reset: moving to 197d31aff5a602c3107c32661340e89781ad0b33
71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 197d31aff5a602c3107c32661340e89781ad0b33 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437773 +0100	checkout: moving from merge to main
197d31aff5a602c3107c32661340e89781ad0b33 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437774 +0100	pull --ff-only: Fast-forward
0b7abfbdebe8c5ab30b89499a70dd7727de41184 25a303edcd07bd21c4875dce21fd2148d9c10c05 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437787 +0100	checkout: moving from main to merge
71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437879 +0100	checkout: moving from merge to main
0b7abfbdebe8c5ab30b89499a70dd7727de41184 71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437888 +0100	checkout: moving from main to merge
71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437893 +0100	merge main: Fast-forward
0b7abfbdebe8c5ab30b89499a70dd7727de41184 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437897 +0100	checkout: moving from merge to main
0b7abfbdebe8c5ab30b89499a70dd7727de41184 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437904 +0100	checkout: moving from main to merge
0b7abfbdebe8c5ab30b89499a70dd7727de41184 cb3149fec63dc9e366baf0399040d20161616b22 Sebastian Thiel <sebastian.thiel@icloud.com> 1732437982 +0100	reset: moving to @~1
cb3149fec63dc9e366baf0399040d20161616b22 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732438006 +0100	reset: moving to main
0b7abfbdebe8c5ab30b89499a70dd7727de41184 71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 Sebastian Thiel <sebastian.thiel@icloud.com> 1732438015 +0100	reset: moving to @^2
71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732438023 +0100	checkout: moving from merge to main
0b7abfbdebe8c5ab30b89499a70dd7727de41184 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732438037 +0100	checkout: moving from main to reduce-memory
664e28caa4304fe489d2b37a1a3328763960517f 0b7abfbdebe8c5ab30b89499a70dd7727de41184 Sebastian Thiel <sebastian.thiel@icloud.com> 1732440459 +0100	checkout: moving from reduce-memory to main
0b7abfbdebe8c5ab30b89499a70dd7727de41184 54ea266a5b57d3081c2ba6ed60dc0612059617ca Sebastian Thiel <sebastian.thiel@icloud.com> 1732440461 +0100	pull --ff-only: Fast-forward
54ea266a5b57d3081c2ba6ed60dc0612059617ca bc9d9943e8499a76fc47a05b63ac5c684187d1ae Sebastian Thiel <sebastian.thiel@icloud.com> 1732441044 +0100	commit: prepare changelogs prior to release
bc9d9943e8499a76fc47a05b63ac5c684187d1ae 8ce49129a75e21346ceedf7d5f87fa3a34b024e1 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441112 +0100	commit: Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates
8ce49129a75e21346ceedf7d5f87fa3a34b024e1 8ce49129a75e21346ceedf7d5f87fa3a34b024e1 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441412 +0100	checkout: moving from main to release
8ce49129a75e21346ceedf7d5f87fa3a34b024e1 8ce49129a75e21346ceedf7d5f87fa3a34b024e1 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441416 +0100	checkout: moving from release to main
8ce49129a75e21346ceedf7d5f87fa3a34b024e1 54ea266a5b57d3081c2ba6ed60dc0612059617ca Sebastian Thiel <sebastian.thiel@icloud.com> 1732441420 +0100	reset: moving to origin/main
54ea266a5b57d3081c2ba6ed60dc0612059617ca 8ce49129a75e21346ceedf7d5f87fa3a34b024e1 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441424 +0100	checkout: moving from main to release
8ce49129a75e21346ceedf7d5f87fa3a34b024e1 4145d2a4c385931731e69c793864ec9b4fd4b87f Sebastian Thiel <sebastian.thiel@icloud.com> 1732441529 +0100	commit: fix gix-path version (which fails publishing due to the patch-level mismatch)
4145d2a4c385931731e69c793864ec9b4fd4b87f 4d82a197ce39692ea2fc6d6ea56ed4e8dc0f87f0 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441555 +0100	commit: Adjusting changelogs prior to release of gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0
4d82a197ce39692ea2fc6d6ea56ed4e8dc0f87f0 7652fb079f92e81dd1be580144f5040f63d3b694 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441848 +0100	commit: prepare changelogs once more
7652fb079f92e81dd1be580144f5040f63d3b694 4145d2a4c385931731e69c793864ec9b4fd4b87f Sebastian Thiel <sebastian.thiel@icloud.com> 1732441882 +0100	rebase (start): checkout refs/remotes/origin/release
4145d2a4c385931731e69c793864ec9b4fd4b87f 9d627bbc27322285e8d2ac3c5135ce425ad76838 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441958 +0100	rebase (continue): prepare changelogs once more
9d627bbc27322285e8d2ac3c5135ce425ad76838 9d627bbc27322285e8d2ac3c5135ce425ad76838 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441959 +0100	rebase (finish): returning to refs/heads/release
9d627bbc27322285e8d2ac3c5135ce425ad76838 4000197ecc8cf1a5d79361620e4c114f86476703 Sebastian Thiel <sebastian.thiel@icloud.com> 1732441984 +0100	commit: Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0
4000197ecc8cf1a5d79361620e4c114f86476703 54ea266a5b57d3081c2ba6ed60dc0612059617ca Sebastian Thiel <sebastian.thiel@icloud.com> 1732455127 +0100	checkout: moving from release to main
54ea266a5b57d3081c2ba6ed60dc0612059617ca e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732455130 +0100	pull --ff-only: Fast-forward
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e 4000197ecc8cf1a5d79361620e4c114f86476703 Sebastian Thiel <sebastian.thiel@icloud.com> 1732455134 +0100	checkout: moving from main to release
4000197ecc8cf1a5d79361620e4c114f86476703 e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732455138 +0100	checkout: moving from release to main
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e dc5ea566347bb19c6ea6dc6fa757635667875e93 Sebastian Thiel <sebastian.thiel@icloud.com> 1732455153 +0100	checkout: moving from main to move-lookup-entry-to-gix-object
71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f 153297304cf6d49427ab93cf3eb2956086eac66a Sebastian Thiel <sebastian.thiel@icloud.com> 1732458133 +0100	pull --ff-only: Fast-forward
dc5ea566347bb19c6ea6dc6fa757635667875e93 3cd11ed950aabd77c9f862980032097bbc3de096 Sebastian Thiel <sebastian.thiel@icloud.com> 1732458162 +0100	reset: moving to @~1
3cd11ed950aabd77c9f862980032097bbc3de096 3cd11ed950aabd77c9f862980032097bbc3de096 Sebastian Thiel <sebastian.thiel@icloud.com> 1732458185 +0100	reset: moving to 3cd11ed95
f1cb1ff7c7da3c16a25992292b0fbbf593b90449 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458203 +0100	reset: moving to 71e9282
71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732458213 +0100	checkout: moving from move-lookup-entry-to-gix-object to main
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458221 +0100	checkout: moving from main to move-lookup-entry-to-gix-object
3cd11ed950aabd77c9f862980032097bbc3de096 e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732458251 +0100	reset: moving to e8b3b41dd79b8f4567670b1f89dd8867b6134e9e
22e695d9bd82a365ecc112b714002734f1d4c145 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458262 +0100	reset: moving to 71e9282
dc5ea566347bb19c6ea6dc6fa757635667875e93 e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732458274 +0100	reset: moving to e8b3b41dd79b8f4567670b1f89dd8867b6134e9e
2a07233ded2be2034b70c6c24934e7e436eb563d 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458281 +0100	reset: moving to 71e9282
2a07233ded2be2034b70c6c24934e7e436eb563d 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458313 +0100	reset: moving to 71e9282
71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732458365 +0100	checkout: moving from move-lookup-entry-to-gix-object to main
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458384 +0100	checkout: moving from main to move-lookup-entry-to-gix-object
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e 71e928248452d3cbe3ee4e3e1c58d61f1a3f3b5f Sebastian Thiel <sebastian.thiel@icloud.com> 1732458433 +0100	reset: moving to 71e9282
9ab86a23d45941c4f0a3239e0cb57d4161dd279c e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732458463 +0100	reset: moving to e8b3b41dd79b8f4567670b1f89dd8867b6134e9e
d7f49916037efb0c95cf1a4d58be215bee67eb0d e8b3b41dd79b8f4567670b1f89dd8867b6134e9e Sebastian Thiel <sebastian.thiel@icloud.com> 1732516874 +0100	checkout: moving from move-lookup-entry-to-gix-object to main
e8b3b41dd79b8f4567670b1f89dd8867b6134e9e 39227a90ca4590b08dc7d782728be2e9a3054618 Sebastian Thiel <sebastian.thiel@icloud.com> 1732516875 +0100	pull --ff-only: Fast-forward
39227a90ca4590b08dc7d782728be2e9a3054618 d7f49916037efb0c95cf1a4d58be215bee67eb0d Sebastian Thiel <sebastian.thiel@icloud.com> 1732516878 +0100	checkout: moving from main to move-lookup-entry-to-gix-object
d7f49916037efb0c95cf1a4d58be215bee67eb0d 39227a90ca4590b08dc7d782728be2e9a3054618 Sebastian Thiel <sebastian.thiel@icloud.com> 1732516885 +0100	checkout: moving from move-lookup-entry-to-gix-object to main
39227a90ca4590b08dc7d782728be2e9a3054618 3082d4076e3ee200ecb5c3da7ae3f940c5a29e8b Sebastian Thiel <sebastian.thiel@icloud.com> 1732516898 +0100	checkout: moving from main to run-ci/duration-units
a769fddbcf4b0513b263c312b6ccc3b583307601 39227a90ca4590b08dc7d782728be2e9a3054618 Sebastian Thiel <sebastian.thiel@icloud.com> 1732519886 +0100	checkout: moving from run-ci/duration-units to main
39227a90ca4590b08dc7d782728be2e9a3054618 39227a90ca4590b08dc7d782728be2e9a3054618 Sebastian Thiel <sebastian.thiel@icloud.com> 1732521084 +0100	checkout: moving from main to fix-1703
39227a90ca4590b08dc7d782728be2e9a3054618 0727b5679f9ddeb05a9a50c895b6d77ba61ed544 Sebastian Thiel <sebastian.thiel@icloud.com> 1732521094 +0100	commit: fix: `gix merge file` now uses `THEIRS` instead of `OURS` where needed (#1703)
0727b5679f9ddeb05a9a50c895b6d77ba61ed544 39227a90ca4590b08dc7d782728be2e9a3054618 Sebastian Thiel <sebastian.thiel@icloud.com> 1732523223 +0100	checkout: moving from fix-1703 to main
39227a90ca4590b08dc7d782728be2e9a3054618 b34d14e83e546cbe423b12c63d5d80b3fedc42d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1732523224 +0100	pull --ff-only: Fast-forward
b34d14e83e546cbe423b12c63d5d80b3fedc42d2 71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 Sebastian Thiel <sebastian.thiel@icloud.com> 1732548162 +0100	checkout: moving from main to merge
71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0 b34d14e83e546cbe423b12c63d5d80b3fedc42d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1732548179 +0100	merge main: Fast-forward
2d918997ab01dbe86392325012c9c519547fe69b b34d14e83e546cbe423b12c63d5d80b3fedc42d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1732549043 +0100	checkout: moving from merge to main
b34d14e83e546cbe423b12c63d5d80b3fedc42d2 b34d14e83e546cbe423b12c63d5d80b3fedc42d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1732549053 +0100	checkout: moving from main to fix-ci
b81b44e4649fcbe5ad28706076d833579f1f6da2 2d918997ab01dbe86392325012c9c519547fe69b Sebastian Thiel <sebastian.thiel@icloud.com> 1732549670 +0100	checkout: moving from fix-ci to merge
0b78aeb602faf3c465cb59dcc246c4cd5339e8f8 b34d14e83e546cbe423b12c63d5d80b3fedc42d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1732551756 +0100	checkout: moving from merge to main
b34d14e83e546cbe423b12c63d5d80b3fedc42d2 1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce Sebastian Thiel <sebastian.thiel@icloud.com> 1732551758 +0100	pull --ff-only: Fast-forward
1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce 0b78aeb602faf3c465cb59dcc246c4cd5339e8f8 Sebastian Thiel <sebastian.thiel@icloud.com> 1732551759 +0100	checkout: moving from main to merge
b34d14e83e546cbe423b12c63d5d80b3fedc42d2 1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce Sebastian Thiel <sebastian.thiel@icloud.com> 1732551762 +0100	reset: moving to 1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce
337952d09ad6d8d4fc85b0c248d0b4803596d3dd 5a803b34b5797fc3e4f290a22ec9828d4199d927 Sebastian Thiel <sebastian.thiel@icloud.com> 1732884112 +0100	checkout: moving from merge to run-ci/mode-it
5a803b34b5797fc3e4f290a22ec9828d4199d927 7e8aedff9a05a84038f885e1a17ef5cc41d9fe2e Sebastian Thiel <sebastian.thiel@icloud.com> 1732884813 +0100	commit: minor refactors
7e8aedff9a05a84038f885e1a17ef5cc41d9fe2e 1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce Sebastian Thiel <sebastian.thiel@icloud.com> 1732895828 +0100	checkout: moving from run-ci/mode-it to main
1435193c5fa1d08a8ff0e588b7f3dffdbd80b0ce c146b7af0469f4925c96a33a2192eba19a062dbe Sebastian Thiel <sebastian.thiel@icloud.com> 1732895829 +0100	pull --ff-only: Fast-forward
c146b7af0469f4925c96a33a2192eba19a062dbe 337952d09ad6d8d4fc85b0c248d0b4803596d3dd Sebastian Thiel <sebastian.thiel@icloud.com> 1732895830 +0100	checkout: moving from main to merge
ffc05ab590b33886b6ee8c951d16ea72c59fcebd c146b7af0469f4925c96a33a2192eba19a062dbe Sebastian Thiel <sebastian.thiel@icloud.com> 1732951450 +0100	checkout: moving from merge to main
c146b7af0469f4925c96a33a2192eba19a062dbe c146b7af0469f4925c96a33a2192eba19a062dbe Sebastian Thiel <sebastian.thiel@icloud.com> 1732951456 +0100	checkout: moving from main to msrv-update
c146b7af0469f4925c96a33a2192eba19a062dbe ffc05ab590b33886b6ee8c951d16ea72c59fcebd Sebastian Thiel <sebastian.thiel@icloud.com> 1732952326 +0100	checkout: moving from msrv-update to merge
1dedb1b17bd704260a30e64df2ce7c9b6d555667 1dedb1b17bd704260a30e64df2ce7c9b6d555667 Sebastian Thiel <sebastian.thiel@icloud.com> 1733064355 +0100	reset: moving to HEAD
70fe9a856967d1cb7bbf43107dde807108b62418 1f6a8669a64b15fbe7021c6906f88f5b7c7c142e Sebastian Thiel <sebastian.thiel@icloud.com> 1733075775 +0100	checkout: moving from merge to run-ci/git-bash
1f6a8669a64b15fbe7021c6906f88f5b7c7c142e 70fe9a856967d1cb7bbf43107dde807108b62418 Sebastian Thiel <sebastian.thiel@icloud.com> 1733076178 +0100	checkout: moving from run-ci/git-bash to merge
70fe9a856967d1cb7bbf43107dde807108b62418 c146b7af0469f4925c96a33a2192eba19a062dbe Sebastian Thiel <sebastian.thiel@icloud.com> 1733076182 +0100	checkout: moving from merge to main
c146b7af0469f4925c96a33a2192eba19a062dbe fadf106c735837c627f072ee37a9f7587f987bf2 Sebastian Thiel <sebastian.thiel@icloud.com> 1733076185 +0100	pull --ff-only: Fast-forward
fadf106c735837c627f072ee37a9f7587f987bf2 70fe9a856967d1cb7bbf43107dde807108b62418 Sebastian Thiel <sebastian.thiel@icloud.com> 1733076185 +0100	checkout: moving from main to merge
c5c651205c5c31685194c7fe98a4051c42223a4a c5c651205c5c31685194c7fe98a4051c42223a4a Sebastian Thiel <sebastian.thiel@icloud.com> 1733128512 +0100	reset: moving to HEAD
d281ba6b180d052aa6df43f43696a84819218d4d d281ba6b180d052aa6df43f43696a84819218d4d Sebastian Thiel <sebastian.thiel@icloud.com> 1733426826 +0100	reset: moving to HEAD
e487cca78d8e6c5b51d2614daf05c98e1469ee69 e487cca78d8e6c5b51d2614daf05c98e1469ee69 Sebastian Thiel <sebastian.thiel@icloud.com> 1733426857 +0100	reset: moving to HEAD
1ec12801d52a24218c9b77782cdcbdcb43a34a13 1ec12801d52a24218c9b77782cdcbdcb43a34a13 Sebastian Thiel <sebastian.thiel@icloud.com> 1733670508 +0100	reset: moving to HEAD
960773e5526d02e1f2294224859c821ed86a3463 fadf106c735837c627f072ee37a9f7587f987bf2 Sebastian Thiel <sebastian.thiel@icloud.com> 1733685673 +0100	checkout: moving from merge to main
fadf106c735837c627f072ee37a9f7587f987bf2 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733685674 +0100	pull --ff-only: Fast-forward
520c832cfcfb34eb7617be55ebe2719ab35595fd 29e3bbf128939d78178500450fd086b5b91691ff Sebastian Thiel <sebastian.thiel@icloud.com> 1733752480 +0100	checkout: moving from main to traverse-topo-builder-enhancements
29e3bbf128939d78178500450fd086b5b91691ff 55eaf52395a179e537f5e3a78d7871247898539c Sebastian Thiel <sebastian.thiel@icloud.com> 1733752833 +0100	commit: minor refactor
55eaf52395a179e537f5e3a78d7871247898539c 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733753584 +0100	checkout: moving from traverse-topo-builder-enhancements to main
520c832cfcfb34eb7617be55ebe2719ab35595fd c7d477dc6f37d30bc5f0871081b08f7931c43ffa Sebastian Thiel <sebastian.thiel@icloud.com> 1733753593 +0100	checkout: moving from main to remove-delegates
bcdce6e873904e4dd77070d7b4e75f969b9f0bea 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733753889 +0100	reset: moving to 520c832cfcfb34eb7617be55ebe2719ab35595fd
ad3c6aeb4d35f7556753cb9b92ad1242200d8ce8 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733759273 +0100	checkout: moving from remove-delegates to main
520c832cfcfb34eb7617be55ebe2719ab35595fd 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733759277 +0100	checkout: moving from main to fix-ide
97396200f85ba691396c80d78d3bd7efc0707f8c ad3c6aeb4d35f7556753cb9b92ad1242200d8ce8 Sebastian Thiel <sebastian.thiel@icloud.com> 1733759373 +0100	checkout: moving from fix-ide to remove-delegates
520c832cfcfb34eb7617be55ebe2719ab35595fd 97396200f85ba691396c80d78d3bd7efc0707f8c Sebastian Thiel <sebastian.thiel@icloud.com> 1733759378 +0100	reset: moving to 97396200f85ba691396c80d78d3bd7efc0707f8c
437be4f487302c9e9cd09c258a2641cb421e9bc4 520c832cfcfb34eb7617be55ebe2719ab35595fd Sebastian Thiel <sebastian.thiel@icloud.com> 1733772873 +0100	checkout: moving from remove-delegates to main
520c832cfcfb34eb7617be55ebe2719ab35595fd 06ef1e97cc93f21810d9b86b124f70929acdba4f Sebastian Thiel <sebastian.thiel@icloud.com> 1733772875 +0100	pull --ff-only: Fast-forward
06ef1e97cc93f21810d9b86b124f70929acdba4f 06ef1e97cc93f21810d9b86b124f70929acdba4f Sebastian Thiel <sebastian.thiel@icloud.com> 1733772885 +0100	checkout: moving from main to with-shell-choice
838420ffa240c9953fdf6871065cb41e3028ca89 06ef1e97cc93f21810d9b86b124f70929acdba4f Sebastian Thiel <sebastian.thiel@icloud.com> 1733822869 +0100	checkout: moving from with-shell-choice to main
06ef1e97cc93f21810d9b86b124f70929acdba4f 801f9e916a066bd222c2174033dacabf44f2d0b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1733822881 +0100	pull --ff-only: Fast-forward
801f9e916a066bd222c2174033dacabf44f2d0b8 437be4f487302c9e9cd09c258a2641cb421e9bc4 Sebastian Thiel <sebastian.thiel@icloud.com> 1733822884 +0100	checkout: moving from main to remove-delegates
1cc6188100027709e26758909696aae09a67b269 1cc6188100027709e26758909696aae09a67b269 Sebastian Thiel <sebastian.thiel@icloud.com> 1733841884 +0100	reset: moving to HEAD
97396200f85ba691396c80d78d3bd7efc0707f8c 801f9e916a066bd222c2174033dacabf44f2d0b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1733841887 +0100	reset: moving to 801f9e916a066bd222c2174033dacabf44f2d0b8
b03eb83cda9fccacc3d0812cd8e3d8e51b39452d 801f9e916a066bd222c2174033dacabf44f2d0b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1734284996 +0100	checkout: moving from remove-delegates to main
801f9e916a066bd222c2174033dacabf44f2d0b8 b03eb83cda9fccacc3d0812cd8e3d8e51b39452d Sebastian Thiel <sebastian.thiel@icloud.com> 1734285061 +0100	checkout: moving from main to remove-delegates
a0d9f51113febefd7926214d9086a9429ac10617 801f9e916a066bd222c2174033dacabf44f2d0b8 Sebastian Thiel <sebastian.thiel@icloud.com> 1734331308 +0100	checkout: moving from remove-delegates to main
801f9e916a066bd222c2174033dacabf44f2d0b8 cd9060aa3cb5b5e02673b55c2b33bef5674b148c Sebastian Thiel <sebastian.thiel@icloud.com> 1734331311 +0100	pull --ff-only: Fast-forward
cd9060aa3cb5b5e02673b55c2b33bef5674b148c cd9060aa3cb5b5e02673b55c2b33bef5674b148c Sebastian Thiel <sebastian.thiel@icloud.com> 1734331325 +0100	checkout: moving from main to gix-command-api
c67770ff118f00846936117fccc2c59ca3220f6e cd9060aa3cb5b5e02673b55c2b33bef5674b148c Sebastian Thiel <sebastian.thiel@icloud.com> 1734333771 +0100	checkout: moving from gix-command-api to main
cd9060aa3cb5b5e02673b55c2b33bef5674b148c faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734333773 +0100	pull --ff-only: Fast-forward
faa0cdeb35a8135ff9513a1c9884126f6b080f4a a0d9f51113febefd7926214d9086a9429ac10617 Sebastian Thiel <sebastian.thiel@icloud.com> 1734336648 +0100	checkout: moving from main to remove-delegates
a0d9f51113febefd7926214d9086a9429ac10617 c67770ff118f00846936117fccc2c59ca3220f6e Sebastian Thiel <sebastian.thiel@icloud.com> 1734336661 +0100	checkout: moving from remove-delegates to gix-command-api
c67770ff118f00846936117fccc2c59ca3220f6e a0d9f51113febefd7926214d9086a9429ac10617 Sebastian Thiel <sebastian.thiel@icloud.com> 1734336666 +0100	checkout: moving from gix-command-api to remove-delegates
a0d9f51113febefd7926214d9086a9429ac10617 faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734336677 +0100	checkout: moving from remove-delegates to main
faa0cdeb35a8135ff9513a1c9884126f6b080f4a a0d9f51113febefd7926214d9086a9429ac10617 Sebastian Thiel <sebastian.thiel@icloud.com> 1734336691 +0100	checkout: moving from main to remove-delegates
b9f9273d82a1fb578d014de3ceea37e4f74022ab b9f9273d82a1fb578d014de3ceea37e4f74022ab Sebastian Thiel <sebastian.thiel@icloud.com> 1734361997 +0100	reset: moving to HEAD
b9f9273d82a1fb578d014de3ceea37e4f74022ab b9f9273d82a1fb578d014de3ceea37e4f74022ab Sebastian Thiel <sebastian.thiel@icloud.com> 1734362000 +0100	reset: moving to HEAD
29950a74407ea2244ec6d25f64cee15b12b45961 29950a74407ea2244ec6d25f64cee15b12b45961 Sebastian Thiel <sebastian.thiel@icloud.com> 1734370049 +0100	reset: moving to HEAD
29950a74407ea2244ec6d25f64cee15b12b45961 29950a74407ea2244ec6d25f64cee15b12b45961 Sebastian Thiel <sebastian.thiel@icloud.com> 1734370054 +0100	reset: moving to HEAD
43193364412e51af030533cbb503260b7a6b0d8e 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734373040 +0100	reset: moving to HEAD
43193364412e51af030533cbb503260b7a6b0d8e faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734373044 +0100	checkout: moving from remove-delegates to main
faa0cdeb35a8135ff9513a1c9884126f6b080f4a 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734373093 +0100	checkout: moving from main to remove-delegates
43193364412e51af030533cbb503260b7a6b0d8e 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734373131 +0100	reset: moving to HEAD
43193364412e51af030533cbb503260b7a6b0d8e 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734373419 +0100	reset: moving to HEAD
43193364412e51af030533cbb503260b7a6b0d8e faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734373423 +0100	checkout: moving from remove-delegates to main
faa0cdeb35a8135ff9513a1c9884126f6b080f4a faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734373562 +0100	reset: moving to HEAD
faa0cdeb35a8135ff9513a1c9884126f6b080f4a 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734373565 +0100	checkout: moving from main to remove-delegates
43193364412e51af030533cbb503260b7a6b0d8e 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734374571 +0100	reset: moving to HEAD
43193364412e51af030533cbb503260b7a6b0d8e 43193364412e51af030533cbb503260b7a6b0d8e Sebastian Thiel <sebastian.thiel@icloud.com> 1734374762 +0100	reset: moving to HEAD
95664885d3a6f2e1fa4cb4b8a1ea9fd8e5abfc28 faa0cdeb35a8135ff9513a1c9884126f6b080f4a Sebastian Thiel <sebastian.thiel@icloud.com> 1734383747 +0100	checkout: moving from remove-delegates to main
faa0cdeb35a8135ff9513a1c9884126f6b080f4a ddeb97f550bb95835648841b476d7647dd7c1dc0 Sebastian Thiel <sebastian.thiel@icloud.com> 1734383748 +0100	pull --ff-only: Fast-forward
ddeb97f550bb95835648841b476d7647dd7c1dc0 972d720ee535e12c9f02eac2080dda462c48ba83 Sebastian Thiel <sebastian.thiel@icloud.com> 1734440285 +0100	pull --ff-only: Fast-forward
972d720ee535e12c9f02eac2080dda462c48ba83 972d720ee535e12c9f02eac2080dda462c48ba83 Sebastian Thiel <sebastian.thiel@icloud.com> 1734440466 +0100	checkout: moving from main to radicle-tuning
25b848080c7df2da0fa662c580451aec0deb29c4 cd9fae3593c679dfe5951a99551142aeac264dd3 Sebastian Thiel <sebastian.thiel@icloud.com> 1734457399 +0100	revert: Revert "Revert "forcefully fix CI until it's clear what the problem is""
d1d3f7c48695bd6de401d1991dc4d161f88662b5 972d720ee535e12c9f02eac2080dda462c48ba83 Sebastian Thiel <sebastian.thiel@icloud.com> 1734511003 +0100	checkout: moving from radicle-tuning to main
972d720ee535e12c9f02eac2080dda462c48ba83 a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734511005 +0100	pull --ff-only: Fast-forward
a54277561a62cd560a9a072c6052eaf182ad4ace fab6b303401a25b97813fb1779ebf3b60c767e1e Sebastian Thiel <sebastian.thiel@icloud.com> 1734511016 +0100	checkout: moving from main to no-special-files
64872690e60efdd9267d517f4d9971eecd3b875c a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734511098 +0100	reset: moving to a54277561a62cd560a9a072c6052eaf182ad4ace
665db1c95e8f946e6363112bf30f17f1f87f5fd4 a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734511333 +0100	checkout: moving from no-special-files to main
a54277561a62cd560a9a072c6052eaf182ad4ace a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734511472 +0100	checkout: moving from main to dirwalk-ignore-non-regulars
5a434aa1bfcf28dac9554dff5a225c1175b4c8d2 5a434aa1bfcf28dac9554dff5a225c1175b4c8d2 Sebastian Thiel <sebastian.thiel@icloud.com> 1734532681 +0100	reset: moving to HEAD
a49c960bf7e9ec41b3d0548a3aa6ccc90a59cc2b a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734536090 +0100	checkout: moving from dirwalk-ignore-non-regulars to main
a54277561a62cd560a9a072c6052eaf182ad4ace a49c960bf7e9ec41b3d0548a3aa6ccc90a59cc2b Sebastian Thiel <sebastian.thiel@icloud.com> 1734536096 +0100	checkout: moving from main to dirwalk-ignore-non-regulars
a49c960bf7e9ec41b3d0548a3aa6ccc90a59cc2b a54277561a62cd560a9a072c6052eaf182ad4ace Sebastian Thiel <sebastian.thiel@icloud.com> 1734537799 +0100	checkout: moving from dirwalk-ignore-non-regulars to main
a54277561a62cd560a9a072c6052eaf182ad4ace 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734537801 +0100	pull --ff-only: Fast-forward
69ee6a32dd221a1aae7b8c3817f90feacf577598 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734538890 +0100	checkout: moving from main to hEAD
69ee6a32dd221a1aae7b8c3817f90feacf577598 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734538898 +0100	checkout: moving from 69ee6a32dd221a1aae7b8c3817f90feacf577598 to main
69ee6a32dd221a1aae7b8c3817f90feacf577598 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734594187 +0100	checkout: moving from main to journey-testing
59eae5324e782408b003c156758c4e9b22dee004 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734594231 +0100	checkout: moving from journey-testing to main
69ee6a32dd221a1aae7b8c3817f90feacf577598 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734594307 +0100	checkout: moving from main to fix-1729
69d533349766ab1331606b1139a9e0d7d86142ac 0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b Sebastian Thiel <sebastian.thiel@icloud.com> 1734631777 +0100	reset: moving to FETCH_HEAD
0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b 69d533349766ab1331606b1139a9e0d7d86142ac Sebastian Thiel <sebastian.thiel@icloud.com> 1734632233 +0100	reset: moving to 69d5333
69d533349766ab1331606b1139a9e0d7d86142ac 0f1da23d82debc5945b85dbb6933693eb20e7a64 Sebastian Thiel <sebastian.thiel@icloud.com> 1734632252 +0100	checkout: moving from fix-1729 to gix-blame
0f1da23d82debc5945b85dbb6933693eb20e7a64 0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b Sebastian Thiel <sebastian.thiel@icloud.com> 1734632258 +0100	reset: moving to FETCH_HEAD
0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b 0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b Sebastian Thiel <sebastian.thiel@icloud.com> 1734632301 +0100	reset: moving to HEAD
db5c9cfce93713b4b3e249cff1f8cc1ef146f470 0cfc8a644ffa5b9de4e42ba2c96bb987eb66dd0b Sebastian Thiel <sebastian.thiel@icloud.com> 1734632361 +0100	reset: moving to FETCH_HEAD
fb86e8253d7c467c0b5876c8a91d163b8d6b84e7 69d533349766ab1331606b1139a9e0d7d86142ac Sebastian Thiel <sebastian.thiel@icloud.com> 1734677248 +0100	checkout: moving from gix-blame to fix-1729
3614c21da2357cb58b7e8572ca49b52656780594 69ee6a32dd221a1aae7b8c3817f90feacf577598 Sebastian Thiel <sebastian.thiel@icloud.com> 1734700115 +0100	checkout: moving from fix-1729 to main
69ee6a32dd221a1aae7b8c3817f90feacf577598 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734700122 +0100	pull --ff-only: Fast-forward
6822689fca04c15e309f9ca41d610bca9cb93e3b fb86e8253d7c467c0b5876c8a91d163b8d6b84e7 Sebastian Thiel <sebastian.thiel@icloud.com> 1734700151 +0100	checkout: moving from main to gix-blame
a590b999aea0800a2b4d238d3eb6b5fbd00d50cf 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734717112 +0100	checkout: moving from gix-blame to main
6822689fca04c15e309f9ca41d610bca9cb93e3b 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734717123 +0100	checkout: moving from main to fix-pack-receive
330a40098a5e88a80996d94ae0866cc9e8662972 330a40098a5e88a80996d94ae0866cc9e8662972 Sebastian Thiel <sebastian.thiel@icloud.com> 1734718496 +0100	reset: moving to HEAD
a06f7d0e48e975ed6402cd417a6647966d019e32 a06f7d0e48e975ed6402cd417a6647966d019e32 Sebastian Thiel <sebastian.thiel@icloud.com> 1734719077 +0100	reset: moving to HEAD
71ad8f3b18b3065b8e5ac7e3b36b5f013bcd6b88 71ad8f3b18b3065b8e5ac7e3b36b5f013bcd6b88 Sebastian Thiel <sebastian.thiel@icloud.com> 1734719719 +0100	reset: moving to HEAD
7d5dbbe3e395c63ae200325d035f8df908ed13ac 7d5dbbe3e395c63ae200325d035f8df908ed13ac Sebastian Thiel <sebastian.thiel@icloud.com> 1734720131 +0100	reset: moving to HEAD
31ce3ff8fdad581fc9d4c2ece0aacaa7c66d6703 31ce3ff8fdad581fc9d4c2ece0aacaa7c66d6703 Sebastian Thiel <sebastian.thiel@icloud.com> 1734720294 +0100	reset: moving to HEAD
4eaf725a52200f0af4283ea1c3db32d12a1cf6cb 4eaf725a52200f0af4283ea1c3db32d12a1cf6cb Sebastian Thiel <sebastian.thiel@icloud.com> 1734720619 +0100	reset: moving to HEAD
466fe524451064339a4e603526ea3a5bc30b6fb8 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734722136 +0100	checkout: moving from fix-pack-receive to main
6822689fca04c15e309f9ca41d610bca9cb93e3b 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734722229 +0100	checkout: moving from main to main
6822689fca04c15e309f9ca41d610bca9cb93e3b 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734722331 +0100	reset: moving to HEAD
6822689fca04c15e309f9ca41d610bca9cb93e3b 466fe524451064339a4e603526ea3a5bc30b6fb8 Sebastian Thiel <sebastian.thiel@icloud.com> 1734722340 +0100	checkout: moving from main to fix-pack-receive
5c21ebc3f523bbe64cb84bbcdf39a2c284ba1df1 6822689fca04c15e309f9ca41d610bca9cb93e3b Sebastian Thiel <sebastian.thiel@icloud.com> 1734770703 +0100	checkout: moving from fix-pack-receive to main
6822689fca04c15e309f9ca41d610bca9cb93e3b ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734770705 +0100	pull --ff-only: Fast-forward
ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc 5c21ebc3f523bbe64cb84bbcdf39a2c284ba1df1 Sebastian Thiel <sebastian.thiel@icloud.com> 1734770708 +0100	checkout: moving from main to fix-pack-receive
5c21ebc3f523bbe64cb84bbcdf39a2c284ba1df1 ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734770713 +0100	checkout: moving from fix-pack-receive to main
ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc 6b2643530e04bccfc741777a4b84d645d1537574 Sebastian Thiel <sebastian.thiel@icloud.com> 1734770725 +0100	checkout: moving from main to refloglookup-date
db5c9cfce93713b4b3e249cff1f8cc1ef146f470 ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734770813 +0100	reset: moving to ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc
4d4a9b6e372b3f6d8beabc82a6bf63d5b3f84e21 ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734789256 +0100	checkout: moving from refloglookup-date to main
ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734789267 +0100	checkout: moving from main to gix-reflog-parsing
ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc 0000000000000000000000000000000000000000 Sebastian Thiel <sebastian.thiel@icloud.com> 1734789298 +0100	Branch: renamed refs/heads/gix-reflog-parsing to refs/heads/reflog-parseing
0000000000000000000000000000000000000000 ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734789298 +0100	Branch: renamed refs/heads/gix-reflog-parsing to refs/heads/reflog-parseing
ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc 0000000000000000000000000000000000000000 Sebastian Thiel <sebastian.thiel@icloud.com> 1734789302 +0100	Branch: renamed refs/heads/reflog-parseing to refs/heads/reflog-parsing
0000000000000000000000000000000000000000 ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc Sebastian Thiel <sebastian.thiel@icloud.com> 1734789302 +0100	Branch: renamed refs/heads/reflog-parseing to refs/heads/reflog-parsing
87b0acf0e9cac2781312bd478df0ae72ec6d194b 4d4a9b6e372b3f6d8beabc82a6bf63d5b3f84e21 Sebastian Thiel <sebastian.thiel@icloud.com> 1734789364 +0100	checkout: moving from reflog-parsing to refloglookup-date
EOF
