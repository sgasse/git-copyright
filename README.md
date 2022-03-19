# git_copyright

Extract added/last modified times from git history and add/update copyright notes accordingly.

## Installation

The easiest way to install `git_copyright` is via `cargo` from `crates.io`:

```bash
cargo install git_copyright
```

If you want to build it from source, clone the repository and then run:

```bash
cargo build --release
```

## Running

The only required argument is the name that your copyright should carry, e.g.:

```bash
git_copyright --name "MyCompany Ltd."
```

Additional useful arguments:

- `--repo`: Specify a repo-root other than `./`.
- `--config`: Pass your own YAML config file with comment signs and glob patterns to ignore.
- `--ignore-changes`: Do not exit with an error even if tracked files changed.

A full command might look like this:

```bash
git_copyright --name "MyCompany Ltd." --repo "../../my_repo" --config "./custom_cfg.yml" --ignore-changes
```

### Run with Docker

You can also use a pre-built image:

```bash
docker run --rm -u $(id -u) -v $(pwd):/mnt sgasse/git_copyright:v0.2.7 --name "MyCompany Ltd."
```

## Development

When developing, you can set the log environment variable to see debug log output:

```bash
RUST_LOG=debug cargo run -- --repo "../../my_repo" --name "MyCompany Ltd."
```
