# git-copyright

Extract added/last modified times from git history and add/update copyright notes accordingly.

## Building & Running
Build
```bash
cargo build --release
```

Run (default config)
```bash
RUST_LOG=debug cargo run -- --repo "../tmp/test_repo" --name "DummyCompany Ltd."
```

Run (with custom config)
```bash
RUST_LOG=debug cargo run -- --repo "../tmp/test_repo" --config "./my_cfg.yml" --name "DummyCompany Ltd."
```
