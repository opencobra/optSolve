//! The one shared test module (constitution II; research R14). Every solver test is written
//! once against `optsolver::Backend` and registered here once per declared backend. A backend
//! is *available* when it is compiled in (Cargo feature) and an environment can be created
//! (licence present); otherwise its trials are reported as ignored with the reason, never as
//! passed (SC-002).

#![allow(dead_code, missing_docs)]

pub mod fake_backend;
pub mod fixtures;
pub mod mps;
pub mod objectives;
pub mod us3;

pub use libtest_mimic::{Arguments, Failed, Trial};
use optsolver::Backend;

/// Constructor of a backend instance.
pub type MakeBackend = fn() -> Result<Box<dyn Backend>, String>;

/// A backend this project declares, whether or not it is compiled in.
#[derive(Clone)]
pub struct DeclaredBackend {
    pub name: &'static str,
    pub make: Option<MakeBackend>,
}

/// Whether a declared backend can run tests here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Available,
    NotCompiled,
    Unavailable(String),
}

/// Every backend the project declares.
pub fn declared_backends() -> Vec<DeclaredBackend> {
    vec![DeclaredBackend {
        name: "gurobi",
        make: gurobi_maker(),
    }]
}

#[cfg(feature = "gurobi")]
fn gurobi_maker() -> Option<MakeBackend> {
    Some(|| {
        optsolver::gurobi::Env::new()
            .map(|e| Box::new(e) as Box<dyn Backend>)
            .map_err(|e| e.to_string())
    })
}

#[cfg(not(feature = "gurobi"))]
fn gurobi_maker() -> Option<MakeBackend> {
    None
}

/// A test written once against the contract.
pub type BackendTest = fn(&dyn Backend) -> Result<(), Failed>;

/// Backends with their availability, detected once per test binary.
pub struct Registry {
    pub entries: Vec<(DeclaredBackend, Availability)>,
}

impl Registry {
    /// Detects availability of every declared backend.
    pub fn detect() -> Self {
        Self::from_declared(declared_backends())
    }

    /// Detects availability of the given backends (used by the harness self-test).
    pub fn from_declared(declared: Vec<DeclaredBackend>) -> Self {
        let entries = declared
            .into_iter()
            .map(|b| {
                let a = match b.make {
                    None => Availability::NotCompiled,
                    Some(make) => match make() {
                        Ok(_) => Availability::Available,
                        Err(e) => Availability::Unavailable(e),
                    },
                };
                (b, a)
            })
            .collect();
        Self { entries }
    }

    /// One trial per declared backend; unavailable backends give ignored trials with the
    /// reason in the name.
    pub fn trials(&self, name: &str, test: BackendTest) -> Vec<Trial> {
        self.entries
            .iter()
            .map(|(b, a)| {
                let label = format!("{name}::{}", b.name);
                match (a, b.make) {
                    (Availability::Available, Some(make)) => Trial::test(label, move || {
                        let backend = make().map_err(Failed::from)?;
                        test(&*backend)
                    }),
                    (Availability::NotCompiled, _) | (Availability::Available, None) => {
                        Trial::test(format!("{label} [{}: not compiled]", b.name), || Ok(()))
                            .with_ignored_flag(true)
                    }
                    (Availability::Unavailable(reason), _) => {
                        let reason = reason.replace('\n', " ");
                        Trial::test(
                            format!("{label} [{}: unavailable: {reason}]", b.name),
                            || Ok(()),
                        )
                        .with_ignored_flag(true)
                    }
                }
            })
            .collect()
    }
}

/// Converts any displayable error into a test failure.
pub fn fail(msg: impl std::fmt::Display) -> Failed {
    Failed::from(msg.to_string())
}

/// Asserts a condition with a message.
pub fn ensure(cond: bool, msg: impl std::fmt::Display) -> Result<(), Failed> {
    if cond { Ok(()) } else { Err(fail(msg)) }
}
