//! C compiler discovery for tests: one copy, used by the header conformance tests and the
//! C-interface tests. Included with `#[path]` by tests that do not compile the whole shared
//! module.

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

/// Target triple for `cc`. Integration tests run on the host, and Cargo does not pass `TARGET`
/// to test binaries, so the triple is reconstructed for the supported 64-bit targets.
pub fn host_triple() -> &'static str {
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
        panic!("C compiler discovery: unsupported host target")
    }
}

/// The host C compiler found by the `cc` crate, with `out_dir` as its working directory.
pub fn compiler(out_dir: &Path) -> cc::Tool {
    let triple = host_triple();
    cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .emit_rerun_if_env_changed(false)
        .target(triple)
        .host(triple)
        .opt_level(0)
        .debug(false)
        .out_dir(out_dir)
        .try_get_compiler()
        .expect("a C compiler found by the cc crate")
}

/// A syntax-only compile of `source` with `-I include`, warnings as errors.
pub fn syntax_only(out_dir: &Path, include: &Path, source: &Path) -> Command {
    let tool = compiler(out_dir);
    let mut cmd = tool.to_command();
    if tool.is_like_msvc() {
        cmd.arg("/Zs").arg("/std:c11").arg("/WX");
    } else {
        cmd.arg("-fsyntax-only")
            .arg("-std=c11")
            .arg("-Werror=incompatible-pointer-types")
            .arg("-Werror");
    }
    cmd.arg("-I").arg(include).arg(source).current_dir(out_dir);
    cmd
}

// History
//
// Feature 005 (task T003) extracted this module from the two copies formerly in
// tests/header_conformance.rs and tests/header_conformance_mosek.rs.
