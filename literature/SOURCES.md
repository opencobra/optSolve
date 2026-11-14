# Literature sources and licences

Downloaded 2026-11-14 from GitHub at the pinned commits below (constitution
principles IX-J and X). Each folder holds the source text of the published book as its
authors maintain it (mdBook Markdown, or Jekyll for the Omnibus) plus the
upstream licence files.

| Folder | Upstream | Path in repo | Commit | Licence | Published at |
|---|---|---|---|---|---|
| `rustonomicon/` | rust-lang/nomicon | `src/` | `b0bb569883a3e65ab19d4f1adf1b8dbc1629d0a6` | MIT OR Apache-2.0 | https://doc.rust-lang.org/nomicon/ |
| `rust-performance-book/` | nnethercote/perf-book | `src/` | `bc7cb0d1bca9561dda9effd0f3d86112eab3e948` | MIT OR Apache-2.0 | https://nnethercote.github.io/perf-book/ |
| `rust-api-guidelines/` | rust-lang/api-guidelines | `src/` | `97a0969cb07fe4cabb0eed8a56234053f47d83dc` | MIT OR Apache-2.0 | https://rust-lang.github.io/api-guidelines/ |
| `bindgen-user-guide/` | rust-lang/rust-bindgen | `book/src/` | `0d213bc3dbad03e27fe729b7fb1bb5c77cffda1d` | BSD-3-Clause | https://rust-lang.github.io/rust-bindgen/ |
| `unsafe-code-guidelines/` | rust-lang/unsafe-code-guidelines | `reference/src/` | `d809f91e62968913d808e53397fbcf7cea762cce` | MIT OR Apache-2.0 | https://rust-lang.github.io/unsafe-code-guidelines/ |
| `rust-edition-guide/` | rust-lang/edition-guide | `src/` | `ab8544aeed7b792984366aa122ac19bd47ad9a2f` | MIT OR Apache-2.0 | https://doc.rust-lang.org/edition-guide/ |
| `rust-design-patterns/` | rust-unofficial/patterns | `src/` | `36fdd17cdebfffd7af94479a4caa9375e40bde80` | MPL-2.0 | https://rust-unofficial.github.io/patterns/ |
| `criterion-user-guide/` | bheisler/criterion.rs | `book/src/` | `3dbc6c618acb48885066422d81d50729aa17b2b7` | MIT OR Apache-2.0 | https://bheisler.github.io/criterion.rs/book/ |
| `cargo-reference/` | rust-lang/cargo | `doc/book/src/reference/` (8 pages: build scripts, features, workspaces, profiles, manifest, rust-version) | `086b56d764fc390e0f4f333edbe79516efa6ed8a` | MIT OR Apache-2.0 | https://doc.rust-lang.org/cargo/reference/ |
| `rust-ffi-omnibus/` | shepmaster/rust-ffi-omnibus | `site/` | `a0ae88cfbc149df6642fcda299837f41818fd0e5` | Text CC BY-NC-SA 4.0; code MIT | http://jakegoulding.com/rust-ffi-omnibus/ |

## Most relevant to optSolver

- FFI to Gurobi/MOSEK: `rustonomicon/ffi.md`, `rust-ffi-omnibus/`,
  `bindgen-user-guide/`, `cargo-reference/build-scripts.md` (`links`, linking),
  `rust-edition-guide/rust-2024/unsafe-extern.md`.
- Zero-copy, memory and speed: `rust-performance-book/` (heap allocations,
  bounds checks, profiling, build configuration).
- `unsafe` correctness: `rustonomicon/`, `unsafe-code-guidelines/`.
- Public API: `rust-api-guidelines/`, `rust-design-patterns/` (FFI idioms,
  RAII).
- Backends as features: `cargo-reference/features.md`, `workspaces.md`.
- Benchmarks as acceptance criteria: `criterion-user-guide/`.

## Not downloaded (no redistributable source)

Read online or buy: *Rust for Rustaceans* (J. Gjengset), *Rust Atomics and
Locks* (M. Bos, readable free at https://marabos.nl/atomics/), *Effective Rust*
(D. Drysdale, https://www.lurklurk.org/effective-rust/; source repository not
public).

## Redistribution

All folders except `rust-ffi-omnibus/` are under licences that permit redistribution with
the licence files kept. The Omnibus text is licensed CC BY-NC-SA 4.0 (non-commercial), so it
stays local (constitution IX-J). Whether each folder is tracked is stated in `.gitignore`.
