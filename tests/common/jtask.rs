//! A reader for MOSEK JTASK files, the format of the CLP fixtures (task T008; maintainer
//! decision 2026-11-15, see docs/test-corpus.md). Test-only code; no dependency: it reuses the
//! JSON reader of [`super::fixtures`].
//!
//! JTASK is MOSEK's JSON task format (https://docs.mosek.com/latest/capi/json-format.html). The
//! reader accepts what MOSEK 11.2.0 writes for a continuous conic problem: `Task/INFO` with the
//! counts, and in `Task/data` the sections `var`, `con`, `objective`, `A`, `AFE`, `domains` and
//! `ACC`. `Task/parameters` and other top-level keys are ignored. Anything the problem type
//! cannot hold is rejected with a message, never dropped: unknown `Task/data` or `objective`
//! keys (for example a quadratic objective), a nonzero count of cones, semidefinite variables,
//! symmetric matrices or disjunctive constraints, and non-continuous variables.
//!
//! Conventions:
//! * bound keys decide infinities, never the stored values: `fr` gives (−∞, +∞), `lo` (bl, +∞),
//!   `up` (−∞, bu), `fx` (bl, bl) with bl = bu required, `ra` (bl, bu); only the values a key
//!   uses must be finite;
//! * A (m × n) and F (k × n) are 0-based triplets in any order, converted to CSC with row
//!   indices sorted within each column; a repeated (row, column) pair is rejected;
//! * c and g are sparse vectors; an absent or empty c or g is zero;
//! * domains map to [`Cone`]s: `quad` d, `rquad` d, `pexp`, `dexp`, `rplus` d, `rzero` d,
//!   `ppow` d [α], `dpow` d [α] (encoding confirmed in MOSEK's JSON format specification).
//!   Other types (`svecpsd`, `r`, `rminus`, geometric-mean cones) are rejected by name;
//! * the cones of K cover consecutive rows of Fx + g in order, so the concatenated `afeidx`
//!   lists of the ACCs must be exactly 0, 1, …, k − 1, and the dimension of each ACC's domain
//!   must equal the length of its `afeidx` list;
//! * an ACC constant vector `b` is accepted only when every entry is zero (MOSEK's
//!   F x + g − b ∈ D would otherwise need folding into g; no fixture has one).

#![allow(non_snake_case)] // A and F are matrices (Notation Convention).

use super::fixtures::{
    Json, Provenance, clp_instances, clp_provenance_path, fixtures_dir, parse_json, sha256_hex,
};
use super::{Trial, ensure, fail};
use optsolver::{Cone, ConicRef, CscRef, Problem, RowBounds, Sense};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type Obj = BTreeMap<String, Json>;

/// A CSC matrix as (column pointers, row indices, values).
type Csc = (Vec<usize>, Vec<usize>, Vec<f64>);

/// A continuous conic problem read from a JTASK file, owning its arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct JtaskProblem {
    /// `Task/name` (empty when absent).
    pub name: String,
    /// Objective sense.
    pub sense: Sense,
    /// Number of rows of A.
    pub m: usize,
    /// Number of columns of A.
    pub n: usize,
    /// Column pointers of A, length n + 1.
    pub A_col_ptrs: Vec<usize>,
    /// Row indices of A, sorted within each column.
    pub A_row_indices: Vec<usize>,
    /// Values of A.
    pub A_values: Vec<f64>,
    /// Objective coefficients c, length n.
    pub c: Vec<f64>,
    /// Objective constant γ (`cfix`).
    pub gamma: f64,
    /// Lower row bounds b_l, length m.
    pub b_l: Vec<f64>,
    /// Upper row bounds b_u, length m.
    pub b_u: Vec<f64>,
    /// Variable lower bounds l, length n.
    pub l: Vec<f64>,
    /// Variable upper bounds u, length n.
    pub u: Vec<f64>,
    /// Number of affine conic rows k.
    pub k: usize,
    /// Column pointers of F, length n + 1.
    pub F_col_ptrs: Vec<usize>,
    /// Row indices of F, sorted within each column.
    pub F_row_indices: Vec<usize>,
    /// Values of F.
    pub F_values: Vec<f64>,
    /// Conic offset g, length k.
    pub g: Vec<f64>,
    /// The cones of K, in row order.
    pub cones: Vec<Cone>,
}

impl JtaskProblem {
    /// Number of stored entries of A.
    pub fn nnz_A(&self) -> usize {
        self.A_values.len()
    }

    /// Number of stored entries of F.
    pub fn nnz_F(&self) -> usize {
        self.F_values.len()
    }

    /// The problem as an `optsolver::Problem` with range rows b_l ≤ Ax ≤ b_u, γ and, when
    /// k > 0, the conic constraints Fx + g ∈ K.
    pub fn problem(&self) -> Result<Problem<'_, usize>, String> {
        let e = |e: optsolver::Error| e.to_string();
        let A = CscRef::new(
            self.m,
            self.n,
            &self.A_col_ptrs,
            &self.A_row_indices,
            &self.A_values,
        )
        .map_err(e)?;
        let rows = RowBounds::Ranges {
            b_l: &self.b_l,
            b_u: &self.b_u,
        };
        let p = Problem::new(A, &self.c, self.sense, rows, &self.l, &self.u)
            .and_then(|p| p.with_gamma(self.gamma))
            .map_err(e)?;
        if self.k == 0 {
            return Ok(p);
        }
        let F = CscRef::new(
            self.k,
            self.n,
            &self.F_col_ptrs,
            &self.F_row_indices,
            &self.F_values,
        )
        .map_err(e)?;
        let conic = ConicRef::new(F, &self.g, &self.cones).map_err(e)?;
        p.with_conic(conic).map_err(e)
    }
}

/// The JTASK file of a CLP corpus instance (`fixtures`/clp).
pub fn jtask_path(instance: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?
        .join("clp")
        .join(format!("{instance}.jtask")))
}

/// Reads a JTASK file.
pub fn read_jtask_file(path: &Path) -> Result<JtaskProblem, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    read_jtask_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a JTASK document.
pub fn read_jtask_str(text: &str) -> Result<JtaskProblem, String> {
    let Json::Object(top) = parse_json(text)? else {
        return Err("JTASK: the document is not a JSON object".into());
    };
    let name = match top.get("Task/name") {
        None => String::new(),
        Some(Json::String(s)) => s.clone(),
        Some(other) => return Err(format!("Task/name: expected a string, got {other:?}")),
    };
    let info = obj(key(&top, "Task/INFO", "the document")?, "Task/INFO")?;
    let count = |k: &str| index(key(info, k, "Task/INFO")?, &format!("Task/INFO.{k}"));
    let n = count("numvar")?;
    let m = count("numcon")?;
    let numanz = count("numanz")?;
    let k = count("numafe")?;
    let numfnz = count("numfnz")?;
    let numacc = count("numacc")?;
    let numdom = count("numdom")?;
    for unsupported in ["numcone", "numbarvar", "numsymmat", "numdjc"] {
        if let Some(v) = info.get(unsupported) {
            let v = index(v, &format!("Task/INFO.{unsupported}"))?;
            if v != 0 {
                return Err(format!(
                    "Task/INFO.{unsupported} = {v}: not supported (must be 0)"
                ));
            }
        }
    }

    let data = obj(key(&top, "Task/data", "the document")?, "Task/data")?;

    // Variables.
    let var = obj(key(data, "var", "Task/data")?, "var")?;
    let (l, u) = bounds(var, n, "var")?;
    if let Some(t) = var.get("type") {
        let types = strings(t, "var.type")?;
        check_len(types.len(), n, "var.type", "Task/INFO.numvar")?;
        if let Some((j, t)) = types.iter().enumerate().find(|(_, t)| **t != "cont") {
            return Err(format!(
                "var.type[{j}] = {t:?}: only continuous variables (\"cont\") are supported"
            ));
        }
    }
    reject_unknown(var, &["name", "bk", "bl", "bu", "type"], "var")?;

    // Rows.
    let (b_l, b_u) = match data.get("con") {
        Some(con) => {
            let con = obj(con, "con")?;
            reject_unknown(con, &["name", "bk", "bl", "bu"], "con")?;
            bounds(con, m, "con")?
        }
        None if m == 0 => (Vec::new(), Vec::new()),
        None => return Err(missing("con", "Task/data")),
    };

    // Objective.
    let objective = obj(key(data, "objective", "Task/data")?, "objective")?;
    let sense = match key(objective, "sense", "objective")? {
        Json::String(s) if s == "min" => Sense::Minimise,
        Json::String(s) if s == "max" => Sense::Maximise,
        other => {
            return Err(format!(
                "objective.sense: expected \"min\" or \"max\", got {other:?}"
            ));
        }
    };
    let c = match objective.get("c") {
        None => vec![0.0; n],
        Some(c) => sparse_vector(obj(c, "objective.c")?, "subj", n, "objective.c")?,
    };
    let gamma = match objective.get("cfix") {
        None => 0.0,
        Some(v) => finite(number(v, "objective.cfix")?, "objective.cfix")?,
    };
    reject_unknown(objective, &["sense", "name", "c", "cfix"], "objective")?;

    // A.
    let (A_col_ptrs, A_row_indices, A_values) = match data.get("A") {
        Some(a) => {
            let a = obj(a, "A")?;
            reject_unknown(a, &["subi", "subj", "val"], "A")?;
            triplets_to_csc(a, m, n, numanz, "A", "Task/INFO.numanz")?
        }
        None if numanz == 0 => (vec![0; n + 1], Vec::new(), Vec::new()),
        None => return Err(missing("A", "Task/data")),
    };

    // F and g.
    let (F_col_ptrs, F_row_indices, F_values, g) = match data.get("AFE") {
        Some(afe) => {
            let afe = obj(afe, "AFE")?;
            reject_unknown(afe, &["numafe", "F", "g"], "AFE")?;
            let numafe = index(key(afe, "numafe", "AFE")?, "AFE.numafe")?;
            if numafe != k {
                return Err(format!(
                    "AFE.numafe = {numafe} does not match Task/INFO.numafe = {k}"
                ));
            }
            let (p, i, v) = match afe.get("F") {
                Some(f) => {
                    let f = obj(f, "AFE.F")?;
                    reject_unknown(f, &["subi", "subj", "val"], "AFE.F")?;
                    triplets_to_csc(f, k, n, numfnz, "AFE.F", "Task/INFO.numfnz")?
                }
                None if numfnz == 0 => (vec![0; n + 1], Vec::new(), Vec::new()),
                None => return Err(missing("F", "AFE")),
            };
            let g = match afe.get("g") {
                None => vec![0.0; k],
                Some(g) => sparse_vector(obj(g, "AFE.g")?, "subi", k, "AFE.g")?,
            };
            (p, i, v, g)
        }
        None if k == 0 => (vec![0; n + 1], Vec::new(), Vec::new(), Vec::new()),
        None => return Err(missing("AFE", "Task/data")),
    };

    // Domains.
    let domains: Vec<Cone> = match data.get("domains") {
        Some(d) => {
            let d = obj(d, "domains")?;
            reject_unknown(d, &["name", "type"], "domains")?;
            let types = array(key(d, "type", "domains")?, "domains.type")?;
            check_len(types.len(), numdom, "domains.type", "Task/INFO.numdom")?;
            types
                .iter()
                .enumerate()
                .map(|(i, t)| domain(t, &format!("domains.type[{i}]")))
                .collect::<Result<_, _>>()?
        }
        None if numdom == 0 => Vec::new(),
        None => return Err(missing("domains", "Task/data")),
    };

    // Affine conic constraints.
    let cones = match data.get("ACC") {
        Some(acc) => {
            let acc = obj(acc, "ACC")?;
            reject_unknown(acc, &["name", "domain", "afeidx", "b"], "ACC")?;
            accs(acc, numacc, &domains, k)?
        }
        None if numacc == 0 => Vec::new(),
        None => return Err(missing("ACC", "Task/data")),
    };
    if numacc == 0 && k != 0 {
        return Err(format!(
            "{k} affine rows but no ACC: every row of F must belong to a cone"
        ));
    }

    reject_unknown(
        data,
        &["var", "con", "objective", "A", "AFE", "domains", "ACC"],
        "Task/data",
    )?;

    Ok(JtaskProblem {
        name,
        sense,
        m,
        n,
        A_col_ptrs,
        A_row_indices,
        A_values,
        c,
        gamma,
        b_l,
        b_u,
        l,
        u,
        k,
        F_col_ptrs,
        F_row_indices,
        F_values,
        g,
        cones,
    })
}

// ---------------------------------------------------------------------------------------------
// Sections

/// Bounds from `bk`, `bl`, `bu` of `var` or `con` (see the module conventions).
fn bounds(o: &Obj, len: usize, what: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    let count = if what == "var" {
        "Task/INFO.numvar"
    } else {
        "Task/INFO.numcon"
    };
    let bk = strings(key(o, "bk", what)?, &format!("{what}.bk"))?;
    let bl = numbers(key(o, "bl", what)?, &format!("{what}.bl"))?;
    let bu = numbers(key(o, "bu", what)?, &format!("{what}.bu"))?;
    check_len(bk.len(), len, &format!("{what}.bk"), count)?;
    check_len(bl.len(), len, &format!("{what}.bl"), count)?;
    check_len(bu.len(), len, &format!("{what}.bu"), count)?;
    let mut lo = Vec::with_capacity(len);
    let mut hi = Vec::with_capacity(len);
    for (i, bkey) in bk.iter().enumerate() {
        let fin = |v: f64, side: &str| finite(v, &format!("{what}.{side}[{i}] (bound key {bkey})"));
        let (a, b) = match *bkey {
            "fr" => (f64::NEG_INFINITY, f64::INFINITY),
            "lo" => (fin(bl[i], "bl")?, f64::INFINITY),
            "up" => (f64::NEG_INFINITY, fin(bu[i], "bu")?),
            "fx" => {
                let v = fin(bl[i], "bl")?;
                if bu[i] != v {
                    return Err(format!(
                        "{what}[{i}]: bound key fx with bl = {v} but bu = {}",
                        bu[i]
                    ));
                }
                (v, v)
            }
            "ra" => (fin(bl[i], "bl")?, fin(bu[i], "bu")?),
            other => {
                return Err(format!(
                    "{what}.bk[{i}] = {other:?}: unknown bound key (expected fr, lo, up, fx or ra)"
                ));
            }
        };
        lo.push(a);
        hi.push(b);
    }
    Ok((lo, hi))
}

/// A dense vector of length `len` from a sparse JTASK vector with index array `idx_key` and
/// `val`; an empty object is the zero vector.
fn sparse_vector(o: &Obj, idx_key: &str, len: usize, what: &str) -> Result<Vec<f64>, String> {
    reject_unknown(o, &[idx_key, "val"], what)?;
    let mut out = vec![0.0; len];
    if o.is_empty() {
        return Ok(out);
    }
    let idx = indices(key(o, idx_key, what)?, &format!("{what}.{idx_key}"))?;
    let val = numbers(key(o, "val", what)?, &format!("{what}.val"))?;
    check_len(
        val.len(),
        idx.len(),
        &format!("{what}.val"),
        &format!("{what}.{idx_key}"),
    )?;
    let mut seen = vec![false; len];
    for (&i, &v) in idx.iter().zip(&val) {
        if i >= len {
            return Err(format!(
                "{what}.{idx_key}: index {i} out of range (length {len})"
            ));
        }
        if seen[i] {
            return Err(format!("{what}.{idx_key}: duplicate index {i}"));
        }
        seen[i] = true;
        out[i] = finite(v, &format!("{what}.val (index {i})"))?;
    }
    Ok(out)
}

/// A rows × cols CSC matrix from `subi`, `subj`, `val` triplets, with `nnz` the expected
/// count. Row indices are sorted within each column; a repeated (row, column) pair is
/// rejected.
fn triplets_to_csc(
    o: &Obj,
    rows: usize,
    cols: usize,
    nnz: usize,
    what: &str,
    nnz_name: &str,
) -> Result<Csc, String> {
    let (subi, subj, val) = if o.is_empty() {
        (Vec::new(), Vec::new(), Vec::new())
    } else {
        (
            indices(key(o, "subi", what)?, &format!("{what}.subi"))?,
            indices(key(o, "subj", what)?, &format!("{what}.subj"))?,
            numbers(key(o, "val", what)?, &format!("{what}.val"))?,
        )
    };
    check_len(subi.len(), nnz, &format!("{what}.subi"), nnz_name)?;
    check_len(subj.len(), nnz, &format!("{what}.subj"), nnz_name)?;
    check_len(val.len(), nnz, &format!("{what}.val"), nnz_name)?;
    for t in 0..nnz {
        if subi[t] >= rows || subj[t] >= cols {
            return Err(format!(
                "{what}: entry {t} at ({}, {}) out of range ({rows} x {cols})",
                subi[t], subj[t]
            ));
        }
        finite(val[t], &format!("{what}.val[{t}]"))?;
    }
    let mut order: Vec<usize> = (0..nnz).collect();
    order.sort_unstable_by_key(|&t| (subj[t], subi[t]));
    let mut col_ptrs = vec![0usize; cols + 1];
    let mut row_indices = Vec::with_capacity(nnz);
    let mut values = Vec::with_capacity(nnz);
    for (pos, &t) in order.iter().enumerate() {
        if pos > 0 {
            let p = order[pos - 1];
            if subi[p] == subi[t] && subj[p] == subj[t] {
                return Err(format!(
                    "{what}: duplicate entry at ({}, {})",
                    subi[t], subj[t]
                ));
            }
        }
        col_ptrs[subj[t] + 1] += 1;
        row_indices.push(subi[t]);
        values.push(val[t]);
    }
    for j in 0..cols {
        col_ptrs[j + 1] += col_ptrs[j];
    }
    Ok((col_ptrs, row_indices, values))
}

/// One domain: `[type]` for `pexp`/`dexp`, `[type, d]` for `quad`, `rquad`, `rplus`, `rzero`,
/// `[type, d, [α…]]` for `ppow`/`dpow`.
fn domain(t: &Json, what: &str) -> Result<Cone, String> {
    let parts = array(t, what)?;
    let kind = match parts.first() {
        Some(Json::String(s)) => s.as_str(),
        other => {
            return Err(format!(
                "{what}: expected a domain type string, got {other:?}"
            ));
        }
    };
    let shape = |want: usize| {
        if parts.len() == want {
            Ok(())
        } else {
            Err(format!(
                "{what}: domain {kind:?} has {} fields, expected {want}",
                parts.len()
            ))
        }
    };
    let dim = || index(&parts[1], &format!("{what} dimension"));
    Ok(match kind {
        "pexp" | "dexp" => {
            shape(1)?;
            if kind == "pexp" {
                Cone::PrimalExponential
            } else {
                Cone::DualExponential
            }
        }
        "quad" => {
            shape(2)?;
            Cone::Quadratic(dim()?)
        }
        "rquad" => {
            shape(2)?;
            Cone::RotatedQuadratic(dim()?)
        }
        "rplus" => {
            shape(2)?;
            Cone::Nonnegative(dim()?)
        }
        "rzero" => {
            shape(2)?;
            Cone::Zero(dim()?)
        }
        "ppow" | "dpow" => {
            shape(3)?;
            let dim = dim()?;
            let alpha = numbers(&parts[2], &format!("{what} weights"))?;
            for a in &alpha {
                finite(*a, &format!("{what} weights"))?;
            }
            if kind == "ppow" {
                Cone::PrimalPower { dim, alpha }
            } else {
                Cone::DualPower { dim, alpha }
            }
        }
        other => {
            return Err(format!(
                "{what}: unsupported domain type {other:?} (supported: quad, rquad, pexp, dexp, \
                 ppow, dpow, rplus, rzero)"
            ));
        }
    })
}

/// The cones of the ACCs, checking that they cover rows 0, 1, …, k − 1 consecutively and in
/// order, and that any constant vector b is zero.
fn accs(acc: &Obj, numacc: usize, domains: &[Cone], k: usize) -> Result<Vec<Cone>, String> {
    let dom = indices(key(acc, "domain", "ACC")?, "ACC.domain")?;
    check_len(dom.len(), numacc, "ACC.domain", "Task/INFO.numacc")?;
    let afeidx = array(key(acc, "afeidx", "ACC")?, "ACC.afeidx")?;
    check_len(afeidx.len(), numacc, "ACC.afeidx", "Task/INFO.numacc")?;
    let b = match acc.get("b") {
        None => None,
        Some(b) => {
            let b = array(b, "ACC.b")?;
            check_len(b.len(), numacc, "ACC.b", "Task/INFO.numacc")?;
            Some(b)
        }
    };
    let mut next_row = 0usize;
    let mut cones = Vec::with_capacity(numacc);
    for (i, (&d, rows)) in dom.iter().zip(afeidx).enumerate() {
        let cone = domains.get(d).ok_or_else(|| {
            format!(
                "ACC.domain[{i}] = {d} out of range ({} domains)",
                domains.len()
            )
        })?;
        let rows = indices(rows, &format!("ACC.afeidx[{i}]"))?;
        if rows.len() != cone.dim() {
            return Err(format!(
                "ACC {i}: domain {d} has dimension {} but afeidx has {} rows",
                cone.dim(),
                rows.len()
            ));
        }
        for &r in &rows {
            if r != next_row {
                return Err(format!(
                    "ACC {i}: afeidx row {r} where row {next_row} was expected; cones must \
                     cover the rows of F consecutively, in order, from 0"
                ));
            }
            next_row += 1;
        }
        if let Some(b) = &b {
            let bi = numbers(&b[i], &format!("ACC.b[{i}]"))?;
            check_len(
                bi.len(),
                rows.len(),
                &format!("ACC.b[{i}]"),
                &format!("ACC.afeidx[{i}]"),
            )?;
            if bi.iter().any(|v| *v != 0.0) {
                return Err(format!(
                    "ACC.b[{i}] is nonzero: a constant vector b is not supported"
                ));
            }
        }
        cones.push(cone.clone());
    }
    if next_row != k {
        return Err(format!(
            "the ACCs cover {next_row} rows but F has k = {k}: every row must belong to a cone"
        ));
    }
    Ok(cones)
}

// ---------------------------------------------------------------------------------------------
// JSON access

fn missing(k: &str, ctx: &str) -> String {
    format!("missing key {k:?} in {ctx}")
}

fn key<'a>(o: &'a Obj, k: &str, ctx: &str) -> Result<&'a Json, String> {
    o.get(k).ok_or_else(|| missing(k, ctx))
}

fn reject_unknown(o: &Obj, known: &[&str], ctx: &str) -> Result<(), String> {
    match o.keys().find(|k| !known.contains(&k.as_str())) {
        Some(k) => Err(format!("unknown key {k:?} in {ctx} (not supported)")),
        None => Ok(()),
    }
}

fn obj<'a>(j: &'a Json, what: &str) -> Result<&'a Obj, String> {
    match j {
        Json::Object(o) => Ok(o),
        other => Err(format!("{what}: expected an object, got {other:?}")),
    }
}

fn array<'a>(j: &'a Json, what: &str) -> Result<&'a [Json], String> {
    match j {
        Json::Array(a) => Ok(a),
        other => Err(format!("{what}: expected an array, got {other:?}")),
    }
}

fn number(j: &Json, what: &str) -> Result<f64, String> {
    match j {
        Json::Number(v) => Ok(*v),
        other => Err(format!("{what}: expected a number, got {other:?}")),
    }
}

fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what}: value {v} is not finite"))
    }
}

/// A non-negative integer below 2^53.
#[allow(clippy::cast_possible_truncation)] // integral and below 2^53, checked first
fn index(j: &Json, what: &str) -> Result<usize, String> {
    match j {
        Json::Number(v) if *v >= 0.0 && v.fract() == 0.0 && *v < 9.0e15 => Ok(*v as usize),
        other => Err(format!(
            "{what}: expected a non-negative integer, got {other:?}"
        )),
    }
}

fn indices(j: &Json, what: &str) -> Result<Vec<usize>, String> {
    array(j, what)?
        .iter()
        .enumerate()
        .map(|(i, v)| index(v, &format!("{what}[{i}]")))
        .collect()
}

fn numbers(j: &Json, what: &str) -> Result<Vec<f64>, String> {
    array(j, what)?
        .iter()
        .enumerate()
        .map(|(i, v)| number(v, &format!("{what}[{i}]")))
        .collect()
}

fn strings<'a>(j: &'a Json, what: &str) -> Result<Vec<&'a str>, String> {
    array(j, what)?
        .iter()
        .enumerate()
        .map(|(i, v)| match v {
            Json::String(s) => Ok(s.as_str()),
            other => Err(format!("{what}[{i}]: expected a string, got {other:?}")),
        })
        .collect()
}

fn check_len(got: usize, want: usize, what: &str, against: &str) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what} has length {got}, but {against} = {want}"))
    }
}

// ---------------------------------------------------------------------------------------------
// Self-tests, registered by tests/corpus.rs.

/// A small conic problem using every supported key: n = 4, m = 2, k = 4, cones R₊¹ × K_exp.
const TINY: &str = r#"{"$schema":"http://mosek.com/json/schema#","Task/name":"tiny",
"Task/INFO":{"numvar":4,"numcon":2,"numcone":0,"numbarvar":0,"numanz":4,"numsymmat":0,
"numafe":4,"numfnz":5,"numacc":2,"numdjc":0,"numdom":2,"mosekver":[11,2,0]},
"Task/data":{
"var":{"name":["x0","x1","x2","x3"],"bk":["fr","lo","up","fx"],"bl":[-1e+30,0.5,-1e+30,2],
"bu":[1e+30,1e+30,7,2],"type":["cont","cont","cont","cont"]},
"con":{"name":["r0","r1"],"bk":["ra","fx"],"bl":[-1,4],"bu":[3,4]},
"objective":{"sense":"max","name":"obj","c":{"subj":[2],"val":[1.5]},"cfix":2.25},
"A":{"subi":[1,0,0,1],"subj":[0,0,2,3],"val":[1,5,2,3]},
"AFE":{"numafe":4,"F":{"subi":[0,1,2,2,3],"subj":[3,0,1,0,0],"val":[1,-1,2,4,0.5]},
"g":{"subi":[3],"val":[-0.25]}},
"domains":{"name":["","e"],"type":[["rplus",1],["pexp"]]},
"ACC":{"name":["a","b"],"domain":[0,1],"afeidx":[[0],[1,2,3]],"b":[[0],[0,0,0]]}},
"Task/parameters":{"rparam":{"MSK_DPAR_DATA_TOL_BOUND_INF":1e500,"x":-1e500},"iparam":{},
"sparam":{}}}"#;

/// A problem with n = 1, m = 0 and one ACC per domain, each on consecutive rows of F = 1.
fn conic_only(types: &str, dims: &[usize]) -> String {
    let k: usize = dims.iter().sum();
    let list =
        |f: &dyn Fn(usize) -> String, len: usize| (0..len).map(f).collect::<Vec<_>>().join(",");
    let mut start = 0;
    let afeidx = dims
        .iter()
        .map(|d| {
            let s = format!("[{}]", list(&|i| (start + i).to_string(), *d));
            start += d;
            s
        })
        .collect::<Vec<_>>()
        .join(",");
    let p = dims.len();
    format!(
        r#"{{"Task/INFO":{{"numvar":1,"numcon":0,"numanz":0,"numafe":{k},"numfnz":{k},
"numacc":{p},"numdom":{p}}},"Task/data":{{"var":{{"bk":["fr"],"bl":[0],"bu":[0]}},
"objective":{{"sense":"min","c":{{}}}},"AFE":{{"numafe":{k},"F":{{"subi":[{}],
"subj":[{}],"val":[{}]}}}},"domains":{{"type":{types}}},
"ACC":{{"domain":[{}],"afeidx":[{afeidx}]}}}}}}"#,
        list(&|i| i.to_string(), k),
        list(&|_| "0".into(), k),
        list(&|_| "1".into(), k),
        list(&|i| i.to_string(), p),
    )
}

fn tiny_conic_every_key() -> Result<(), super::Failed> {
    let inf = f64::INFINITY;
    let p = read_jtask_str(TINY).map_err(fail)?;
    ensure(p.name == "tiny", "name")?;
    ensure(p.sense == Sense::Maximise, "sense")?;
    ensure((p.m, p.n, p.k) == (2, 4, 4), "m, n, k")?;
    // A (2 x 4) from column-major triplets with column 0 out of row order; column 1 empty.
    ensure(
        p.A_col_ptrs == [0, 2, 2, 3, 4],
        format!("A ptrs {:?}", p.A_col_ptrs),
    )?;
    ensure(
        p.A_row_indices == [0, 1, 0, 1],
        format!("A rows {:?}", p.A_row_indices),
    )?;
    ensure(
        p.A_values == [5.0, 1.0, 2.0, 3.0],
        format!("A vals {:?}", p.A_values),
    )?;
    ensure(p.c == [0.0, 0.0, 1.5, 0.0], format!("c {:?}", p.c))?;
    ensure(p.gamma == 2.25, "gamma")?;
    ensure(p.l == [-inf, 0.5, -inf, 2.0], format!("l {:?}", p.l))?;
    ensure(p.u == [inf, inf, 7.0, 2.0], format!("u {:?}", p.u))?;
    ensure(p.b_l == [-1.0, 4.0], format!("b_l {:?}", p.b_l))?;
    ensure(p.b_u == [3.0, 4.0], format!("b_u {:?}", p.b_u))?;
    // F (4 x 4) from row-major triplets.
    ensure(
        p.F_col_ptrs == [0, 3, 4, 4, 5],
        format!("F ptrs {:?}", p.F_col_ptrs),
    )?;
    ensure(
        p.F_row_indices == [1, 2, 3, 2, 0],
        format!("F rows {:?}", p.F_row_indices),
    )?;
    ensure(
        p.F_values == [-1.0, 4.0, 0.5, 2.0, 1.0],
        format!("F vals {:?}", p.F_values),
    )?;
    ensure(p.g == [0.0, 0.0, 0.0, -0.25], format!("g {:?}", p.g))?;
    ensure(
        p.cones == [Cone::Nonnegative(1), Cone::PrimalExponential],
        format!("cones {:?}", p.cones),
    )?;
    let q = p.problem().map_err(fail)?;
    ensure(
        (q.m(), q.n(), q.k(), q.gamma()) == (2, 4, 4, 2.25),
        "problem dimensions and gamma",
    )?;
    ensure(
        q.conic().is_some_and(|c| c.cones().len() == 2),
        "problem cones",
    )
}

fn no_matrix_and_no_cones() -> Result<(), super::Failed> {
    // numanz = 0 with A absent, and with A an empty object; k = 0 with no AFE.
    for a in ["", r#","A":{}"#, r#","A":{"subi":[],"subj":[],"val":[]}"#] {
        let text = format!(
            r#"{{"Task/INFO":{{"numvar":2,"numcon":1,"numanz":0,"numafe":0,"numfnz":0,
"numacc":0,"numdom":0}},"Task/data":{{"var":{{"bk":["lo","lo"],"bl":[0,0],"bu":[1e30,1e30]}},
"con":{{"bk":["up"],"bl":[-1e30],"bu":[1]}},"objective":{{"sense":"min","c":{{}},"cfix":0}}{a}}}}}"#
        );
        let p = read_jtask_str(&text).map_err(|e| fail(format!("{a:?}: {e}")))?;
        ensure(
            p.A_col_ptrs == [0, 0, 0] && p.nnz_A() == 0 && p.k == 0 && p.cones.is_empty(),
            format!("{a:?}: {p:?}"),
        )?;
        let q = p.problem().map_err(fail)?;
        ensure(q.conic().is_none() && q.k() == 0, "no conic part")?;
    }
    Ok(())
}

fn domain_types() -> Result<(), super::Failed> {
    let types = r#"[["quad",3],["rquad",3],["pexp"],["dexp"],["rplus",2],["rzero",2],
["ppow",3,[0.25,0.75]],["dpow",4,[1,2]]]"#;
    let p = read_jtask_str(&conic_only(types, &[3, 3, 3, 3, 2, 2, 3, 4])).map_err(fail)?;
    let want = vec![
        Cone::Quadratic(3),
        Cone::RotatedQuadratic(3),
        Cone::PrimalExponential,
        Cone::DualExponential,
        Cone::Nonnegative(2),
        Cone::Zero(2),
        Cone::PrimalPower {
            dim: 3,
            alpha: vec![0.25, 0.75],
        },
        Cone::DualPower {
            dim: 4,
            alpha: vec![1.0, 2.0],
        },
    ];
    ensure(p.cones == want, format!("cones {:?}", p.cones))?;
    ensure(p.k == 23 && p.g == vec![0.0; 23], "k and zero g")?;
    p.problem().map(|_| ()).map_err(fail)
}

/// Malformed variants of [`TINY`]: (label, text replaced, replacement, expected message part).
const MALFORMED: &[(&str, &str, &str, &str)] = &[
    (
        "missing_info",
        "\"Task/INFO\"",
        "\"Task/INFOX\"",
        "missing key \"Task/INFO\"",
    ),
    (
        "missing_numvar",
        "\"numvar\"",
        "\"numvarX\"",
        "missing key \"numvar\"",
    ),
    (
        "missing_var_bk",
        "\"bk\":[\"fr\"",
        "\"bkX\":[\"fr\"",
        "missing key \"bk\"",
    ),
    (
        "missing_sense",
        "\"sense\"",
        "\"senseX\"",
        "missing key \"sense\"",
    ),
    ("missing_A", "\"A\":{", "\"AX\":{", "missing key \"A\""),
    (
        "missing_ACC",
        "\"ACC\":{",
        "\"ACCX\":{",
        "missing key \"ACC\"",
    ),
    (
        "unknown_data_key",
        "\"domains\":{",
        "\"qobj\":{},\"domains\":{",
        "unknown key \"qobj\"",
    ),
    (
        "unsupported_count",
        "\"numcone\":0",
        "\"numcone\":1",
        "numcone",
    ),
    (
        "var_length",
        "\"bl\":[-1e+30,0.5,-1e+30,2]",
        "\"bl\":[-1e+30,0.5,2]",
        "var.bl has length 3",
    ),
    (
        "con_length",
        "\"bu\":[3,4]",
        "\"bu\":[3]",
        "con.bu has length 1",
    ),
    (
        "numanz_mismatch",
        "\"numanz\":4",
        "\"numanz\":5",
        "Task/INFO.numanz",
    ),
    (
        "A_arrays_differ",
        "\"val\":[1,5,2,3]",
        "\"val\":[1,5,2]",
        "A.val has length 3",
    ),
    (
        "numafe_mismatch",
        "\"AFE\":{\"numafe\":4",
        "\"AFE\":{\"numafe\":5",
        "AFE.numafe = 5",
    ),
    (
        "numfnz_mismatch",
        "\"numfnz\":5",
        "\"numfnz\":4",
        "Task/INFO.numfnz",
    ),
    (
        "numdom_mismatch",
        "\"numdom\":2",
        "\"numdom\":3",
        "Task/INFO.numdom",
    ),
    (
        "numacc_mismatch",
        "\"numacc\":2",
        "\"numacc\":1",
        "Task/INFO.numacc",
    ),
    (
        "unknown_bound_key",
        "\"bk\":[\"fr\"",
        "\"bk\":[\"zz\"",
        "unknown bound key",
    ),
    (
        "fx_unequal",
        "\"bu\":[1e+30,1e+30,7,2]",
        "\"bu\":[1e+30,1e+30,7,3]",
        "bound key fx",
    ),
    (
        "bad_sense",
        "\"sense\":\"max\"",
        "\"sense\":\"maximize\"",
        "objective.sense",
    ),
    (
        "integer_variable",
        "\"type\":[\"cont\",\"cont\"",
        "\"type\":[\"int\",\"cont\"",
        "continuous",
    ),
    (
        "unknown_domain",
        "[\"pexp\"]",
        "[\"svecpsd\",3]",
        "unsupported domain type \"svecpsd\"",
    ),
    (
        "domain_dimension",
        "[\"rplus\",1]",
        "[\"rplus\",2]",
        "has dimension 2 but afeidx has 1",
    ),
    (
        "pexp_with_dimension",
        "[\"pexp\"]",
        "[\"pexp\",3]",
        "has 2 fields",
    ),
    (
        "domain_index",
        "\"domain\":[0,1]",
        "\"domain\":[0,2]",
        "ACC.domain[1] = 2 out of range",
    ),
    (
        "afeidx_out_of_order",
        "[[0],[1,2,3]]",
        "[[0],[1,3,2]]",
        "consecutively",
    ),
    (
        "afeidx_acc_order",
        "\"domain\":[0,1],\"afeidx\":[[0],[1,2,3]],\"b\":[[0],[0,0,0]]",
        "\"domain\":[1,0],\"afeidx\":[[1,2,3],[0]],\"b\":[[0,0,0],[0]]",
        "consecutively",
    ),
    (
        "acc_b_nonzero",
        "\"b\":[[0],[0,0,0]]",
        "\"b\":[[0],[0,1,0]]",
        "ACC.b[1] is nonzero",
    ),
    (
        "duplicate_A",
        "\"subj\":[0,0,2,3]",
        "\"subj\":[0,0,0,3]",
        "duplicate entry at (0, 0)",
    ),
    (
        "duplicate_F",
        "\"subj\":[3,0,1,0,0]",
        "\"subj\":[3,0,0,0,0]",
        "AFE.F: duplicate entry",
    ),
    (
        "duplicate_c",
        "\"c\":{\"subj\":[2],\"val\":[1.5]}",
        "\"c\":{\"subj\":[2,2],\"val\":[1.5,1]}",
        "duplicate index 2",
    ),
    (
        "A_row_range",
        "\"subi\":[1,0,0,1]",
        "\"subi\":[1,0,0,2]",
        "out of range (2 x 4)",
    ),
    (
        "F_column_range",
        "\"subj\":[3,0,1,0,0]",
        "\"subj\":[4,0,1,0,0]",
        "out of range (4 x 4)",
    ),
    (
        "c_range",
        "\"subj\":[2],\"val\":[1.5]",
        "\"subj\":[4],\"val\":[1.5]",
        "index 4 out of range",
    ),
    (
        "g_range",
        "\"subi\":[3],\"val\":[-0.25]",
        "\"subi\":[4],\"val\":[-0.25]",
        "out of range",
    ),
    (
        "non_integer_index",
        "\"subi\":[1,0,0,1]",
        "\"subi\":[1,0,0.5,1]",
        "non-negative integer",
    ),
    (
        "negative_index",
        "\"subi\":[1,0,0,1]",
        "\"subi\":[1,0,-1,1]",
        "non-negative integer",
    ),
    (
        "infinite_A",
        "\"val\":[1,5,2,3]",
        "\"val\":[1,1e500,2,3]",
        "not finite",
    ),
    (
        "infinite_F",
        "\"val\":[1,-1,2,4,0.5]",
        "\"val\":[1,-1,2,-1e500,0.5]",
        "not finite",
    ),
    (
        "infinite_g",
        "\"val\":[-0.25]",
        "\"val\":[1e500]",
        "not finite",
    ),
    (
        "infinite_c",
        "\"val\":[1.5]",
        "\"val\":[1e500]",
        "not finite",
    ),
    (
        "infinite_cfix",
        "\"cfix\":2.25",
        "\"cfix\":1e500",
        "not finite",
    ),
    (
        "infinite_used_bound",
        "\"bl\":[-1,4]",
        "\"bl\":[-1e500,4]",
        "not finite",
    ),
    (
        "nan_token",
        "\"val\":[1,5,2,3]",
        "\"val\":[1,NaN,2,3]",
        "JSON",
    ),
    (
        "not_an_object",
        "\"Task/data\":{",
        "\"Task/data\":[{",
        "JSON",
    ),
];

/// Every malformed variant is rejected with an error (not a panic) naming the fault, and each
/// replacement is checked to apply, so a stale case cannot pass by testing the valid input.
fn malformed_trials() -> Vec<Trial> {
    MALFORMED
        .iter()
        .map(|&(label, from, to, needle)| {
            Trial::test(format!("jtask::rejects::{label}"), move || {
                ensure(
                    TINY.matches(from).count() == 1,
                    format!("{from:?} does not occur exactly once in the valid input"),
                )?;
                match read_jtask_str(&TINY.replace(from, to)) {
                    Ok(_) => Err(fail(format!("accepted {to:?}"))),
                    Err(e) => ensure(e.contains(needle), format!("error {e:?} lacks {needle:?}")),
                }
            })
        })
        .collect()
}

/// The provenance name of a cone type.
fn cone_type(c: &Cone) -> &'static str {
    match c {
        Cone::Quadratic(_) => "quadratic",
        Cone::RotatedQuadratic(_) => "rotated quadratic",
        Cone::PrimalExponential => "primal exponential",
        Cone::DualExponential => "dual exponential",
        Cone::PrimalPower { .. } => "primal power",
        Cone::DualPower { .. } => "dual power",
        Cone::Nonnegative(_) => "nonnegative",
        Cone::Zero(_) => "zero",
        _ => "other",
    }
}

/// The licensed-tier CLP fixtures read back with the dimensions, entry counts and cone counts
/// their provenance records state, and with the recorded SHA-256.
fn clp_fixtures_match_provenance() -> Result<(), super::Failed> {
    let instances = clp_instances().map_err(fail)?;
    ensure(!instances.is_empty(), "no CLP instances declared")?;
    for inst in instances {
        let path = jtask_path(&inst).map_err(fail)?;
        let bytes = std::fs::read(&path).map_err(|e| fail(format!("{}: {e}", path.display())))?;
        let rec = Provenance::load(&clp_provenance_path(&inst).map_err(fail)?).map_err(fail)?;
        let field = |k: &str| rec.usize(k).map_err(|e| fail(format!("{inst}: {e}")));
        ensure(
            sha256_hex(&bytes) == rec.str("jtask_sha256").map_err(fail)?,
            format!("{inst}: SHA-256 differs from jtask_sha256"),
        )?;
        let text = String::from_utf8(bytes).map_err(fail)?;
        let p = read_jtask_str(&text).map_err(|e| fail(format!("{inst}: {e}")))?;
        let got = (p.n, p.m, p.nnz_A(), p.k, p.nnz_F());
        let want = (
            field("n")?,
            field("m")?,
            field("nnz_A")?,
            field("k")?,
            field("nnz_F")?,
        );
        ensure(
            got == want,
            format!("{inst}: (n, m, nnz_A, k, nnz_F) = {got:?}, provenance {want:?}"),
        )?;
        let mut counts: BTreeMap<(String, usize), usize> = BTreeMap::new();
        for c in &p.cones {
            *counts.entry((cone_type(c).into(), c.dim())).or_default() += 1;
        }
        let mut recorded: BTreeMap<(String, usize), usize> = BTreeMap::new();
        let Some(Json::Array(groups)) = rec.fields.get("cones") else {
            return Err(fail(format!("{inst}: provenance has no cones array")));
        };
        for g in groups {
            let g = obj(g, "provenance cones").map_err(fail)?;
            let t = match key(g, "type", "provenance cones").map_err(fail)? {
                Json::String(s) => s.clone(),
                other => return Err(fail(format!("{inst}: cone type {other:?}"))),
            };
            let d =
                index(key(g, "dimension", "cones").map_err(fail)?, "dimension").map_err(fail)?;
            let n = index(key(g, "count", "cones").map_err(fail)?, "count").map_err(fail)?;
            *recorded.entry((t, d)).or_default() += n;
        }
        ensure(
            counts == recorded,
            format!("{inst}: cones {counts:?}, provenance {recorded:?}"),
        )?;
        let sense = if p.sense == Sense::Minimise {
            "minimize"
        } else {
            "maximize"
        };
        ensure(
            rec.str("objective_sense").map_err(fail)? == sense,
            format!("{inst}: objective sense"),
        )?;
        p.problem()
            .map_err(|e| fail(format!("{inst}: not a valid problem: {e}")))?;
    }
    Ok(())
}

/// Trials of the JTASK reader: hand-written inputs, malformed variants and the licensed-tier
/// CLP fixtures against their provenance records.
pub fn self_tests() -> Vec<Trial> {
    let mut trials = vec![
        Trial::test("jtask::tiny_conic_every_key", tiny_conic_every_key),
        Trial::test("jtask::no_matrix_and_no_cones", no_matrix_and_no_cones),
        Trial::test("jtask::domain_types", domain_types),
        Trial::test(
            "jtask::clp_fixtures_match_provenance",
            clp_fixtures_match_provenance,
        ),
    ];
    trials.extend(malformed_trials());
    trials
}
