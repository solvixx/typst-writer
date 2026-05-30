# Typst Writer Dev Tools

This directory contains standalone utility scripts and binary tools used for development and debugging purposes.

These tools are excluded from the main project build to improve compilation speeds.

## Running a Tool
To run a tool, you can use `cargo run` by specifying the path (if treated as a cargo project) or just execute the source file if it's a standalone script:

```bash
# Example
cargo run --bin inspect_geom --manifest-path dev-tools/Cargo.toml
```

*Note: If you need these tools to be part of the workspace, create a `Cargo.toml` in this directory and add it to the workspace members in the root `Cargo.toml`.*
