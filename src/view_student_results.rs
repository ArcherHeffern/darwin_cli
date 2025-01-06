use std::{
    fs::OpenOptions,
    io::{self, BufReader},
    path::Path,
    str::FromStr,
    time::Duration,
};

use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent, EventReader};

use crate::{
    config::{compile_errors_file, student_result_file},
    list_students::list_students,
    list_tests::list_tests,
    types::{StatusMsg, TestResult, TestResultError, TestResults, TestState},
    util::file_contains_line,
};

pub fn parse_test_results(student: &str, test: &str) -> Result<TestResults, TestResultError> {
    if !list_students().iter().any(|s| s == student) {
        return Err(TestResultError::IOError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Student {} not recognized", student),
        )));
    }

    if !list_tests().contains(test) {
        return Err(TestResultError::IOError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Test {} not recognized", test),
        )));
    }

    _parse_test_results(student, test)
}

fn _parse_test_results(student: &str, test: &str) -> Result<TestResults, TestResultError> {
    let mut out = TestResults {
        student: student.to_string(),
        test: test.to_string(),
        state: TestState::CompilationError,
    };

    if file_contains_line(&compile_errors_file(), student).unwrap() {
        out.state = TestState::CompilationError;
        return Ok(out);
    }

    let result_file = student_result_file(student, test);
    if !result_file.is_file() {
        return Err(TestResultError::TestsNotRun);
    }

    parse_surefire_report(&result_file, student, test).map(|results| {
        out.state = TestState::Ok { results };
        out
    })
}

fn get_attr(owned_attributes: &[OwnedAttribute], attr: &str) -> Option<String> {
    owned_attributes.iter().find_map(|a| {
        if a.name == OwnedName::from_str(attr).unwrap() {
            return Some(a.value.clone());
        }
        None
    })
}

fn parse_surefire_report(
    report_path: &Path,
    student: &str,
    test: &str,
) -> Result<Vec<TestResult>, TestResultError> {
    let mut out = Vec::new();

    let test_results_file = OpenOptions::new().read(true).open(report_path).unwrap();
    let test_results_file = BufReader::new(test_results_file);

    let parser = EventReader::new(test_results_file);
    let testcase = OwnedName::from_str("testcase").unwrap();
    let failure = OwnedName::from_str("failure").unwrap();
    let error = OwnedName::from_str("error").unwrap();

    let mut name = String::new();
    let mut classname = String::new();
    let mut msg = StatusMsg::None;
    let mut time = Duration::new(0, 0);
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name: _name,
                attributes,
                ..
            }) => {
                if _name == testcase {
                    name = get_attr(&attributes, "name")
                        .expect("XML Testcase must have name attribute");
                    classname = get_attr(&attributes, "classname")
                        .expect("XML Testcase must have classname attribute");
                    let time_str = get_attr(&attributes, "time")
                        .expect("XML Testcase must have time attribute");
                    let time_seconds = time_str
                        .parse::<f32>()
                        .expect("XML Testcase time attribute must be parsable as a f32");
                    time = Duration::from_secs_f32(time_seconds);
                } else if _name == failure {
                    let message = get_attr(&attributes, "message");
                    let type_ = get_attr(&attributes, "type")
                        .unwrap_or_else(|| panic!("{}: XML Failure must have type attribute", name));
                    let full_message = None;
                    msg = StatusMsg::Failure {
                        message,
                        type_,
                        full_message,
                    };
                } else if _name == error {
                    let message = get_attr(&attributes, "message");
                    let type_ = get_attr(&attributes, "type")
                        .unwrap_or_else(|| panic!("{}: XML Failure must have type attribute", name));
                    let full_message = None;
                    msg = StatusMsg::Error {
                        message,
                        type_,
                        full_message,
                    };
                }
            }
            Ok(XmlEvent::CData(data)) => match msg {
                StatusMsg::None => {}
                StatusMsg::Error {
                    ref mut full_message,
                    ..
                } => {
                    let _ = full_message.insert(data);
                }
                StatusMsg::Failure {
                    ref mut full_message,
                    ..
                } => {
                    let _ = full_message.insert(data);
                }
            },
            Ok(XmlEvent::EndElement { name: _name }) if _name == testcase => {
                out.push(TestResult {
                    name,
                    classname,
                    time,
                    msg,
                });

                name = String::new();
                classname = String::new();
                msg = StatusMsg::None;
                time = Duration::new(0, 0);
            }
            Err(e) => {
                return Err(TestResultError::IOError(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to parse {}'s {} test results: {}", student, test, e),
                )));
            }
            _ => {}
        }
    }
    Ok(out)
}
