default:
    @just --list

# Format check + clippy as warnings-are-errors. Mirrors CI gates.
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# Re-run `cargo check` (or any other cargo subcommand) on file change.
watch cmd='check':
    cargo watch -x {{cmd}}

# Preview the artifacts that a release would build.
dist-plan:
    dist plan

# Bump version, commit, tag vX.Y.Z, push. Fires the release workflow.
release version:
    @git diff --quiet && git diff --cached --quiet || { echo "error: uncommitted changes"; exit 1; }
    cargo set-version {{version}}
    git commit -am "Release v{{version}}"
    git tag v{{version}}
    git push --follow-tags
