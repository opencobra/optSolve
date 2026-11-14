//! A small MPS reader for test fixtures (research R11). Test-only code; no dependency.
//!
//! Reads the sections NAME, OBJSENSE, ROWS, COLUMNS, RHS, RANGES, BOUNDS and ENDATA, in free
//! format (whitespace-separated fields, so names must not contain spaces; fixed-format files
//! whose names have no spaces read identically). Bound types UP, LO, FX, FR, MI and PL are
//! accepted; integer markers and other bound types are rejected with a message.
//!
//! Conventions:
//! * the first N row is the objective; a second N row is rejected;
//! * a column with no BOUNDS entry has l = 0, u = +∞;
//! * an UP bound below zero on a column whose lower bound is not set explicitly makes l = −∞
//!   (the classic MPS rule, which Gurobi's reader also applies);
//! * a bound value with magnitude at least 1e30 is infinite (Gurobi's convention), as are the
//!   literals `inf`, `infinity` and their negatives;
//! * a RHS entry on the objective row sets the objective constant γ = −value;
//! * RANGES follow the MPS definition: on an L row b_l = b − |r|; on a G row b_u = b + |r|; on
//!   an E row b_u = b + r when r > 0 and b_l = b + r when r < 0.

#![allow(non_snake_case)] // A is the constraint matrix (Notation Convention).

use super::{Trial, ensure, fail};
use optsolver::{CscRef, Problem, RowBounds, Sense};
use std::collections::HashMap;

/// A linear programme read from an MPS file, owning its arrays.
#[derive(Debug, Clone)]
pub struct MpsProblem {
    /// Name from the NAME line (empty when absent).
    pub name: String,
    /// Objective sense (OBJSENSE; minimise when absent).
    pub objective_sense: Sense,
    /// Number of rows of A.
    pub m: usize,
    /// Number of columns of A.
    pub n: usize,
    /// Column pointers of A, length n + 1.
    pub col_ptrs: Vec<usize>,
    /// Row indices of A, sorted within each column, length nnz.
    pub row_indices: Vec<usize>,
    /// Values of A, length nnz.
    pub values: Vec<f64>,
    /// Row senses `b'='`, `b'<'`, `b'>'`, length m.
    pub sense: Vec<u8>,
    /// Row right-hand sides b, length m.
    pub b: Vec<f64>,
    /// RANGES value r of each row, if any.
    pub ranges: Vec<Option<f64>>,
    /// Variable lower bounds l, length n.
    pub l: Vec<f64>,
    /// Variable upper bounds u, length n.
    pub u: Vec<f64>,
    /// Objective coefficients c, length n.
    pub c: Vec<f64>,
    /// Objective constant γ.
    pub gamma: f64,
    /// Name of the objective row.
    pub objective_name: String,
    /// Row names, length m.
    pub row_names: Vec<String>,
    /// Column names, length n.
    pub col_names: Vec<String>,
}

impl MpsProblem {
    /// Number of stored entries of A.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Whether any row has a RANGES entry.
    pub fn has_ranges(&self) -> bool {
        self.ranges.iter().any(Option::is_some)
    }

    /// Canonical bounds (b_l,i, b_u,i) of row i, applying RANGES.
    pub fn row_bounds(&self, i: usize) -> (f64, f64) {
        let b = self.b[i];
        let (lo, hi) = match self.sense[i] {
            b'<' => (f64::NEG_INFINITY, b),
            b'>' => (b, f64::INFINITY),
            _ => (b, b),
        };
        match (self.sense[i], self.ranges[i]) {
            (_, None) => (lo, hi),
            (b'<', Some(r)) => (b - r.abs(), hi),
            (b'>', Some(r)) => (lo, b + r.abs()),
            (_, Some(r)) if r >= 0.0 => (b, b + r),
            (_, Some(r)) => (b + r, b),
        }
    }

    /// Lower and upper row bounds b_l, b_u (owned), applying RANGES.
    pub fn row_ranges(&self) -> (Vec<f64>, Vec<f64>) {
        (0..self.m).map(|i| self.row_bounds(i)).unzip()
    }

    /// A borrowed view of A.
    pub fn A(&self) -> Result<CscRef<'_, usize>, optsolver::Error> {
        CscRef::new(
            self.m,
            self.n,
            &self.col_ptrs,
            &self.row_indices,
            &self.values,
        )
    }

    /// The problem as an `optsolver::Problem` with sense-and-right-hand-side rows and the
    /// objective `c` given (the stored c or a seeded one). Rows with RANGES cannot be expressed
    /// this way: use [`MpsProblem::row_ranges`] with `RowBounds::Ranges` instead.
    pub fn problem<'a>(&'a self, c: &'a [f64]) -> Result<Problem<'a, usize>, String> {
        if self.has_ranges() {
            return Err(format!(
                "{}: RANGES present; build RowBounds::Ranges from row_ranges()",
                self.name
            ));
        }
        let A = self.A().map_err(|e| e.to_string())?;
        let rows = RowBounds::SenseRhs {
            sense: &self.sense,
            b: &self.b,
        };
        Problem::new(A, c, self.objective_sense, rows, &self.l, &self.u)
            .and_then(|p| p.with_gamma(self.gamma))
            .map_err(|e| e.to_string())
    }
}

/// Reads an MPS file.
pub fn read_mps_file(path: &std::path::Path) -> Result<MpsProblem, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_mps(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Name,
    ObjSense,
    Rows,
    Columns,
    Rhs,
    Ranges,
    Bounds,
    End,
}

/// Parses MPS text. Errors name the line number and the problem.
pub fn parse_mps(text: &str) -> Result<MpsProblem, String> {
    let mut name = String::new();
    let mut objective_sense = Sense::Minimise;
    let mut objective_name: Option<String> = None;
    let mut row_index: HashMap<String, usize> = HashMap::new();
    let mut row_names: Vec<String> = Vec::new();
    let mut sense: Vec<u8> = Vec::new();
    let mut col_index: HashMap<String, usize> = HashMap::new();
    let mut col_names: Vec<String> = Vec::new();
    let mut col_ptrs: Vec<usize> = vec![0];
    let mut row_indices: Vec<usize> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let mut c: Vec<f64> = Vec::new();
    let mut gamma = 0.0;
    let mut b: Vec<f64> = Vec::new();
    let mut ranges: Vec<Option<f64>> = Vec::new();
    let mut l: Vec<f64> = Vec::new();
    let mut u: Vec<f64> = Vec::new();
    let mut lower_set: Vec<bool> = Vec::new();
    let mut section = Section::None;
    // Entries of the column being read, sorted and checked when the column ends.
    let mut current: Vec<(usize, f64)> = Vec::new();

    fn close_column(
        current: &mut Vec<(usize, f64)>,
        row_indices: &mut Vec<usize>,
        values: &mut Vec<f64>,
        col_ptrs: &mut Vec<usize>,
        col: &str,
        row_names: &[String],
    ) -> Result<(), String> {
        current.sort_by_key(|e| e.0);
        if let Some(w) = current.windows(2).find(|w| w[0].0 == w[1].0) {
            return Err(format!(
                "column {col}: row {} given twice",
                row_names[w[0].0]
            ));
        }
        row_indices.extend(current.iter().map(|e| e.0));
        values.extend(current.iter().map(|e| e.1));
        col_ptrs.push(row_indices.len());
        current.clear();
        Ok(())
    }

    for (k, raw) in text.lines().enumerate() {
        let line_no = k + 1;
        let err = |msg: String| format!("line {line_no}: {msg}");
        let line = raw.trim_end();
        if line.trim().is_empty() || line.starts_with('*') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let header = !raw.starts_with(' ') && !raw.starts_with('\t');
        if header {
            let new_section = match fields[0] {
                "NAME" => Section::Name,
                "OBJSENSE" => Section::ObjSense,
                "ROWS" => Section::Rows,
                "COLUMNS" => Section::Columns,
                "RHS" => Section::Rhs,
                "RANGES" => Section::Ranges,
                "BOUNDS" => Section::Bounds,
                "ENDATA" => Section::End,
                other => return Err(err(format!("unknown or unsupported section {other}"))),
            };
            if section == Section::Columns {
                if let Some(col) = col_names.last() {
                    if col_ptrs.len() == col_names.len() {
                        close_column(
                            &mut current,
                            &mut row_indices,
                            &mut values,
                            &mut col_ptrs,
                            col,
                            &row_names,
                        )
                        .map_err(err)?;
                    }
                }
            }
            if section == Section::End {
                return Err(err("data after ENDATA".into()));
            }
            section = new_section;
            match section {
                Section::Name => name = fields.get(1..).map(|f| f.join(" ")).unwrap_or_default(),
                Section::ObjSense if fields.len() == 2 => {
                    objective_sense = parse_sense(fields[1]).map_err(err)?;
                }
                _ if fields.len() > 1 => {
                    return Err(err(format!("unexpected fields after {}", fields[0])));
                }
                _ => {}
            }
            continue;
        }
        match section {
            Section::None | Section::Name => {
                return Err(err("data line before any section".into()));
            }
            Section::End => return Err(err("data after ENDATA".into())),
            Section::ObjSense => {
                if fields.len() != 1 {
                    return Err(err("OBJSENSE takes one field".into()));
                }
                objective_sense = parse_sense(fields[0]).map_err(err)?;
            }
            Section::Rows => {
                if fields.len() != 2 {
                    return Err(err(format!(
                        "ROWS line needs a type and a name, got {} fields",
                        fields.len()
                    )));
                }
                let (t, r) = (fields[0], fields[1]);
                if row_index.contains_key(r) || objective_name.as_deref() == Some(r) {
                    return Err(err(format!("row {r} defined twice")));
                }
                let s = match t {
                    "N" => {
                        if objective_name.is_some() {
                            return Err(err(format!(
                                "second objective (N) row {r} is not supported"
                            )));
                        }
                        objective_name = Some(r.to_string());
                        continue;
                    }
                    "E" => b'=',
                    "L" => b'<',
                    "G" => b'>',
                    other => return Err(err(format!("unknown row type {other}"))),
                };
                row_index.insert(r.to_string(), row_names.len());
                row_names.push(r.to_string());
                sense.push(s);
                b.push(0.0);
                ranges.push(None);
            }
            Section::Columns => {
                if fields.contains(&"'MARKER'") {
                    return Err(err("integer markers are not supported".into()));
                }
                if fields.len() != 3 && fields.len() != 5 {
                    return Err(err(format!(
                        "COLUMNS line needs 3 or 5 fields, got {}",
                        fields.len()
                    )));
                }
                let col = fields[0];
                let j = match col_names.last() {
                    Some(last) if last == col => col_names.len() - 1,
                    _ => {
                        if col_index.contains_key(col) {
                            return Err(err(format!("column {col} is not contiguous")));
                        }
                        if let Some(prev) = col_names.last() {
                            let prev = prev.clone();
                            close_column(
                                &mut current,
                                &mut row_indices,
                                &mut values,
                                &mut col_ptrs,
                                &prev,
                                &row_names,
                            )
                            .map_err(err)?;
                        }
                        col_index.insert(col.to_string(), col_names.len());
                        col_names.push(col.to_string());
                        c.push(0.0);
                        l.push(0.0);
                        u.push(f64::INFINITY);
                        lower_set.push(false);
                        col_names.len() - 1
                    }
                };
                for pair in fields[1..].chunks(2) {
                    let v = parse_number(pair[1]).map_err(err)?;
                    if objective_name.as_deref() == Some(pair[0]) {
                        c[j] = v;
                    } else {
                        let i = *row_index
                            .get(pair[0])
                            .ok_or_else(|| err(format!("unknown row {}", pair[0])))?;
                        current.push((i, v));
                    }
                }
            }
            Section::Rhs | Section::Ranges => {
                // Optional set name: an odd field count means it is present.
                let pairs = if fields.len() % 2 == 1 {
                    &fields[1..]
                } else {
                    &fields[..]
                };
                if pairs.is_empty() {
                    return Err(err("RHS/RANGES line has no entries".into()));
                }
                for pair in pairs.chunks(2) {
                    let v = parse_number(pair[1]).map_err(err)?;
                    if objective_name.as_deref() == Some(pair[0]) {
                        if section == Section::Ranges {
                            return Err(err("RANGES on the objective row".into()));
                        }
                        gamma = -v;
                        continue;
                    }
                    let i = *row_index
                        .get(pair[0])
                        .ok_or_else(|| err(format!("unknown row {}", pair[0])))?;
                    if section == Section::Rhs {
                        b[i] = v;
                    } else {
                        ranges[i] = Some(v);
                    }
                }
            }
            Section::Bounds => {
                let t = fields[0];
                let needs_value = matches!(t, "UP" | "LO" | "FX");
                let no_value = matches!(t, "FR" | "MI" | "PL");
                if !needs_value && !no_value {
                    return Err(err(format!("unsupported bound type {t}")));
                }
                let rest = &fields[1..];
                let expected = usize::from(needs_value) + 1;
                let col = match rest.len() {
                    n if n == expected => rest[0],
                    n if n == expected + 1 => rest[1],
                    n => return Err(err(format!("{t} bound has {n} fields after the type"))),
                };
                let j = *col_index
                    .get(col)
                    .ok_or_else(|| err(format!("unknown column {col}")))?;
                let v = if needs_value {
                    parse_bound(rest[rest.len() - 1]).map_err(err)?
                } else {
                    0.0
                };
                match t {
                    "UP" => u[j] = v,
                    "LO" => {
                        l[j] = v;
                        lower_set[j] = true;
                    }
                    "FX" => {
                        l[j] = v;
                        u[j] = v;
                        lower_set[j] = true;
                    }
                    "FR" => {
                        l[j] = f64::NEG_INFINITY;
                        u[j] = f64::INFINITY;
                        lower_set[j] = true;
                    }
                    "MI" => {
                        l[j] = f64::NEG_INFINITY;
                        lower_set[j] = true;
                    }
                    _ => u[j] = f64::INFINITY,
                }
            }
        }
    }
    if section != Section::End {
        return Err("missing ENDATA".into());
    }
    let objective_name = objective_name.ok_or("no objective (N) row")?;
    for j in 0..col_names.len() {
        if !lower_set[j] && u[j] < 0.0 {
            l[j] = f64::NEG_INFINITY;
        }
    }
    Ok(MpsProblem {
        name,
        objective_sense,
        m: row_names.len(),
        n: col_names.len(),
        col_ptrs,
        row_indices,
        values,
        sense,
        b,
        ranges,
        l,
        u,
        c,
        gamma,
        objective_name,
        row_names,
        col_names,
    })
}

fn parse_sense(s: &str) -> Result<Sense, String> {
    match s.to_ascii_uppercase().as_str() {
        "MAX" | "MAXIMIZE" | "MAXIMISE" => Ok(Sense::Maximise),
        "MIN" | "MINIMIZE" | "MINIMISE" => Ok(Sense::Minimise),
        other => Err(format!("unknown objective sense {other}")),
    }
}

fn parse_number(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("not a number: {s}"))?;
    if v.is_nan() {
        return Err(format!("not a number: {s}"));
    }
    Ok(v)
}

fn parse_bound(s: &str) -> Result<f64, String> {
    let v = parse_number(s)?;
    Ok(if v >= 1e30 {
        f64::INFINITY
    } else if v <= -1e30 {
        f64::NEG_INFINITY
    } else {
        v
    })
}

// ---------------------------------------------------------------------------------------------
// Self-tests (task T014). Integration-test modules cannot use #[cfg(test)] unit tests reachable
// from every binary, so these are returned as trials and registered by tests/corpus.rs.

/// Trials checking the reader on small hand-written MPS strings.
pub fn self_tests() -> Vec<Trial> {
    vec![
        Trial::test(
            "mps::sections_rows_columns_rhs_objsense",
            sections_rows_columns_rhs_objsense,
        ),
        Trial::test("mps::objsense_on_header_line", objsense_on_header_line),
        Trial::test("mps::ranges_on_each_row_type", ranges_on_each_row_type),
        Trial::test("mps::bounds_up_lo_fx_fr_mi_pl", bounds_up_lo_fx_fr_mi_pl),
        Trial::test(
            "mps::infinite_and_negative_upper_bounds",
            infinite_and_negative_upper_bounds,
        ),
        Trial::test("mps::rejects_malformed_input", rejects_malformed_input),
    ]
}

const SMALL: &str = "\
NAME small
* a comment line
OBJSENSE
    MAX
ROWS
 N  obj
 E  r1
 L  r2
 G  r3
COLUMNS
    x1  obj  1.5  r2  2
    x1  r1  1
    x2  r3  -1  obj  -0.25
    x3  obj  0
RHS
    RHS  r1  4  r2  1e1
    RHS  r3  -2.5  obj  3
ENDATA
";

fn sections_rows_columns_rhs_objsense() -> Result<(), super::Failed> {
    let p = parse_mps(SMALL).map_err(fail)?;
    ensure(p.name == "small", format!("name {}", p.name))?;
    ensure(p.objective_sense == Sense::Maximise, "objective sense")?;
    ensure(
        (p.m, p.n, p.nnz()) == (3, 3, 3),
        format!("dims {} {} {}", p.m, p.n, p.nnz()),
    )?;
    ensure(p.row_names == ["r1", "r2", "r3"], "row names")?;
    ensure(p.col_names == ["x1", "x2", "x3"], "column names")?;
    ensure(p.objective_name == "obj", "objective name")?;
    ensure(p.sense == b"=<>", "row senses")?;
    ensure(
        p.col_ptrs == [0, 2, 3, 3],
        format!("col_ptrs {:?}", p.col_ptrs),
    )?;
    // Column x1 lists r2 before r1; the reader sorts row indices within the column.
    ensure(
        p.row_indices == [0, 1, 2],
        format!("row_indices {:?}", p.row_indices),
    )?;
    ensure(
        p.values == [1.0, 2.0, -1.0],
        format!("values {:?}", p.values),
    )?;
    ensure(p.c == [1.5, -0.25, 0.0], format!("c {:?}", p.c))?;
    ensure(p.b == [4.0, 10.0, -2.5], format!("b {:?}", p.b))?;
    ensure(p.gamma == -3.0, format!("gamma {}", p.gamma))?;
    ensure(
        p.l == [0.0; 3] && p.u == [f64::INFINITY; 3],
        "default bounds are [0, +inf)",
    )?;
    ensure(!p.has_ranges(), "no ranges")?;
    let pr = p.problem(&p.c).map_err(fail)?;
    ensure(pr.m() == 3 && pr.n() == 3, "converts to optsolver::Problem")
}

fn objsense_on_header_line() -> Result<(), super::Failed> {
    let text = SMALL.replace("OBJSENSE\n    MAX\n", "OBJSENSE MIN\n");
    let p = parse_mps(&text).map_err(fail)?;
    ensure(
        p.objective_sense == Sense::Minimise,
        "OBJSENSE MIN on one line",
    )?;
    let text = SMALL.replace("OBJSENSE\n    MAX\n", "");
    let p = parse_mps(&text).map_err(fail)?;
    ensure(
        p.objective_sense == Sense::Minimise,
        "minimise when OBJSENSE is absent",
    )
}

fn ranges_on_each_row_type() -> Result<(), super::Failed> {
    let text = "\
ROWS
 N  obj
 E  e_pos
 E  e_neg
 L  le
 G  ge
COLUMNS
    x  e_pos  1  e_neg  1
    x  le  1  ge  1
RHS
    RHS  e_pos  1  e_neg  1
    RHS  le  1  ge  1
RANGES
    RNG  e_pos  2  e_neg  -2
    le  -3  ge  -4
ENDATA
";
    let p = parse_mps(text).map_err(fail)?;
    ensure(p.has_ranges(), "ranges present")?;
    let (b_l, b_u) = p.row_ranges();
    ensure(b_l == [1.0, -1.0, -2.0, 1.0], format!("b_l {b_l:?}"))?;
    ensure(b_u == [3.0, 1.0, 1.0, 5.0], format!("b_u {b_u:?}"))?;
    ensure(
        p.problem(&p.c).is_err(),
        "sense-rhs conversion refuses ranged rows",
    )
}

fn bounds_up_lo_fx_fr_mi_pl() -> Result<(), super::Failed> {
    let text = "\
ROWS
 N  obj
 E  r
COLUMNS
    up  r  1
    lo  r  1
    fx  r  1
    fr  r  1
    mi  r  1
    pl  r  1
    none  r  1
BOUNDS
 UP BND  up  4
 LO BND  lo  -2
 FX BND  fx  3.25
 FR BND  fr
 MI BND  mi
 UP BND  mi  7
 UP BND  pl  9
 PL BND  pl
 LO lo  -1
ENDATA
";
    let p = parse_mps(text).map_err(fail)?;
    let inf = f64::INFINITY;
    ensure(
        p.l == [0.0, -1.0, 3.25, -inf, -inf, 0.0, 0.0],
        format!("l {:?}", p.l),
    )?;
    ensure(
        p.u == [4.0, inf, 3.25, inf, 7.0, inf, inf],
        format!("u {:?}", p.u),
    )
}

fn infinite_and_negative_upper_bounds() -> Result<(), super::Failed> {
    let text = "\
ROWS
 N  obj
 E  r
COLUMNS
    a  r  1
    b  r  1
    c  r  1
    d  r  1
BOUNDS
 UP BND  a  -1
 LO BND  b  -5
 UP BND  b  -1
 LO BND  c  -1e30
 UP BND  c  1e+30
 UP BND  d  -inf
ENDATA
";
    let p = parse_mps(text).map_err(fail)?;
    let inf = f64::INFINITY;
    ensure(p.l == [-inf, -5.0, -inf, -inf], format!("l {:?}", p.l))?;
    ensure(p.u == [-1.0, -1.0, inf, -inf], format!("u {:?}", p.u))
}

fn rejects_malformed_input() -> Result<(), super::Failed> {
    let base = "ROWS\n N  obj\n E  r\nCOLUMNS\n    x  r  1\nRHS\n    RHS  r  1\nENDATA\n";
    ensure(parse_mps(base).is_ok(), "base case parses")?;
    let cases: &[(&str, String, &str)] = &[
        ("missing ENDATA", base.replace("ENDATA\n", ""), "ENDATA"),
        (
            "bad number",
            base.replace("r  1\nRHS", "r  1.2.3\nRHS"),
            "not a number",
        ),
        (
            "unknown row",
            base.replace("x  r  1", "x  q  1"),
            "unknown row q",
        ),
        (
            "unknown row type",
            base.replace(" E  r", " X  r"),
            "unknown row type",
        ),
        (
            "unknown section",
            base.replace("RHS\n", "QUADOBJ\n"),
            "unsupported section",
        ),
        (
            "unknown column in BOUNDS",
            base.replace("ENDATA", "BOUNDS\n UP BND  y  1\nENDATA"),
            "unknown column",
        ),
        (
            "unsupported bound type",
            base.replace("ENDATA", "BOUNDS\n BV BND  x\nENDATA"),
            "unsupported bound",
        ),
        (
            "missing bound value",
            base.replace("ENDATA", "BOUNDS\n UP x\nENDATA"),
            "fields after the type",
        ),
        (
            "odd COLUMNS fields",
            base.replace("x  r  1", "x  r  1  obj"),
            "3 or 5 fields",
        ),
        (
            "non-contiguous column",
            base.replace("x  r  1", "x  r  1\n    y  obj  1\n    x  obj  2"),
            "not contiguous",
        ),
        (
            "duplicate entry",
            base.replace("x  r  1", "x  r  1  r  2"),
            "given twice",
        ),
        (
            "duplicate row",
            base.replace(" E  r", " E  r\n L  r"),
            "defined twice",
        ),
        (
            "second objective",
            base.replace(" E  r", " E  r\n N  obj2"),
            "second objective",
        ),
        (
            "integer marker",
            base.replace("x  r  1", "M  'MARKER'  'INTORG'\n    x  r  1"),
            "integer markers",
        ),
        (
            "bad objective sense",
            base.replace("ROWS", "OBJSENSE\n    UP\nROWS"),
            "objective sense",
        ),
        (
            "NaN value",
            base.replace("r  1\nRHS", "r  NaN\nRHS"),
            "not a number",
        ),
    ];
    for (what, text, needle) in cases {
        match parse_mps(text) {
            Ok(_) => return Err(fail(format!("{what}: accepted"))),
            Err(e) => ensure(
                e.contains(needle),
                format!("{what}: message {e:?} lacks {needle:?}"),
            )?,
        }
    }
    Ok(())
}
