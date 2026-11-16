# Contributing

## Use of AI

All use of AI in contributions must follow the
[AI Policy](https://github.com/cryptopatrick/robocode/blob/master/AI_POLICY.md).

Contributions not following the AI Policy will be closed.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): a type such
as `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:` or `style:`,
then a short description. The release notes are generated from these messages
(see `cliff.toml`), so a clear first line is what ends up in the changelog.

## Releasing

Needs [git-cliff](https://git-cliff.org): `cargo install git-cliff --locked`.

1. Bump `version` in `Cargo.toml`, and the version in `README.md`.
2. `scripts/changelog vX.Y.Z` adds the new section to `CHANGELOG.md`, generated
   from the commits since the last tag. Review it and edit where a message
   needs more words. Mark a breaking change with `!` after its type (for
   example `feat!:`), so the notes say *Breaking*.
3. Commit the bump and the changelog, and push.
4. `cargo publish -p robocode --dry-run`, then `cargo publish -p robocode`.
5. Tag and push: `git tag -a vX.Y.Z -m "robocode X.Y.Z: <summary>"`, then
   `git push origin vX.Y.Z`.
6. Release page: `git cliff --latest --strip header > notes.md`, then
   `gh release create vX.Y.Z --verify-tag --title "robocode X.Y.Z: <summary>" --notes-file notes.md`.
