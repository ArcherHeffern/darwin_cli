use std::{fs::OpenOptions, io::{self, BufReader}, path::Path, str::FromStr, time::Duration};

use xml::{name::OwnedName, attribute::OwnedAttribute, reader::XmlEvent, EventReader};

use crate::{util::file_contains_line, StatusMsg, TestResult, TestResults};

#[derive(Debug)]
pub enum TestResultError {
    IOError(io::Error),
    CompilationError,
    TestsNotRun
}

pub fn parse_test_results(darwin_path: &Path, student: &str, test: &str) -> Result<TestResults, TestResultError> {
    let mut test_results = TestResults { student: student.to_string(), test: test.to_string(), results: Vec::new() };
    let compile_error_path = Path::new(darwin_path).join("results").join("compile_errors");

    if file_contains_line(&compile_error_path, student).unwrap() {
        return Err(TestResultError::CompilationError);
    }

    let test_results_file_path = Path::new(darwin_path).join("results").join(format!("{}_{}", student, test));
    if !test_results_file_path.is_file() {
        return Err(TestResultError::TestsNotRun);
    }

    let test_results_file = OpenOptions::new().read(true).open(test_results_file_path).unwrap();
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
            Ok(XmlEvent::StartElement { name: _name, attributes, .. }) => {
                if _name == testcase {
                    name = get_attr(&attributes, "name").expect("XML Testcase must have name attribute");
                    classname = get_attr(&attributes, "classname").expect("XML Testcase must have classname attribute");
                    let time_str = get_attr(&attributes, "time").expect("XML Testcase must have time attribute");
                    let time_seconds = time_str.parse::<f32>().expect("XML Testcase time attribute must be parsable as a f32");
                    time = Duration::from_secs_f32(time_seconds);
                } else if _name == failure {
                    let message = get_attr(&attributes, "message");
                    let type_ = get_attr(&attributes, "type").expect(&format!("{}: XML Failure must have type attribute", name));
                    msg = StatusMsg::Failure { message, type_ };
                } else if _name == error {
                    let message = get_attr(&attributes, "message");
                    let type_ = get_attr(&attributes, "type").expect(&format!("{}: XML Failure must have type attribute", name));
                    msg = StatusMsg::Error { message, type_ };
                }
            }
            Ok(XmlEvent::EndElement { name: _name }) if _name == testcase => {
                let test_result = TestResult { name, classname, msg, time };
                test_results.results.push(test_result);

                name = String::new();
                classname = String::new();
                msg = StatusMsg::None;
                time = Duration::new(0, 0);
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return Err(TestResultError::IOError(io::Error::new(io::ErrorKind::Other, format!("Failed to parse {}'s {} test results", student, test))));
            }
            _ => {}
        }
    }

    Ok(test_results)
}

fn get_attr(owned_attributes: &[OwnedAttribute], attr: &str) -> Option<String> {
    owned_attributes.iter().find_map(|a| {
        if a.name==OwnedName::from_str(attr).unwrap() {
            return Some(a.value.clone());
        } 
        None
    })
}