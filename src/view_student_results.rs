use std::{
    fs::OpenOptions,
    io::{self, BufReader},
    path::Path,
    str::FromStr,
    time::Duration,
};

use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent, EventReader};

use crate::{
    config::{compile_errors_file, student_result_file}, list_students::list_students, project_runner::Project, types::{StatusMsg, TestResult, TestResultError, TestResults, TestState}, util::file_contains_line
};

pub fn parse_test_results(project: &Project, student: &str, test: &str) -> Result<TestResults, TestResultError> {
    if !list_students().iter().any(|s| s == student) {
        return Err(TestResultError::IOError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Student {} not recognized", student),
        )));
    }

    if !project.list_tests().contains(test) {
        return Err(TestResultError::IOError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Test {} not recognized", test),
        )));
    }

    _parse_test_results(project, student, test)
}

fn _parse_test_results(project: &Project, student: &str, test: &str) -> Result<TestResults, TestResultError> {
    let mut out = TestResults {
        student: student.to_string(),
        test: test.to_string(),
        state: TestState::CompilationError,
    };

    if file_contains_line(&compile_errors_file(), student).unwrap() {
        out.state = TestState::CompilationError;
        return Ok(out);
    }

    let result_path = student_result_file(student, test);
    if !result_path.is_file() {
        return Err(TestResultError::TestsNotRun);
    }

    project.parse_result_report(&result_path, student, test).map(|results| {
        out.state = TestState::Ok { results };
        out
    })
}

