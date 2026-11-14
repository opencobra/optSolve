//! Build script: links the Gurobi shared library when the `gurobi` feature is enabled.
//!
//! The Gurobi installation is located at build time through `GUROBI_HOME` (never stored in
//! the repository). The library name is derived from the supported version declared in
//! `.specify/memory/project.yml` (`domain.solvers.gurobi`), so that value lives in one place.

use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.specify/memory/project.yml");
    println!("cargo:rerun-if-env-changed=GUROBI_HOME");
    if env::var_os("CARGO_FEATURE_GUROBI").is_none() {
        return;
    }
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR");
    let declaration = Path::new(&manifest_dir).join(".specify/memory/project.yml");
    let text = fs::read_to_string(&declaration)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", declaration.display()));
    let version = gurobi_version(&text).unwrap_or_else(|| {
        panic!(
            "domain.solvers.gurobi not found in {}",
            declaration.display()
        )
    });
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

/// Reads `gurobi: "x.y.z"` from the `domain.solvers` block of the project declaration.
fn gurobi_version(text: &str) -> Option<String> {
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
            if let Some(rest) = trimmed.strip_prefix("gurobi:") {
                return Some(rest.trim().trim_matches('"').to_string());
            }
            if !line.starts_with("    ") {
                in_solvers = false;
            }
        }
    }
    None
}
