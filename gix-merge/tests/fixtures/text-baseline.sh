#!/usr/bin/env bash
set -eu -o pipefail

git init
rm -Rf .git/hooks

function baseline() {
  local ours=$DIR/${1:?1: our file}.blob;
  local base=$DIR/${2:?2: base file}.blob;
  local theirs=$DIR/${3:?3: their file}.blob;
  local output=$DIR/${4:?4: the name of the output file}.merged;

  shift 4
  git merge-file --stdout "$@" "$ours" "$base" "$theirs" > "$output" || true
  echo "$ours" "$base" "$theirs" "$output" "$@" >> baseline.cases

  local output="${output}-reversed"
  git merge-file --stdout "$@" "$theirs" "$base" "$ours" > "${output}" || true
  echo "$theirs" "$base" "$ours" "${output}" "$@" >> baseline-reversed.cases
}

mkdir simple
(cd simple
  echo -e "line1-changed-by-both\nline2-to-be-changed-in-incoming" > ours.blob
  echo -e "line1-to-be-changed-by-both\nline2-to-be-changed-in-incoming" > base.blob
  echo -e "line1-changed-by-both\nline2-changed" > theirs.blob
)

# one big change includes multiple smaller ones
mkdir multi-change
(cd multi-change
  cat <<EOF > base.blob
0
1
2
3
4
5
6
7
8
9
EOF

  cat <<EOF > ours.blob
0
1
X
X
4
5
Y
Y
8
Z
EOF

  cat <<EOF > theirs.blob
T
T
T
T
T
T
T
T
T
T
EOF
)

# a change with deletion/clearing our file
mkdir clear-ours
(cd clear-ours
  cat <<EOF > base.blob
0
1
2
3
4
5
EOF

  touch ours.blob

  cat <<EOF > theirs.blob
T
T
T
T
T
EOF
)

# a change with deletion/clearing their file
mkdir clear-theirs
(cd clear-theirs
  cat <<EOF > base.blob
0
1
2
3
4
5
EOF

  cat <<EOF > ours.blob
O
O
O
O
O
EOF

  touch theirs.blob
)

# differently sized changes
mkdir ours-2-lines-theirs-1-line
(cd ours-2-lines-theirs-1-line
  cat <<EOF > base.blob
0
1
2
3
4
5
EOF

  cat <<EOF > ours.blob
0
1
X
X
4
5
EOF

  cat <<EOF > theirs.blob
0
1
Y
3
4
5
EOF
)

# partial match
mkdir partial-match
(cd partial-match
  cat <<EOF > base.blob
0
1
2
3
4
5
EOF

  cat <<EOF > ours.blob
0
X1
X2
X3
X4
5
EOF

  cat <<EOF > theirs.blob
0
X1
2
X3
X4
5
EOF
)

# based on 'unique merge base' from 'diff3-conflict-markers'
mkdir unique-merge-base-with-insertion
(cd unique-merge-base-with-insertion
  cat <<EOF > base.blob
1
2
3
4
5
EOF

  # no trailing newline
  echo -n $'1\n2\n3\n4\n5\n7' > ours.blob
  echo -n $'1\n2\n3\n4\n5\nsix' > theirs.blob
)

mkdir zdiff3-basic
(cd zdiff3-basic
  cat <<EOF > base.blob
1
2
3
4
5
6
7
8
9
EOF

  cat <<EOF > ours.blob
1
2
3
4
A
B
C
D
E
7
8
9
EOF

  cat <<EOF > theirs.blob
1
2
3
4
A
X
C
Y
E
7
8
9
EOF
)

mkdir zdiff3-middlecommon
(cd zdiff3-middlecommon
  cat <<EOF > base.blob
1
2
3
AA
4
5
BB
6
7
8
EOF

  cat <<EOF > ours.blob
1
2
3
CC
4
5
DD
6
7
8
EOF

  cat <<EOF > theirs.blob
1
2
3
EE
4
5
FF
6
7
8
EOF
)


mkdir zdiff3-interesting
(cd zdiff3-interesting
  cat <<EOF > base.blob
1
2
3
4
5
6
7
8
9
EOF

  cat <<EOF > ours.blob
1
2
3
4
A
B
C
D
E
F
G
H
I
J
7
8
9
EOF

  cat <<EOF > theirs.blob
1
2
3
4
A
B
C
5
6
G
H
I
J
7
8
9
EOF
)

mkdir zdiff3-evil
(cd zdiff3-evil
  cat <<EOF > base.blob
1
2
3
4
5
6
7
8
9
EOF

  cat <<EOF > ours.blob
1
2
3
4
X
A
B
C
7
8
9
EOF

  cat <<EOF > theirs.blob
1
2
3
4
Y
A
B
C
B
C
7
8
9
EOF
)

mkdir no-change-add
(cd no-change-add

  echo -e "  line 1\n\n  line 2" > base.blob
  echo -e "  line 1\n\n  line in between\n\n  line 2\n\n  line in between\n\n  line 3" > ours.blob
  cp ours.blob theirs.blob
)

mkdir no-change-remove
(cd no-change-remove

  echo -e "  line 1\n\n  line in between\n\n  line 2\n\n  line in between\n\n  line 3" > base.blob
  echo -e "  line 1\n\n  line 2" > ours.blob
  cp ours.blob theirs.blob
)

mkdir complex
(cd complex
  cat <<EOF >base.blob
Dominus regit me,
et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
propter nomen suum.
EOF

	cat <<EOF >new1.blob
Dominus regit me,
et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
propter nomen suum.
Nam et si ambulavero in medio umbrae mortis,
non timebo mala, quoniam tu mecum es:
virga tua et baculus tuus ipsa me consolata sunt.
EOF

	cat <<EOF >new2.blob
Dominus regit me, et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
propter nomen suum.
EOF

	cat <<EOF >new3.blob
DOMINUS regit me,
et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
propter nomen suum.
EOF

	cat <<EOF >new4.blob
Dominus regit me, et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
EOF

	cat <<EOF >new5.blob
Dominus regit me,
et nihil mihi deerit.
In loco pascuae ibi me collocavit,
super aquam refectionis educavit me;
animam meam convertit,
deduxit me super semitas jusitiae,
propter nomen suum.
Nam et si ambulavero in medio umbrae mortis,
non timebo mala, quoniam TU mecum es:
virga tua et baculus tuus ipsa me consolata sunt.
EOF

	echo -n "propter nomen suum." >>new4.blob

	cat <<EOF >base.c
int f(int x, int y)
{
  if (x == 0)
  {
    return y;
  }
  return x;
}

int g(size_t u)
{
  while (u < 30)
  {
    u++;
  }
  return u;
}
EOF

	cat <<EOF >ours.c
int g(size_t u)
{
  while (u < 30)
  {
    u++;
  }
  return u;
}

int h(int x, int y, int z)
{
  if (z == 0)
  {
    return x;
  }
  return y;
}
EOF

	cat <<EOF >theirs.c
int f(int x, int y)
{
  if (x == 0)
  {
    return y;
  }
  return x;
}

int g(size_t u)
{
  while (u > 34)
  {
    u--;
  }
  return u;
}
EOF

  mkdir no-change
  (cd no-change
    cp ../base.blob .
    cp base.blob ours.blob
    cp base.blob theirs.blob
  )
  mkdir no-conflict
  (cd no-conflict
    cp ../new1.blob ours.blob
    cp ../base.blob base.blob
    cp ../new2.blob theirs.blob
  )
  mkdir no-conflict-too
  (cd no-conflict-too
    cp ../base.blob ours.blob
    cp ../base.blob base.blob
    cp ../new2.blob theirs.blob
  )
  mkdir they-changed
  (cd they-changed
    touch ours.blob base.blob
    cp ../new2.blob theirs.blob
  )
  mkdir missing-LF-at-EOF
  (cd missing-LF-at-EOF
    cp ../new1.blob ours.blob
    cp ../base.blob base.blob
    cp ../new4.blob theirs.blob
  )
  mkdir missing-LF-at-EOF-no-conflict
  (cd missing-LF-at-EOF-no-conflict
    cp ../new4.blob ours.blob
    cp ../new2.blob base.blob
    cp ../new3.blob theirs.blob
  )
  mkdir with-conflicts
  (cd with-conflicts
    cp ../new1.blob ours.blob
    cp ../base.blob base.blob
    cp ../new3.blob theirs.blob
  )
  mkdir with-conflicts-in-removed-tail
  (cd with-conflicts-in-removed-tail
    cp ../base.blob ours.blob
    cp ../new1.blob base.blob
    cp ../new5.blob theirs.blob
  )
  mkdir auto-simplification
  (cd auto-simplification
    sed -e "s/deerit.\$/deerit;/" -e "s/me;\$/me./" <../new5.blob >ours.blob
    cp ../new5.blob base.blob
    sed -e "s/deerit.\$/deerit,/" -e "s/me;\$/me,/" <../new5.blob >theirs.blob
  )
  mkdir auto-simplification2
  (cd auto-simplification2
    sed -e "s/deerit./&%%%%/" -e "s/locavit,/locavit;/" <../auto-simplification/ours.blob | tr % "\012" >ours.blob
    cp ../new5.blob base.blob
    sed -e "s/deerit./&%%%%/" -e "s/locavit,/locavit --/" <../auto-simplification/theirs.blob | tr % "\012" >theirs.blob
  )
  mkdir conflict-without-LF
  (cd conflict-without-LF
   	printf "line1\nline2\nline3" >base.blob
   	printf "line1\nline2\nline3x" >ours.blob
   	printf "line1\nline2\nline3y" >theirs.blob
  )

  mkdir marker-newline-handling-crlf
  (cd marker-newline-handling-crlf
  	printf "1\\r\\n2\\r\\n3" >base.blob
  	printf "1\\r\\n2\\r\\n4" >ours.blob
  	printf "1\\r\\n2\\r\\n5" >theirs.blob
  )

  mkdir marker-newline-handling-lf
  (cd marker-newline-handling-lf
  	printf "1\\r\\n2\\n3" >base.blob
  	printf "1\\r\\n2\\n4" >ours.blob
  	printf "1\\r\\n2\\n5" >theirs.blob
  )

  mkdir marker-newline-handling-lf2
  (cd marker-newline-handling-lf2
  	printf "1\\r\\n2\\r\\n3" >base.blob
  	printf "1\\r\\n2\\n4" >ours.blob
  	printf "1\\r\\n2\\n5" >theirs.blob
  )

  mkdir spurious-c-conflicts
  (cd spurious-c-conflicts
  	cp ../base.c base.blob
  	cp ../ours.c ours.blob
  	cp ../theirs.c theirs.blob
  )
)

mkdir line-ending-change
(cd line-ending-change

  echo -e "a\n" > base.blob
  echo -e "a\r\n" > ours.blob
  echo -e "a\n" > theirs.blob
)


for dir in  simple \
            multi-change \
            clear-ours \
            clear-theirs \
            ours-2-lines-theirs-1-line \
            partial-match \
            unique-merge-base-with-insertion \
            zdiff3-basic \
            zdiff3-middlecommon \
            zdiff3-interesting \
            zdiff3-evil \
            no-change-add \
            no-change-remove \
            line-ending-change \
            complex/no-change \
            complex/no-conflict \
            complex/no-conflict-too \
            complex/they-changed \
            complex/missing-LF-at-EOF \
            complex/missing-LF-at-EOF-no-conflict \
            complex/with-conflicts \
            complex/with-conflicts-in-removed-tail \
            complex/auto-simplification \
            complex/auto-simplification2 \
            complex/conflict-without-LF \
            complex/marker-newline-handling-crlf \
            complex/marker-newline-handling-lf \
            complex/marker-newline-handling-lf2 \
            complex/spurious-c-conflicts; do
  DIR=$dir
  baseline ours base theirs merge
  baseline ours base theirs diff3 --diff3
  baseline ours base theirs zdiff3 --zdiff3
  baseline ours base theirs merge-ours --ours
  baseline ours base theirs merge-theirs --theirs
  baseline ours base theirs merge-union --union
  (
    export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=diff.algorithm GIT_CONFIG_KEY_VALUE=0=histogram
    baseline ours base theirs diff3-histogram --diff3
    baseline ours base theirs zdiff3-histogram --zdiff3
  )
done