//! Build script: links the vendor shared libraries of the enabled backends.
//!
//! - `gurobi`: located through `GUROBI_HOME`; library name derived from the supported
//!   version declared in `.specify/memory/project.yml` (`domain.solvers.gurobi`).
//! - `mosek`: `libmosek64` located through `MOSEK_BINDIR` (the MOSEK `bin` directory); the
//!   supported version is read from `domain.solvers.mosek`.
//!
//! Installation paths are never stored in the repository. Cargo allows one `links` key per
//! package, which the Gurobi library holds; MOSEK is linked with `rustc-link-lib` alone
//! (feature 002 research, R16).

use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.specify/memory/project.yml");
    println!("cargo:rerun-if-env-changed=GUROBI_HOME");
    println!("cargo:rerun-if-env-changed=MOSEK_BINDIR");
    let gurobi = env::var_os("CARGO_FEATURE_GUROBI").is_some();
    let mosek = env::var_os("CARGO_FEATURE_MOSEK").is_some();
    if !gurobi && !mosek {
        return;
    }
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR");
    let declaration = Path::new(&manifest_dir).join(".specify/memory/project.yml");
    let text = fs::read_to_string(&declaration)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", declaration.display()));
    let declared = |solver: &str| {
        solver_version(&text, solver).unwrap_or_else(|| {
            panic!(
                "domain.solvers.{solver} not found in {}",
                declaration.display()
            )
        })
    };
    if gurobi {
        let version = declared("gurobi");
        let mut parts = version.split('.');
        let major = parts.next().unwrap_or_default();
        let minor = parts.next().unwrap_or_default();
        let lib = format!("gurobi{major}{minor}");
        let home = env::var("GUROBI_HOME")
            .expect("the gurobi feature needs GUROBI_HOME pointing at the Gurobi installation");
        println!("cargo:rustc-link-search=native={home}/lib");
        println!("cargo:rustc-link-lib=dylib={lib}");
        println!("cargo:rustc-env=OPTSOLVER_GUROBI_DECLARED_VERSION={version}");
    }
    if mosek {
        let version = declared("mosek");
        let bindir = env::var("MOSEK_BINDIR").expect(
            "the mosek feature needs MOSEK_BINDIR pointing at the MOSEK bin directory \
             (tools/platform/<platform>/bin)",
        );
        println!("cargo:rustc-link-search=native={bindir}");
        println!("cargo:rustc-link-lib=dylib=mosek64");
        println!("cargo:rustc-env=OPTSOLVER_MOSEK_DECLARED_VERSION={version}");
    }
}

/// Reads `<solver>: "x.y.z"` from the `domain.solvers` block of the project declaration.
fn solver_version(text: &str, solver: &str) -> Option<String> {
    let key = format!("{solver}:");
    let mut in_solvers = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "solvers:" {
            in_solvers = true;
            continue;
        }
        if in_solvers {
            if let Some(rest) = trimmed.strip_prefix(&key) {
                return Some(rest.trim().trim_matches('"').to_string());
            }
            if !line.starts_with("    ") {
                in_solvers = false;
            }
        }
    }
    None
}
