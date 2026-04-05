# cat-self-update

## Context

Currently dogfooding.

## Usage

Reference implementation of self-update for Windows Rust.

## Approach

Prioritizes simple implementation.

## Plans
- The following features are planned to be implemented in the library crate:
  - auto-update
  - background-check
  - force-update (when an update is detected by background-check)
  - Notice upon app termination (when an update is detected by background-check)

## install

```
cargo install --force --git https://github.com/cat2151/cat-self-update
```

## Run

```
cat-self-update update
cat-self-update check
```

## Note

Requires Python to function correctly.

## Operational Notes

- Ideal: If there's a bug fix for this library, apps using it should be able to automatically detect it and perform `cargo install`.
- Analysis: However, to achieve this, the 'cost incurred solely for this library' on the app side would be substantial and disproportionate.
- Reality: If the `update` subcommand itself, which utilizes this library, crashes, re-run `cargo install`.