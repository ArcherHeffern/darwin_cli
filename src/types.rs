use std::{collections::HashMap, io::Error, time::Duration};

use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TestResults {
    pub student: String,
    pub test: String,
    pub state: TestState,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub enum TestState {
    CompilationError,
    Ok { results: Vec<TestResult> },
}

#[derive(Debug)]
pub enum TestResultError {
    IOError(Error),
    TestsNotRun,
}

impl TestResults {
    pub fn summary(&self) -> (bool, usize, usize, usize) {
        match &self.state {
            TestState::CompilationError => (true, 0, 0, 0),
            TestState::Ok { results } => {
                let num_correct = results.iter().filter(|r| r.msg == StatusMsg::None).count();
                let num_errored = results
                    .iter()
                    .filter(|r| matches!(r.msg, StatusMsg::Error { .. }))
                    .count();
                let num_failed = results
                    .iter()
                    .filter(|r| matches!(r.msg, StatusMsg::Failure { .. }))
                    .count();
                (false, num_correct, num_errored, num_failed)
            }
        }
        // Correct, errored, failed
    }
    pub fn summarize(&self) -> String {
        let summary = self.summary();
        format!(
            "{}_{}: Compilation Error: {}, Correct: {}, Errored: {}, Failed: {}",
            self.student, self.test, summary.0, summary.1, summary.2, summary.3
        )
    }

    pub fn summarize_by_classname(&self) -> Option<HashMap<String, (i32, i32, i32)>> {
        match &self.state {
            TestState::CompilationError => None,
            TestState::Ok { results } => {
                let mut m: HashMap<String, (i32, i32, i32)> = HashMap::new();
                for result in results.iter() {
                    if !m.contains_key(&result.classname) {
                        m.insert(result.classname.clone(), (0, 0, 0));
                    }
                    let index = match &result.msg {
                        StatusMsg::None => 0,
                        StatusMsg::Error { .. } => 1,
                        StatusMsg::Failure { .. } => 2,
                    };
                    if let Some(entry) = m.get_mut(&result.classname) {
                        match index {
                            0 => entry.0 += 1,
                            1 => entry.1 += 1,
                            2 => entry.2 += 1,
                            _ => {} // In case of an unexpected index, do nothing
                        }
                    }
                }
                Some(m)
            }
        }
    }

    pub fn print(&self) -> String {
        let m = self.summarize_by_classname();
        format!("{}_{} {:?}", self.student, self.test, m)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TestResult {
    pub name: String,
    pub classname: String,
    pub time: Duration,
    pub msg: StatusMsg,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub enum StatusMsg {
    None,
    Failure {
        message: Option<String>,
        type_: String,
    },
    Error {
        message: Option<String>,
        type_: String,
    },
}
