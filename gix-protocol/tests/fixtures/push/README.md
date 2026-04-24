# push fixtures

Recorded pkt-line byte streams from real `git push` invocations, captured by
wrapping `git-receive-pack` with a `tee`. Each fixture is two files:

- `<name>.c2s.bin` — client → server bytes (command list + pack)
- `<name>.s2c.bin` — server → client bytes (ref advertisement + report)

## Regenerating

```
bash gix-protocol/tests/fixtures/push/capture.sh
```

Requires git >= 2.30. Object IDs will differ on each run (commits include
timestamps); tests must be written to be OID-agnostic (parse the ref adv.,
extract OIDs from *within* the fixture, don't hard-code).

## Scenarios

| Name                    | What it exercises |
|-------------------------|-------------------|
| `empty-to-new-branch`   | Push one commit from a new repo to a previously-empty bare remote. Simplest happy path; remote advertises only `capabilities^{}` pseudo-ref. |
| `fast-forward`          | Remote already has a ref; client pushes a successor. Exercises `<old-oid> <new-oid> <ref>` format (old-oid non-zero). |
| `delete-ref`            | `git push remote :branch` — deletes the branch. Client sends `<oid> 0000…0000 <ref>`; no pack is sent when command list is delete-only. |
| `non-ff-rejected`       | Client attempts a non-fast-forward push; remote replies `ng <ref> non-fast-forward`. Exercises the error path in report parsing. |
