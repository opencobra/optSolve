//! Frozen test corpus: instance list, fixture paths, provenance records and hash checks
//! (constitution II and X; data-model.md, Fixture provenance record).
//!
//! No dependency: SHA-256 (FIPS 180-4) and a minimal JSON reader are implemented here. The
//! JSON reader accepts the provenance records this project writes (objects, arrays, strings
//! with the common escapes, numbers, booleans, null).

use super::mps::{MpsProblem, read_mps_file};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The repository root (the crate manifest directory).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The project declaration file.
pub fn project_yml() -> PathBuf {
    repo_root()
        .join(".specify")
        .join("memory")
        .join("project.yml")
}

/// The fixture directory declared in `project.yml` (`test_corpus.fixtures`).
pub fn fixtures_dir() -> Result<PathBuf, String> {
    let v = corpus_declaration()?;
    let dir = v
        .fixtures
        .ok_or("project.yml: test_corpus.fixtures is missing")?;
    Ok(repo_root().join(dir))
}

/// The LP corpus instance names declared in `project.yml` (`test_corpus.lp_instances`).
pub fn lp_instances() -> Result<Vec<String>, String> {
    declared_list("lp_instances")
}

/// The derived QP fixture names (`test_corpus.qp_instances`).
pub fn qp_instances() -> Result<Vec<String>, String> {
    declared_list("qp_instances")
}

/// The CLP fixture names of the licensed tier (`test_corpus.clp_instances`).
pub fn clp_instances() -> Result<Vec<String>, String> {
    declared_list("clp_instances")
}

/// The CLP fixture names of the heavy tier (`test_corpus.clp_heavy_instances`).
pub fn clp_heavy_instances() -> Result<Vec<String>, String> {
    declared_list("clp_heavy_instances")
}

/// The toy instance names (`test_corpus.toy_instances`, feature 003).
pub fn toy_instances() -> Result<Vec<String>, String> {
    declared_list("toy_instances")
}

/// The file of a toy instance in `fixtures`/small: `<name>.mps` or `<name>.jtask`, whichever
/// exists (an error if neither or both do).
pub fn toy_path(name: &str) -> Result<PathBuf, String> {
    let dir = fixtures_dir()?.join("small");
    let (mps, jtask) = (
        dir.join(format!("{name}.mps")),
        dir.join(format!("{name}.jtask")),
    );
    match (mps.is_file(), jtask.is_file()) {
        (true, false) => Ok(mps),
        (false, true) => Ok(jtask),
        (true, true) => Err(format!("toy instance {name}: both .mps and .jtask exist")),
        (false, false) => Err(format!("toy instance {name}: no .mps or .jtask file")),
    }
}

/// Provenance record of a toy instance.
pub fn toy_provenance_path(name: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?
        .join("small")
        .join(format!("{name}.provenance.json")))
}

/// CLP fixtures whose solver-reported optimum is expected to fail verification
/// (`test_corpus.clp_expected_verification_failures`; empty when absent).
pub fn clp_expected_verification_failures() -> Result<Vec<String>, String> {
    Ok(corpus_declaration()?
        .lists
        .get("clp_expected_verification_failures")
        .cloned()
        .unwrap_or_default())
}

/// The variational kinetics source instance an LP fixture was extracted from
/// (`test_corpus.lp_source_instances`).
pub fn lp_source_instance(fixture: &str) -> Result<String, String> {
    corpus_declaration()?
        .mappings
        .get("lp_source_instances")
        .and_then(|m| m.get(fixture))
        .cloned()
        .ok_or_else(|| format!("project.yml: no lp_source_instances entry for {fixture}"))
}

fn declared_list(key: &str) -> Result<Vec<String>, String> {
    let v = corpus_declaration()?;
    match v.lists.get(key) {
        Some(l) if !l.is_empty() => Ok(l.clone()),
        _ => Err(format!(
            "project.yml: test_corpus.{key} is missing or empty"
        )),
    }
}

/// MPS file of an LP corpus instance.
pub fn mps_path(instance: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?.join(format!("{instance}.mps")))
}

/// Provenance record of an LP corpus instance.
pub fn provenance_path(instance: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?.join(format!("{instance}.provenance.json")))
}

/// Provenance record of a CLP corpus instance.
pub fn clp_provenance_path(instance: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?
        .join("clp")
        .join(format!("{instance}.provenance.json")))
}

/// Provenance record of a derived QP fixture (`fixtures`/qp).
pub fn qp_provenance_path(instance: &str) -> Result<PathBuf, String> {
    Ok(fixtures_dir()?
        .join("qp")
        .join(format!("{instance}.provenance.json")))
}

struct CorpusDeclaration {
    fixtures: Option<String>,
    lists: BTreeMap<String, Vec<String>>,
    mappings: BTreeMap<String, BTreeMap<String, String>>,
}

/// Reads the `test_corpus` block of `project.yml`: scalar keys, lists of scalars and
/// one-level mappings of scalars. Only the block-style YAML this project writes is understood.
fn corpus_declaration() -> Result<CorpusDeclaration, String> {
    let path = project_yml();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut in_corpus = false;
    let mut current: Option<String> = None;
    let mut out = CorpusDeclaration {
        fixtures: None,
        lists: BTreeMap::new(),
        mappings: BTreeMap::new(),
    };
    for line in text.lines() {
        let content = line.split(" #").next().unwrap_or("").trim_end();
        if content.trim().is_empty() || content.trim_start().starts_with('#') {
            continue;
        }
        let indent = content.len() - content.trim_start().len();
        let t = content.trim();
        if indent == 0 {
            in_corpus = t == "test_corpus:";
            current = None;
            continue;
        }
        if !in_corpus {
            continue;
        }
        if indent == 2 {
            current = None;
            if let Some(key) = t.strip_suffix(':') {
                current = Some(key.to_string());
            } else if let Some(v) = t.strip_prefix("fixtures:") {
                out.fixtures = Some(unquote(v.trim()).to_string());
            }
            continue;
        }
        let Some(key) = current.as_ref() else {
            continue;
        };
        if let Some(item) = t.strip_prefix("- ") {
            out.lists
                .entry(key.clone())
                .or_default()
                .push(unquote(item.trim()).to_string());
        } else if let Some((k, v)) = t.split_once(':') {
            out.mappings
                .entry(key.clone())
                .or_default()
                .insert(unquote(k.trim()).to_string(), unquote(v.trim()).to_string());
        }
    }
    Ok(out)
}

fn unquote(s: &str) -> &str {
    s.trim_matches(|c| c == '"' || c == '\'')
}

// ---------------------------------------------------------------------------------------------
// Provenance

/// Fields every corpus provenance record carries (data-model.md, Fixture provenance record).
pub const CORPUS_PROVENANCE_FIELDS: &[&str] = &[
    "instance",
    "source_repository",
    "source_commit",
    "source_file",
    "source_sha256",
    "extracting_script",
    "optsolver_commit",
    "m",
    "n",
    "nnz",
    "objective_sense",
    "seeds",
    "citation",
    "licence",
    "mps_sha256",
    "round_trip",
];

/// A provenance record: the whole JSON object, with typed accessors.
#[derive(Debug, Clone)]
pub struct Provenance {
    pub fields: BTreeMap<String, Json>,
}

impl Provenance {
    /// Reads a provenance record.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match parse_json(&text).map_err(|e| format!("{}: {e}", path.display()))? {
            Json::Object(fields) => Ok(Self { fields }),
            _ => Err(format!("{}: not a JSON object", path.display())),
        }
    }

    /// Names in `required` that are absent or null.
    pub fn missing(&self, required: &[&str]) -> Vec<String> {
        required
            .iter()
            .filter(|k| matches!(self.fields.get(**k), None | Some(Json::Null)))
            .map(|k| k.to_string())
            .collect()
    }

    /// A string field.
    pub fn str(&self, key: &str) -> Result<&str, String> {
        match self.fields.get(key) {
            Some(Json::String(s)) => Ok(s),
            other => Err(format!(
                "provenance field {key}: expected a string, got {other:?}"
            )),
        }
    }

    /// A non-negative integer field.
    #[allow(clippy::cast_possible_truncation)] // integral and below 2^53, checked first
    pub fn usize(&self, key: &str) -> Result<usize, String> {
        match self.fields.get(key) {
            Some(Json::Number(v)) if *v >= 0.0 && v.fract() == 0.0 && *v < 9.0e15 => {
                Ok(*v as usize)
            }
            other => Err(format!(
                "provenance field {key}: expected a non-negative integer, got {other:?}"
            )),
        }
    }

    /// The recorded objective seeds.
    #[allow(clippy::cast_possible_truncation)] // integral and below 2^53, checked first
    pub fn seeds(&self) -> Result<Vec<u64>, String> {
        match self.fields.get("seeds") {
            Some(Json::Array(a)) => a
                .iter()
                .map(|v| match v {
                    Json::Number(x) if *x >= 0.0 && x.fract() == 0.0 && *x < 9.0e15 => {
                        Ok(*x as u64)
                    }
                    other => Err(format!("seed {other:?} is not a non-negative integer")),
                })
                .collect(),
            other => Err(format!(
                "provenance field seeds: expected an array, got {other:?}"
            )),
        }
    }

    /// A nested object's string field, for example `array_sha256.b`.
    pub fn nested_str(&self, key: &str, inner: &str) -> Result<&str, String> {
        match self.fields.get(key) {
            Some(Json::Object(o)) => match o.get(inner) {
                Some(Json::String(s)) => Ok(s),
                other => Err(format!(
                    "provenance field {key}.{inner}: expected a string, got {other:?}"
                )),
            },
            other => Err(format!(
                "provenance field {key}: expected an object, got {other:?}"
            )),
        }
    }
}

/// A corpus fixture: the parsed MPS file and its provenance record, with the MPS file's
/// SHA-256 checked against the record.
pub struct CorpusFixture {
    pub instance: String,
    pub problem: MpsProblem,
    pub provenance: Provenance,
}

/// Loads a corpus fixture, checking the MPS file's SHA-256 against its provenance record.
pub fn load_corpus_fixture(instance: &str) -> Result<CorpusFixture, String> {
    let provenance = Provenance::load(&provenance_path(instance)?)?;
    let path = mps_path(instance)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let recorded = provenance.str("mps_sha256")?;
    let actual = sha256_hex(&bytes);
    if actual != recorded {
        return Err(format!(
            "{}: SHA-256 {actual} differs from recorded {recorded}",
            path.display()
        ));
    }
    drop(bytes);
    let problem = read_mps_file(&path)?;
    Ok(CorpusFixture {
        instance: instance.to_string(),
        problem,
        provenance,
    })
}

// ---------------------------------------------------------------------------------------------
// SHA-256 (FIPS 180-4)

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Incremental SHA-256.
#[derive(Clone)]
pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// A fresh hasher.
    pub fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0; 64],
            buf_len: 0,
            total: 0,
        }
    }

    /// Absorbs bytes.
    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len < 64 {
                return;
            }
            let block = self.buf;
            self.compress(&block);
            self.buf_len = 0;
        }
        let mut chunks = data.chunks_exact(64);
        for block in &mut chunks {
            self.compress(block.try_into().expect("64-byte block"));
        }
        let rest = chunks.remainder();
        self.buf[..rest.len()].copy_from_slice(rest);
        self.buf_len = rest.len();
    }

    /// The digest.
    pub fn finish(mut self) -> [u8; 32] {
        let bit_len = self.total.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        let pad_zeros = (119 - self.buf_len) % 64;
        pad.extend(std::iter::repeat_n(0u8, pad_zeros));
        pad.extend_from_slice(&bit_len.to_be_bytes());
        let total = self.total;
        self.update(&pad);
        self.total = total;
        debug_assert_eq!(self.buf_len, 0);
        let mut out = [0u8; 32];
        for (o, w) in out.chunks_exact_mut(4).zip(self.h) {
            o.copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (t, word) in block.chunks_exact(4).enumerate() {
            w[t] = u32::from_be_bytes(word.try_into().expect("4 bytes"));
        }
        for t in 16..64 {
            let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
            let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
            w[t] = w[t - 16]
                .wrapping_add(s0)
                .wrapping_add(w[t - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for t in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[t])
                .wrapping_add(w[t]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.h.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

/// Lower-case hexadecimal of a digest.
pub fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of bytes as lower-case hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finish())
}

/// SHA-256 of the little-endian bytes of a slice of doubles.
pub fn sha256_f64_le(v: &[f64]) -> String {
    let mut h = Sha256::new();
    for x in v {
        h.update(&x.to_le_bytes());
    }
    hex(&h.finish())
}

/// SHA-256 of the little-endian bytes of a slice of indices written as 64-bit integers.
pub fn sha256_usize_le(v: &[usize]) -> String {
    let mut h = Sha256::new();
    for x in v {
        h.update(&(*x as u64).to_le_bytes());
    }
    hex(&h.finish())
}

// ---------------------------------------------------------------------------------------------
// Minimal JSON

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

/// Parses a JSON document.
pub fn parse_json(text: &str) -> Result<Json, String> {
    let mut p = JsonParser {
        s: text.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("JSON: trailing data at byte {}", p.i));
    }
    Ok(v)
}

struct JsonParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn err<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("JSON: {msg} at byte {}", self.i))
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                let mut map = BTreeMap::new();
                self.ws();
                if self.eat("}") {
                    return Ok(Json::Object(map));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    if !self.eat(":") {
                        return self.err("expected ':'");
                    }
                    let v = self.value()?;
                    if map.insert(key.clone(), v).is_some() {
                        return self.err(&format!("duplicate key {key}"));
                    }
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        return Ok(Json::Object(map));
                    }
                    return self.err("expected ',' or '}'");
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.eat("]") {
                    return Ok(Json::Array(a));
                }
                loop {
                    a.push(self.value()?);
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        return Ok(Json::Array(a));
                    }
                    return self.err("expected ',' or ']'");
                }
            }
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(_) if self.eat("true") => Ok(Json::Bool(true)),
            Some(_) if self.eat("false") => Ok(Json::Bool(false)),
            Some(_) if self.eat("null") => Ok(Json::Null),
            Some(_) => {
                let start = self.i;
                while self.i < self.s.len()
                    && matches!(
                        self.s[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?;
                match t.parse::<f64>() {
                    Ok(v) if !t.is_empty() => Ok(Json::Number(v)),
                    _ => self.err(&format!("bad value {t:?}")),
                }
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if !self.eat("\"") {
            return self.err("expected a string");
        }
        let mut out = Vec::new();
        loop {
            match self.s.get(self.i) {
                None => return self.err("unterminated string"),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(out).map_err(|e| e.to_string());
                }
                Some(b'\\') => {
                    let esc = self.s.get(self.i + 1).copied();
                    self.i += 2;
                    let ch = match esc {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        Some(b'r') => '\r',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'u') => {
                            let hex = self
                                .s
                                .get(self.i..self.i + 4)
                                .ok_or("JSON: short \\u escape")?;
                            let code = u32::from_str_radix(
                                std::str::from_utf8(hex).map_err(|e| e.to_string())?,
                                16,
                            )
                            .map_err(|e| e.to_string())?;
                            self.i += 4;
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                Some(&b) => {
                    out.push(b);
                    self.i += 1;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Self-tests, registered by tests/corpus.rs.

/// Trials checking SHA-256, the JSON reader and the corpus declaration reader.
pub fn self_tests() -> Vec<super::Trial> {
    use super::{Trial, ensure, fail};
    vec![
        Trial::test("fixtures::sha256_known_vectors", || {
            let cases: [(&[u8], &str); 3] = [
                (
                    b"",
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                ),
                (
                    b"abc",
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                ),
                (
                    b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
                ),
            ];
            for (input, want) in cases {
                ensure(sha256_hex(input) == want, format!("sha256({input:?})"))?;
            }
            // Incremental updates across block boundaries agree with one-shot hashing.
            let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
            let mut h = Sha256::new();
            for piece in data.chunks(37) {
                h.update(piece);
            }
            ensure(hex(&h.finish()) == sha256_hex(&data), "incremental hashing")
        }),
        Trial::test("fixtures::json_reader", || {
            let v = parse_json(
                r#"{"a": "x\"yé", "n": -1.5e2, "s": [1, 2, 3], "o": {"k": true}, "z": null}"#,
            )
            .map_err(fail)?;
            let Json::Object(m) = v else {
                return Err(fail("not an object"));
            };
            ensure(
                m["a"] == Json::String("x\"y\u{e9}".into()),
                "string escapes",
            )?;
            ensure(m["n"] == Json::Number(-150.0), "number")?;
            ensure(
                m["s"]
                    == Json::Array(vec![
                        Json::Number(1.0),
                        Json::Number(2.0),
                        Json::Number(3.0),
                    ]),
                "array",
            )?;
            ensure(m["z"] == Json::Null, "null")?;
            for bad in [
                "{",
                r#"{"a" 1}"#,
                "[1,]",
                r#"{"a":1} x"#,
                r#"{"a":1,"a":2}"#,
            ] {
                ensure(parse_json(bad).is_err(), format!("accepted {bad:?}"))?;
            }
            Ok(())
        }),
        Trial::test("fixtures::corpus_declaration", || {
            for names in [
                toy_instances(),
                lp_instances(),
                qp_instances(),
                clp_instances(),
                clp_heavy_instances(),
            ] {
                let names = names.map_err(fail)?;
                ensure(
                    names.iter().all(|n| !n.is_empty() && !n.contains(' ')),
                    format!("{names:?}"),
                )?;
            }
            for lp in lp_instances().map_err(fail)? {
                let source = lp_source_instance(&lp).map_err(fail)?;
                ensure(!source.is_empty(), format!("{lp}: empty source instance"))?;
            }
            ensure(
                fixtures_dir().map_err(fail)?.ends_with("tests/fixtures"),
                "fixtures dir",
            )
        }),
    ]
}
