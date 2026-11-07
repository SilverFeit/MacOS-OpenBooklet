# Contributing to MacOS OpenBooklet

Thank you for your interest in contributing to **MacOS OpenBooklet**!

## Clean-Room & MIT License Policy

**MacOS OpenBooklet** is a clean-room implementation licensed under the [MIT License](LICENSE). All algorithms in `openbooklet-core` and `openbooklet-pdf` must be written from first-principles mathematics and public prepress specifications (`docs/SPEC.md`), without copying code from GPL/AGPL projects.

## Development Workflow

```bash
# Run the unit and end-to-end vector PDF test suite
cargo test

# Build release app bundle, CLI, .dmg, .app.zip & SHA256SUMS.txt
./Scripts/package_release.sh
```
