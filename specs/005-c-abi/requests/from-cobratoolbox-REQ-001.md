# Request REQ-001 from the COBRA Toolbox fork

- **Origin**: the maintainer's fork of The COBRA Toolbox, feature
  `20261115-173718-runmat-testall-optsolver`, file
  `specs/20261115-173718-runmat-testall-optsolver/requests/REQ-001-c-interface.md` in that
  repository (located by the maintainer's checkout; not stored as a path here).
- **Received**: 2026-11-15, carried by a subagent the maintainer authorised.
- **Subject**: a C interface (header, static and shared library) over optSolver's Rust API for the
  COBRA MEX gateway `optsolver_solve`.
- **Answered by**: feature 005-c-abi (this directory). The request's text is quoted in `spec.md`,
  Input; the consumer's gateway contract is `contracts/optsolver-mex.md` of the same COBRA feature.
- **State**: open until feature 005 passes Gate 3; the hand-back to the COBRA session reports the
  header, libraries, flags and functions.
