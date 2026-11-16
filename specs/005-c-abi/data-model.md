# Data Model: C Interface (feature 005)

Symbols follow the constitution's Notation Convention; no new symbol is introduced. The C
structures are defined in `contracts/c-interface.md`; this table maps each onto the Rust API.

| C | Rust | Rule |
|---|---|---|
| `optsolver_problem.A` (`optsolver_csc`, `size_t` row indices) | `CscRef<usize>::new(m, n, col_ptrs, row_indices, values)` | borrowed; nnz = `col_ptrs[n]` |
| `A_row_indices_i32` non-null | `CscRef<i32>::new(...)` → `ProblemRef::I32` | borrowed; `A.row_indices` must be NULL |
| `c`, `l`, `u` | `Problem::new(.., c, sense, rows, l, u)` | length n, borrowed |
| `sense` 1 / −1 | `Sense::Minimise` / `Sense::Maximise` | other values invalid |
| `gamma` | `with_gamma` | non-finite invalid (Rust rule) |
| `row_sense`, `b` | `RowBounds::SenseRhs { sense: &[u8], b }` | `char` read as `u8`, borrowed |
| `b_l`, `b_u` | `RowBounds::Ranges { b_l, b_u }` | exactly one form |
| `Q` | `with_Q(CscRef<usize>)` | lower triangle (Rust rule) |
| `F`, `g`, `cones` | `ConicRef::new(F, g, &[Cone])`, `with_conic` | cones → `Vec<Cone>` (linear in `num_cones`) |
| `optsolver_cone {kind 1..9, dim, alpha}` | `Cone::Quadratic(dim)` … `Cone::PositiveSemidefinite { order: dim }`; power cones `alpha` copied into the variant | exponential cones require dim 3 or 0 (dim ignored when 0) |
| `optsolver_options` fields | `Options` fields of the same names | NaN / `OPTSOLVER_UNSET` → `None` |
| `threads` ≥ 0 | `Some(u32)` | above `u32::MAX` invalid |
| `random_seed` with `random_seed_set` | `Some(i32)` | outside `i32` invalid |
| `memory_limit_gb` | `Options::memory_limit_gb` | new Rust field (FR-011) |
| `raw[i]` | `Options::raw` `(String, ParamValue)` | name UTF-8 without NUL; string value likewise |
| `optsolver_env` | `enum { Gurobi(gurobi::Env), Mosek(mosek::Env) }` boxed | freed by `optsolver_env_free` |
| `optsolver_result` | `Solution` plus NUL-terminated name copies | arrays read in place |
| `optsolver_buffers` | `SolutionBuffers` | NaN-filled before the solve |
| `optsolver_caps` | `Capabilities` plus usability and reason | names NUL-terminated |

## Status and enum codes

Codes are fixed by the contract and translated in one function each in `src/capi/` (total over
the Rust enums; a future Rust variant added under `#[non_exhaustive]` maps to `UNKNOWN` or a new
constant in a minor version).

## Ownership

Inputs: borrowed for the call only. Library-allocated: `optsolver_env`, `optsolver_result`,
`optsolver_caps` — each freed by its own `_free`, NULL a no-op. Strings returned by accessors
belong to their object; `optsolver_last_error_message` returns a caller-owned copy freed with `optsolver_string_free`.
