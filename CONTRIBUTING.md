# Contributing

## Before you change anything

1. Read [ARCHITECTURE](docs/ARCHITECTURE.md) and the [ADRs](01_governance/adr/README.md).
2. Find the feature in [docs/STATUS.md](docs/STATUS.md) or [docs/traceability.csv](docs/traceability.csv). A change that moves a feature to a better status must add the code **and** the automated test that proves it, and update the evidence in `crates/apsi-core/src/traceability.rs`.

## Local checks

```sh
node tools/verify.mjs            # everything CI runs, in order, stopping at the first failure
node tools/verify.mjs --skip-web # Rust only
```

A change is ready only when `verify.mjs` passes with zero warnings. The workspace denies all rustc and clippy warnings, so there is no warning budget.

After changing the register, capabilities, statuses or units, regenerate the generated files and commit them:

```sh
cargo run -p apsi-cli -- docs generate     # docs/domains, docs/traceability.csv, docs/STATUS.md, docs/UNITS.md
node tools/structure.mjs --write           # the 72 canonical directory READMEs
APSI_BLESS=1 cargo test -p apsi-contracts  # only when an ASPF contract changes on purpose
```

Tests fail when any of these drift from the code.

## Code style

- Rust: `cargo fmt` with the repository [rustfmt.toml](rustfmt.toml). Domain logic is pure: no I/O, no clocks, no globals; effects stay in `Engine`, the server and the CLI. Errors are typed (`ApsiError`, `ScienceError`, `ContractError`); no `unwrap` outside tests. `unsafe` is forbidden in the workspace.
- TypeScript: strict mode, standalone components, signals, no `any`. Every user-visible string goes through `i18n.ts` in both English and Hindi.
- Quantities carry units (see [docs/UNITS.md](docs/UNITS.md)); geometries carry a CRS (see [CRS policy](03_domain_model/crs-policy.md)). Never return NaN or a sentinel number for a missing value.
- Example and test data is synthetic and named as such (`synthetic-tenant`, `field_synthetic_1`).

## Tests expected with a change

| Change | Tests |
|---|---|
| Engine behaviour | a unit test next to the code, plus an integration test in `crates/apsi-core/tests/engine.rs` |
| New HTTP route | a test in `crates/apsi-server/tests/api.rs`, plus the route in [openapi.v1.yaml](10_backend_api/openapi.v1.yaml); a test fails if the route is missing |
| Parsing or validation | a property test (`proptest`) for "never panics, rejects malformed input" |
| Science function | known-answer tests against a cited reference and an input-rejection test |
| ASPF contract | positive and negative cases in `crates/apsi-contracts/tests/contracts.rs`; conformance must still reject the broken mocks |

## Branches and commits

- Branch from `main`: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`, `chore/<topic>`.
- Use Conventional Commits: `feat(engine): ...`, `fix(api): ...`, `docs: ...`. Reference feature ids when relevant (`APSI-F0123`).
- Add user-visible changes to [CHANGELOG.md](CHANGELOG.md) under *Unreleased*.
- Never commit secrets, tokens, real farm or personal data, or generated `target/`, `web/dist/` or `.apsi-data/`.
