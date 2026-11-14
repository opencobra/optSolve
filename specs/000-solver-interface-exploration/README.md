# Solver interface exploration (design inbox)

**Created**: 2026-11-14
**Status**: Exploration. This is not a feature specification. It records the
design discussion that led to this repository, so that SpecKit features
(`/speckit-constitution`, then `/speckit-specify` for feature 001 onwards) can
draw on it.

## Purpose

optSolver is a Rust library for solving **large** optimisation problems that
are already in matrix form (sparse constraint matrix, bounds, objective)
through native solver interfaces: Gurobi and MOSEK first, then other solvers
that offer a Rust or C interface. The goals are, in order:

1. **Speed and memory.** Pass caller-owned compressed sparse column (CSC)
   buffers to each solver's bulk C load routine with no copy of the values or
   column pointers, and no intermediate modelling layer.
2. **Independence.** Usable by any Rust program. No dependency on RunMat or
   any particular sparse-matrix crate.
3. **RunMat compatibility at no cost.** A thin adapter (living in or beside
   RunMat, not here) exposes the library as RunMat builtins `gurobi` and
   `mosekopt` so that the COBRA Toolbox runs unchanged under RunMat.

## Files

- [`design-notes.md`](design-notes.md): the discussion, the options weighed,
  the survey of existing Rust crates, the detailed reading of `oximo`, and the
  resulting design.
- [`open-questions.md`](open-questions.md): what is not yet decided or
  verified, and the suggested first features.

## Origin

The discussion took place on 2026-11-12 to 2026-11-14 while getting the COBRA
Toolbox (`opencobra/cobratoolbox`) to run under RunMat (`../runmat`, branch
`feature/clean-room-matlab-builtins`). COBRA's solver layer calls `gurobi(...)`
and `mosekopt(...)`, and neither works under RunMat today.
