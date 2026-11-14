# Repository folder organisation

Tier 1 companion to the constitution (Principles IX and XIII-A). This is the single statement
of what goes where. It originates no rule; the invariants are in the constitution. Locations
are declared as parameters in `.specify/memory/project.yml`; this document names their roles.
Whether a path is tracked is stated in `.gitignore` only.

## Placement table

| Role | Destination | Notes |
|---|---|---|
| Project metadata and configuration | repository root | `Cargo.toml`, `Cargo.lock`, `build.rs`, `CLAUDE.md`, `AGENTS.md`, `README.md`, licence files, `.gitignore`, `.gitattributes`. Nothing else. |
| Library source | `src/` | The single crate `optsolver`; backends are modules behind features (specified, not yet created). |
| Integration tests | `tests/` | Unit tests sit beside the code they test. |
| Benchmarks (source) | `benches/` | Benchmark code is source. |
| Test fixtures (small) | `tests/fixtures/` | Inputs, not generated output. |
| Large test models | not in Git object history | Fetched or located at run time; provenance recorded (X). IX-G. |
| Generated and run output | `results/<name>/` | One home per feature, exploration or session, keyed by `<name>`; run bundles at `results/<name>/runs/<UTC-timestamp>-<label>/` with that run's logs inside. IX-C, IX-E. |
| Feature artifacts | `specs/<feature>/` | Spec Kit Tier 2. Exploration at `specs/<YYYYMMDD-HHMMSS>-<slug>/` until go (XI). |
| Implementation receipts | `specs/<feature>/agent-runs/<UTC-timestamp>-<name>/` | Constitution, Implementation Receipt Ledger. |
| Architecture model and decisions | `docs/architecture/`, `docs/architecture/decisions/` | XIII. |
| Descriptive documentation | `docs/` | Tier 3; originates no rule. |
| Downloaded external literature | `literature/<source>/` | Upstream text and licence files only; index in `literature/SOURCES.md` (IX-J). |
| Spec Kit governance | `.specify/` | Constitution, templates, skills, extensions, scripts. |
| Agent pointer files | `.claude/`, `.agents/` | Tier 5. |
| Scratch | `tmp/` | Disposable; nothing is read from it by tracked code. |
| Retired content | `old/` at the level of what it replaces | Read-only (IX-H). |

## Never in the repository

Vendor headers, libraries, licence files and keys; bindings generated from vendor headers
unless the vendor's licence permits (constitution VII-E, X). Vendor installations are located
at run time through environment variables read by the build script.

## Retired destinations

None yet.
