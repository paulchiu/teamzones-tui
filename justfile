# justfile for teamzones - who's available and where it's busy, on one time axis

# Show available recipes
default:
    @just --list

# Build the workspace in debug mode
build:
    cargo build --workspace --locked

# Build the workspace in release mode
release:
    cargo build --workspace --release --locked

# Run all tests
test:
    cargo test --workspace --locked

# Format code with rustfmt
fmt:
    cargo fmt --all

# Fail if code is not formatted
fmt-check:
    cargo fmt --all --check

# Lint with clippy, warnings are errors
lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Fail on a broken intra-doc link or a malformed doc comment
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Assert the core crate stays usable by a consumer that owns no terminal
#
# An allowlist rather than a denylist, so a terminal or clipboard crate cannot reach
# the core by being one nobody thought to ban. Each entry pairs a crate with the
# reason it is allowed, and a direct dependency missing from the list fails.
check-core-isolation:
    #!/usr/bin/env bash
    set -euo pipefail
    allowed_with_reasons=(
        "chrono:instants, local dates and weekday maths for the slot grid"
        "chrono-tz:IANA zones and the abbreviations copy prints"
        "serde:deserialising config.toml into typed tables"
        "thiserror:error enums whose Display is the user-facing text, unlike blubat's hand-written Display"
        "toml:parsing config.toml"
        "toml_edit:format-preserving write-back of config.toml text"
    )
    allowed=()
    for entry in "${allowed_with_reasons[@]}"; do
        name="${entry%%:*}"
        reason="${entry#*:}"
        if [ -z "$name" ] || [ "$name" = "$entry" ] || [ -z "$reason" ]; then
            echo "malformed allowlist entry: '$entry' (expected 'crate:reason')" >&2
            exit 1
        fi
        allowed+=("$name")
    done
    actual=$(cargo tree -p teamzones-core --edges normal --depth 1 --prefix none --locked \
        | awk 'NR > 1 && NF { print $1 }' | sort -u)
    unexpected=()
    for crate in $actual; do
        if ! printf '%s\n' "${allowed[@]}" | grep -qx "$crate"; then
            unexpected+=("$crate")
        fi
    done
    if [ "${#unexpected[@]}" -ne 0 ]; then
        echo "teamzones-core depends on crates outside its allowlist: ${unexpected[*]}" >&2
        echo "allowed, with reasons:" >&2
        for entry in "${allowed_with_reasons[@]}"; do
            echo "  ${entry%%:*}: ${entry#*:}" >&2
        done
        echo "add a crate above with its reason only if no frontend can do without it" >&2
        exit 1
    fi
    echo "teamzones-core's direct dependencies are all allowed: ${actual:-none}" | tr '\n' ' '
    echo

# Build and test against the declared rust-version floor
msrv:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(cargo metadata --no-deps --format-version 1 \
        | jq -r '.packages[] | select(.name == "teamzones") | .rust_version')
    if [ -z "$version" ] || [ "$version" = "null" ]; then
        echo "no rust-version in the manifest, so there is no floor to prove" >&2
        exit 1
    fi
    rustup toolchain install "$version" --profile minimal
    cargo "+$version" test --workspace --locked

# Parse every GitHub workflow, so a typo fails a check rather than a release
workflows:
    #!/usr/bin/env python3
    import pathlib, sys, yaml

    files = sorted(pathlib.Path(".github/workflows").glob("*.yml"))
    if not files:
        sys.exit("no workflows found under .github/workflows")
    failures = []
    for path in files:
        try:
            document = yaml.safe_load(path.read_text())
        except yaml.YAMLError as error:
            failures.append(f"{path}: {error}")
            continue
        if not isinstance(document, dict) or "jobs" not in document:
            failures.append(f"{path}: no jobs table")
    for failure in failures:
        print(failure, file=sys.stderr)
    if failures:
        sys.exit(1)
    print(f"{len(files)} workflows parse")

# The single definition of CI: the GitHub workflow runs this recipe, so a green
# run here is a green pipeline
ci: fmt-check lint test docs check-core-isolation build
