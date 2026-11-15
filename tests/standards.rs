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

// ---------------------------------------------------------------------------------------------
// Feature 003 T007: the interface document names every public item (research R10, FR-004).

/// Source files of the solver-independent public interface.
const INTERFACE_FILES: &[&str] = &[
    "src/lib.rs",
    "src/backend.rs",
    "src/conic.rs",
    "src/error.rs",
    "src/options.rs",
    "src/options/check.rs",
    "src/problem.rs",
    "src/problem/convert.rs",
    "src/result.rs",
    "src/verify.rs",
];

/// Public identifiers of one source file: `pub` modules, functions, types, traits, constants,
/// public struct fields, enum variants of public enums and methods of public traits; not
/// `pub(crate)`, not items marked `#[doc(hidden)]`, not test modules.
fn public_identifiers(text: &str) -> Vec<String> {
    fn ident(s: &str) -> String {
        s.chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    }
    let mut out = Vec::new();
    let mut hidden = false;
    // Inside a public enum or trait: the brace depth at which its members sit.
    let mut member_depth: Option<(usize, bool)> = None; // (depth, is_enum)
    let mut depth = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("mod tests") || t == "#[cfg(test)]" {
            break;
        }
        if t.starts_with("//") {
            continue;
        }
        if t.starts_with("#[doc(hidden)]") {
            hidden = true;
            continue;
        }
        if let Some((d, is_enum)) = member_depth {
            if depth == d && !t.starts_with('#') {
                let name = if is_enum {
                    ident(t)
                } else {
                    t.strip_prefix("fn ").map(ident).unwrap_or_default()
                };
                if name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    out.push(name);
                }
            }
        }
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        let mut item = None;
        for kw in [
            "pub fn ",
            "pub struct ",
            "pub enum ",
            "pub trait ",
            "pub const ",
            "pub type ",
            "pub mod ",
        ] {
            if let Some(rest) = t.strip_prefix(kw) {
                item = Some((kw, ident(rest)));
            }
        }
        if let Some((kw, name)) = item {
            if !hidden && !name.is_empty() {
                out.push(name);
                if (kw == "pub enum " || kw == "pub trait ") && opens > closes {
                    member_depth = Some((depth + 1, kw == "pub enum "));
                }
            }
            hidden = false;
        } else if let Some(rest) = t.strip_prefix("pub ") {
            // A public struct field: `pub name: Type`.
            let name = ident(rest);
            if !name.is_empty() && rest[name.len()..].starts_with(':') && !hidden {
                out.push(name);
            }
            hidden = false;
        }
        depth = depth + opens - closes.min(depth + opens);
        if let Some((d, _)) = member_depth {
            if depth < d {
                member_depth = None;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Identifiers absent from `doc` as a whole word inside a backtick span.
fn missing_from_document(doc: &str, identifiers: &[String]) -> Vec<String> {
    let spans: Vec<&str> = doc.split('`').skip(1).step_by(2).collect();
    identifiers
        .iter()
        .filter(|id| {
            !spans.iter().any(|span| {
                span.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|w| w == id.as_str())
            })
        })
        .cloned()
        .collect()
}

#[test]
fn interface_document_names_every_public_item() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = fs::read_to_string(root.join("docs/llm-interface.md"))
        .expect("docs/llm-interface.md exists (feature 003)");
    let mut identifiers = vec!["Env".to_string(), "gurobi".to_string(), "mosek".to_string()];
    for f in INTERFACE_FILES {
        let text = fs::read_to_string(root.join(f)).expect("readable source file");
        identifiers.extend(public_identifiers(&text));
    }
    identifiers.sort();
    identifiers.dedup();
    assert!(
        identifiers.len() > 100,
        "found only {} identifiers",
        identifiers.len()
    );
    let missing = missing_from_document(&doc, &identifiers);
    assert!(
        missing.is_empty(),
        "public items not named in docs/llm-interface.md (in backticks): {missing:?}"
    );
}

#[test]
fn interface_document_check_catches_omissions() {
    // Guard self-test (constitution XV-C): the scanner finds items of every kind, and the
    // check reports an identifier the document does not name.
    let src = "pub struct S {\n    pub field: u8,\n    hidden: u8,\n}\npub enum E {\n    A,\n    B(u8),\n}\n\
               pub trait T {\n    fn method(&self);\n}\npub(crate) fn private() {}\n\
               #[doc(hidden)]\npub fn secret() {}\npub fn open() {}\n";
    let ids = public_identifiers(src);
    assert_eq!(ids, ["A", "B", "E", "S", "T", "field", "method", "open"]);
    let doc = "Use `S` with `S::field`, `E::A`, `E::B`, the trait method `method` and `open()`.";
    assert_eq!(missing_from_document(doc, &ids), ["T"]);
    let doc = format!("{doc} The trait `T`.");
    assert!(missing_from_document(&doc, &ids).is_empty());
}
