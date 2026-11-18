//! Shared test helpers: a deterministic, offline [`CommandRunner`] double.
//!
//! Included by multiple test crates; each uses a different subset, so unused
//! helpers are expected per-crate.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use alfred_brave::runner::CommandRunner;
use anyhow::{bail, Result};

/// A single recorded invocation of [`FakeRunner`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
enum Response {
    Success(String),
    Failure(String),
}

/// A [`CommandRunner`] that returns canned output keyed by program name and
/// records every call, so tests stay offline and can assert on invocations.
///
/// Responses queued for a program are consumed in order; the last one keeps
/// answering once the queue is down to it.
pub struct FakeRunner {
    responses: RefCell<HashMap<String, VecDeque<Response>>>,
    pub calls: RefCell<Vec<Call>>,
}

impl FakeRunner {
    pub fn new() -> Self {
        Self {
            responses: RefCell::new(HashMap::new()),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Queue a successful run of `program` printing `stdout`.
    pub fn with_success(self, program: &str, stdout: &str) -> Self {
        self.push(program, Response::Success(stdout.to_owned()));

        self
    }

    /// Queue a failed run of `program`.
    pub fn with_failure(self, program: &str, message: &str) -> Self {
        self.push(program, Response::Failure(message.to_owned()));

        self
    }

    /// Every recorded call.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }

    /// The programs invoked so far, in order.
    pub fn programs_called(&self) -> Vec<String> {
        self.calls().into_iter().map(|call| call.program).collect()
    }

    /// The trailing arguments of a JXA call (after `-l JavaScript -e <script>`).
    pub fn jxa_args(call: &Call) -> Vec<String> {
        assert_eq!(&call.args[..3], ["-l", "JavaScript", "-e"]);

        call.args[4..].to_vec()
    }

    fn push(&self, program: &str, response: Response) {
        self.responses
            .borrow_mut()
            .entry(program.to_owned())
            .or_default()
            .push_back(response);
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        self.calls.borrow_mut().push(Call {
            program: program.to_owned(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        });

        let mut responses = self.responses.borrow_mut();
        let Some(queue) = responses.get_mut(program) else {
            bail!("unexpected program: `{program}`");
        };

        let response = if queue.len() > 1 {
            queue.pop_front().expect("queue is not empty")
        } else {
            queue.front().cloned().expect("queue is not empty")
        };

        match response {
            Response::Success(stdout) => Ok(stdout),
            Response::Failure(message) => bail!("{message}"),
        }
    }
}
