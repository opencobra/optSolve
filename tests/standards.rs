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

/// Backend module directories, each with its own `ffi.rs` and `params.rs`.
const BACKENDS: [&str; 2] = ["src/gurobi/", "src/mosek/"];

#[test]
fn unsafe_and_extern_only_in_backend_modules() {
    for (path, text) in sources() {
        if BACKENDS.iter().any(|b| path.starts_with(b)) {
            continue;
        }
        for line in code_lines(&text) {
            assert!(
                !line.contains("unsafe ")
                    && !line.contains("unsafe{")
                    && !line.contains("extern \"C\""),
                "{path}: unsafe or extern outside the backend modules: {line}"
            );
        }
    }
}

#[test]
fn extern_c_declarations_only_in_ffi_module() {
    for (path, text) in sources() {
        if BACKENDS.iter().any(|b| path == format!("{b}ffi.rs")) {
            continue;
        }
        for line in code_lines(&text) {
            let t = line.trim_start();
            // Extern *blocks* (declarations) only; `unsafe extern "C" fn` definitions such as
            // callback trampolines are allowed in the backend modules.
            let is_block =
                t.starts_with("unsafe extern \"C\" {") || t.starts_with("extern \"C\" {");
            assert!(
                !is_block,
                "{path}: extern block outside a backend's ffi.rs: {line}"
            );
        }
    }
}

/// Vendor parameter setters per backend (constitution VII-G): only `params.rs` (and the
/// declarations in `ffi.rs`) may name them.
const SETTERS: [(&str, &[&str]); 2] = [
    (
        "src/gurobi/",
        &[
            "GRBsetintparam",
            "GRBsetdblparam",
            "GRBsetstrparam",
            "GRBsetparam",
        ],
    ),
    (
        "src/mosek/",
        &[
            "MSK_putintparam",
            "MSK_putdouparam",
            "MSK_putstrparam",
            "MSK_putnaintparam",
            "MSK_putnadouparam",
            "MSK_putnastrparam",
            "MSK_putparam",
        ],
    ),
];

#[test]
fn vendor_parameter_setters_only_in_params_module() {
    for (path, text) in sources() {
        for (dir, setters) in SETTERS {
            if path == format!("{dir}params.rs") || path == format!("{dir}ffi.rs") {
                continue;
            }
            for line in code_lines(&text) {
                for setter in setters {
                    // Whole identifiers only: MSK_putparam must not match MSK_putparamxyz.
                    let hit = line.match_indices(setter).any(|(i, _)| {
                        !line[i + setter.len()..]
                            .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
                    });
                    assert!(
                        !hit,
                        "{path}: {setter} outside {dir}params.rs (constitution VII-G): {line}"
                    );
                }
            }
        }
    }
}
