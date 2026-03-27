# cat-self-update

## Status
Currently dogfooding.

## Purpose
Reference implementation for self-updating Rust applications on Windows.

## Approach
Prioritizing a simple implementation.

## Plans
The following features are planned to be implemented in a library crate:
  - hash
  - check
  - auto-update
  - background check
  - force-update (when an update is detected by background check)
  - notification upon application exit (when an update is detected by background check)

## Install

```
cargo install --force --git https://github.com/cat2151/cat-self-update
```

## Run

```
cat-self-update update
cat-self-update check
```

Note: It will not function correctly without Python.