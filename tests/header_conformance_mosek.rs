//! MOSEK header conformance test (feature 002, task T018; constitution VII-E).
//!
//! 1. Compiles `tests/header_conformance_mosek.c` against the installed `mosek.h`
//!    (`$MOSEK_BINDIR/../h`): every function ffi.rs declares exists with the parameter types
//!    spelled in the C file, every enumeration type used is 4 bytes, and every constant ffi.rs
//!    defines has the header's value.
//! 2. Compares each Rust declaration in `src/mosek/ffi.rs`, parameter by parameter, with the
//!    C types spelled in the C file (arity, width, pointer shape and constness), and checks
//!    that every function and constant in ffi.rs is named in the C file.
//!
//! The C compiler is found with the `cc` crate and invoked with `-fsyntax-only`, so nothing is
//! linked or run. Without `MOSEK_BINDIR` the header compile prints
//! "ignored: MOSEK_BINDIR not set" and returns; the Rust-side comparison always runs.

#![cfg(feature = "mosek")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[path = "common/ccompiler.rs"]
mod ccompiler;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn ffi_declarations_match_installed_header() {
    let Some(bindir) = env::var_os("MOSEK_BINDIR") else {
        println!("ignored: MOSEK_BINDIR not set");
        return;
    };
    let include = Path::new(&bindir).join("..").join("h");
    let header = include.join("mosek.h");
    assert!(
        header.is_file(),
        "MOSEK_BINDIR is set but {} does not exist",
        header.display()
    );
    let source = manifest_dir().join("tests/header_conformance_mosek.c");
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("header_conformance_mosek");
    fs::create_dir_all(&out_dir).expect("create temporary directory under target/");
    let mut cmd = ccompiler::syntax_only(&out_dir, &include, &source);
    let output = cmd.output().expect("run the C compiler");
    assert!(
        output.status.success(),
        "src/mosek/ffi.rs does not match {}:\n{}{}",
        header.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `pub fn MSK_x(a: T, b: U) -> i32;` declarations of ffi.rs: name → parameter types.
fn rust_functions(ffi: &str) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut rest = ffi;
    while let Some(i) = rest.find("pub fn MSK_") {
        let decl = &rest[i + "pub fn ".len()..];
        let open = decl.find('(').expect("parameter list");
        let close = decl.find(')').expect("end of parameter list");
        let name = decl[..open].to_string();
        let params = decl[open + 1..close]
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| {
                let (_, ty) = p.split_once(':').expect("name: type");
                ty.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .collect();
        out.insert(name, params);
        rest = &decl[close..];
    }
    out
}

/// `... (MSKAPI *const p_MSK_x)(T1, T2) = MSK_x;` lines of the C file: name → C types.
fn c_functions(c: &str) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for line in c.lines() {
        let Some(i) = line.find("*const p_") else {
            continue;
        };
        let decl = &line[i + "*const p_".len()..];
        let close = decl.find(')').expect("name");
        let name = decl[..close].to_string();
        let args = &decl[close + 1..];
        let args = args
            .strip_prefix('(')
            .and_then(|a| a.split_once(") ="))
            .expect("parameter list")
            .0;
        let types = args
            .split(',')
            .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        out.insert(name, types);
    }
    out
}

fn is_enum(c: &str) -> bool {
    // MOSEK enumeration typedefs end in `e` (MSKrescodee, MSKsoltypee, ...).
    c.starts_with("MSK") && c.ends_with('e') && c.chars().all(|ch| ch.is_ascii_alphanumeric())
}

/// Whether a Rust parameter type in ffi.rs has the width and shape of a header C type.
fn compatible(rust: &str, c: &str) -> bool {
    let int32 = |t: &str| t == "MSKint32t" || t == "MSKbooleant" || is_enum(t);
    match rust {
        "i32" => int32(c),
        "i64" => c == "MSKint64t",
        "f64" => c == "MSKrealt",
        "*mut i32" => c.strip_suffix(" *").is_some_and(int32),
        "*const i32" => c
            .strip_prefix("const ")
            .and_then(|t| t.strip_suffix(" *"))
            .is_some_and(int32),
        "*mut i64" => c == "MSKint64t *",
        "*const i64" => c == "const MSKint64t *",
        "*mut f64" => c == "MSKrealt *",
        "*const f64" => c == "const MSKrealt *",
        "*mut c_char" => c == "char *",
        "*const c_char" => c == "const char *",
        "*mut c_void" => c == "MSKuserhandle_t",
        "MSKenv_t" | "MSKtask_t" | "MSKcallbackfunc" | "MSKstreamfunc" => c == rust,
        "*mut MSKenv_t" => c == "MSKenv_t *",
        "*mut MSKtask_t" => c == "MSKtask_t *",
        _ => false,
    }
}

#[test]
fn rust_declarations_match_c_file() {
    let root = manifest_dir();
    let ffi = fs::read_to_string(root.join("src/mosek/ffi.rs")).expect("read src/mosek/ffi.rs");
    let c = fs::read_to_string(root.join("tests/header_conformance_mosek.c"))
        .expect("read tests/header_conformance_mosek.c");
    let rust = rust_functions(&ffi);
    let cfns = c_functions(&c);
    assert!(
        rust.len() >= 45,
        "expected about 45 functions, found {}",
        rust.len()
    );
    for (name, params) in &rust {
        let ctypes = cfns.get(name).unwrap_or_else(|| {
            panic!("{name} is declared in ffi.rs but not checked in the C file")
        });
        assert_eq!(
            params.len(),
            ctypes.len(),
            "{name}: {} Rust parameters, {} in the header",
            params.len(),
            ctypes.len()
        );
        for (k, (r, ct)) in params.iter().zip(ctypes).enumerate() {
            assert!(
                compatible(r, ct),
                "{name}, parameter {k}: Rust `{r}` does not match header `{ct}`"
            );
        }
    }
    for name in cfns.keys() {
        assert!(
            rust.contains_key(name),
            "{name} checked in the C file but not in ffi.rs"
        );
    }
    for line in ffi.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("pub const MSK_") {
            let name: String = rest
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect();
            assert!(
                c.contains(&format!("_Static_assert(MSK_{name} == ")),
                "MSK_{name} is defined in ffi.rs but not asserted in the C file"
            );
        }
    }
}
