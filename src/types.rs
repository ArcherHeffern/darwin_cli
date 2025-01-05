use std::{collections::HashMap, io::Error, time::Duration};

pub struct TestResults {
    pub student: String,
    pub test: String,
    pub state: TestState,
}

#[derive(Debug)]
pub enum TestState {
    CompilationError,
    Ok { results: Vec<TestResult> },
}

#[derive(Debug)]
pub enum TestResultError {
    IOError(Error),
    TestsNotRun,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TestResult {
    pub name: String,
    pub classname: String,
    pub time: Duration,
    pub msg: StatusMsg,
}

impl ToString for TestResult {
    fn to_string(&self) -> String {
        format!("{}: {:?}", self.name, self.msg)
    }
}

#[derive(Debug, PartialEq)]
pub enum StatusMsg {
    None,
    Failure {
        message: Option<String>,
        type_: String,
        full_message: Option<String>
    },
    Error {
        message: Option<String>,
        type_: String,
        full_message: Option<String>
    },
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

    pub fn group_by_classname(&self) -> Option<HashMap<String, Vec<&TestResult>>> {
        match &self.state {
            TestState::CompilationError => None,
            TestState::Ok { results } => {
                let mut m: HashMap<String, Vec<&TestResult>> = HashMap::new();
                for result in results.iter() {
                    m.entry(result.classname.clone())
                        .and_modify(|e| e.push(result))
                        .or_default();
                }
                Some(m)
            }
        }
    }

    pub fn everything(&self) -> String {
        format!("{:?}", self.state)
    }

    pub fn summarize_by_classname(&self) -> Option<HashMap<String, (i32, i32, i32)>> {
        match self.group_by_classname() {
            None => None,
            Some(s) => {
                let mut m: HashMap<String, (i32, i32, i32)> = HashMap::new();
                for (_, v) in s.iter() {
                    for res in v.iter() {
                        m.entry(res.classname.clone())
                            .and_modify(|item| {
                                match res.msg {
                                    StatusMsg::None => item.0 += 1,
                                    StatusMsg::Error { .. } => item.1 += 1,
                                    StatusMsg::Failure { .. } => item.2 += 1,
                                };
                            })
                            .or_insert((0, 0, 0));
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
