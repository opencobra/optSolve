//! Source standards checks (constitution VII-A, VII-G; tasks T010b, T010c). These search the
//! sources only; clippy runs in the in-gate checks (T050).

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read_to_string(&p).expect("readable source file"))
        })
        .collect()
}

/// Lines that are code, not comments.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| {
        let t = l.trim_start();
        !(t.starts_with("//") || t.starts_with("*") || t.starts_with("/*"))
    })
}

#[test]
fn unsafe_and_extern_only_in_gurobi_modules() {
    for (path, text) in sources() {
        if path.starts_with("src/gurobi/") {
            continue;
        }
        for line in code_lines(&text) {
            assert!(
                !line.contains("unsafe ")
                    && !line.contains("unsafe{")
                    && !line.contains("extern \"C\""),
                "{path}: unsafe or extern outside src/gurobi/: {line}"
            );
        }
    }
}

#[test]
fn extern_c_declarations_only_in_ffi_module() {
    for (path, text) in sources() {
        if path == "src/gurobi/ffi.rs" {
            continue;
        }
        for line in code_lines(&text) {
            let t = line.trim_start();
            // Extern *blocks* (declarations) only; `unsafe extern "C" fn` definitions such as
            // callback trampolines are allowed in the gurobi modules.
            let is_block =
                t.starts_with("unsafe extern \"C\" {") || t.starts_with("extern \"C\" {");
            assert!(
                !is_block,
                "{path}: extern block outside src/gurobi/ffi.rs: {line}"
            );
        }
    }
}

#[test]
fn vendor_parameter_setters_only_in_params_module() {
    for (path, text) in sources() {
        if path == "src/gurobi/params.rs" || path == "src/gurobi/ffi.rs" {
            continue;
        }
        for line in code_lines(&text) {
            for setter in [
                "GRBsetintparam",
                "GRBsetdblparam",
                "GRBsetstrparam",
                "GRBsetparam",
            ] {
                assert!(
                    !line.contains(setter),
                    "{path}: {setter} outside src/gurobi/params.rs (constitution VII-G): {line}"
                );
            }
        }
    }
}
