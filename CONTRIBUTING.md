# Contributing

## Branch policy

`master` is protected by convention: **nothing is committed to it directly**.
All work happens on short-lived branches and reaches `master` through a
reviewed merge when a milestone is stable.

| Branch | Purpose |
|---|---|
| `master` | Stable, always buildable. Only merged milestones and releases. |
| `feat/m<N>-<topic>` | One branch per milestone slice (for example `feat/m0-inspect`). |
| `fix/<topic>` | Bugs found after a milestone merged. |
| `docs/<topic>` | Documentation-only changes. |

Branches may stack: `feat/m1-runtime` starts from `feat/m0-inspect` and is
rebased onto `master` after the previous milestone merges.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(apk): parse resources.arsc labels
fix(cli): reject relative paths in remove
docs(roadmap): update M1 status
test(axml): cover UTF-16 string pools
chore(ci): add release build
```

Rules:

- One logical change per commit; the tree must build at every commit.
- Reference the milestone in the body when relevant (`M1`, `ROADMAP`).
- Never commit APKs, build output or generated binaries.

## Before pushing

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same checks on every push and pull request.

## Merging and releases

1. Milestone is complete and the acceptance criteria in `docs/ROADMAP.md` are met.
2. Open a pull request; CI must be green.
3. Merge with `--no-ff` to keep the milestone visible in history:

   ```sh
   git checkout master
   git merge --no-ff feat/m0-inspect
   ```

4. Tag releases on `master`: `git tag -a v0.1.0 -m "M0: APK inspection"`.

## Licensing

By contributing you agree that your changes are licensed under
GPL-3.0-or-later, the license of this project.
