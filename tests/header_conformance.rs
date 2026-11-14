//! Header conformance test (task T037; research R12; constitution VII-E).
//!
//! Compiles `tests/header_conformance.c` against the installed `$GUROBI_HOME/include/gurobi_c.h`.
//! That C file takes the address of every function declared in `src/gurobi/ffi.rs` into a
//! pointer of the type ffi.rs declares, and `_Static_assert`s every constant ffi.rs defines, so
//! any mismatch between ffi.rs and the header fails compilation and therefore this test. A
//! second test checks that every `GRB…` function and `GRB_…` constant in ffi.rs is named in the
//! C file, so the two lists cannot drift apart silently.
//!
//! The C compiler is found with the `cc` crate (dev-dependency), and invoked with
//! `-fsyntax-only`, so nothing is linked or run. Without `GUROBI_HOME` the header test prints
//! "ignored: GUROBI_HOME not set" and returns.

#![cfg(feature = "gurobi")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Target triple for `cc`. Integration tests run on the host, and Cargo does not pass
/// `TARGET` to test binaries, so the triple is reconstructed for the supported 64-bit targets.
fn host_triple() -> &'static str {
    if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        "x86_64-pc-windows-msvc"
    } else {
        panic!("header conformance test: unsupported host target")
    }
}

#[test]
fn ffi_declarations_match_installed_header() {
    let Some(home) = env::var_os("GUROBI_HOME") else {
        println!("ignored: GUROBI_HOME not set");
        return;
    };
    let include = Path::new(&home).join("include");
    let header = include.join("gurobi_c.h");
    assert!(
        header.is_file(),
        "GUROBI_HOME is set but {} does not exist",
        header.display()
    );

    let source = manifest_dir().join("tests/header_conformance.c");
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("header_conformance");
    fs::create_dir_all(&out_dir).expect("create temporary directory under target/");

    let triple = host_triple();
    let compiler = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .emit_rerun_if_env_changed(false)
        .target(triple)
        .host(triple)
        .opt_level(0)
        .debug(false)
        .out_dir(&out_dir)
        .try_get_compiler()
        .expect("a C compiler found by the cc crate");

    let mut cmd = compiler.to_command();
    if compiler.is_like_msvc() {
        cmd.arg("/Zs").arg("/std:c11").arg("/WX");
    } else {
        cmd.arg("-fsyntax-only")
            .arg("-std=c11")
            .arg("-Werror=incompatible-pointer-types")
            .arg("-Werror");
    }
    cmd.arg("-I")
        .arg(&include)
        .arg(&source)
        .current_dir(&out_dir);
    let output = cmd.output().expect("run the C compiler");
    assert!(
        output.status.success(),
        "src/gurobi/ffi.rs does not match {}:\n{}{}",
        header.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Identifiers `prefix…` that follow `keyword` in Rust source, for example every name after
/// `pub fn ` that starts with `GRB`.
fn names_after(text: &str, keyword: &str, prefix: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix(keyword))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|name| name.starts_with(prefix))
        .collect()
}

#[test]
fn every_ffi_declaration_is_checked_in_c_file() {
    let root = manifest_dir();
    let ffi = fs::read_to_string(root.join("src/gurobi/ffi.rs")).expect("read src/gurobi/ffi.rs");
    let c = fs::read_to_string(root.join("tests/header_conformance.c"))
        .expect("read tests/header_conformance.c");

    let functions = names_after(&ffi, "pub fn ", "GRB");
    let constants = names_after(&ffi, "pub const ", "GRB_");
    assert!(
        functions.len() >= 25,
        "expected about 25 functions, found {functions:?}"
    );

    for f in &functions {
        let pattern = format!("p_{f})");
        assert!(
            c.contains(&pattern),
            "{f} is declared in ffi.rs but not checked in the C file"
        );
    }
    for k in &constants {
        let pattern = format!("_Static_assert({k} ");
        assert!(
            c.contains(&pattern),
            "{k} is defined in ffi.rs but not asserted in the C file"
        );
    }
}
