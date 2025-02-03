use std::collections::HashSet;
use std::fs::{rename, OpenOptions};
use std::path::Path;
use std::io::{BufReader, Error, ErrorKind, Result};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::Duration;

use xml::attribute::OwnedAttribute;
use xml::name::OwnedName;
use xml::reader::XmlEvent;
use xml::EventReader;

use crate::config::skel_dir;
use crate::types::{StatusMsg, TestResult, TestResultError};
use crate::util::dir_list_absolute_file_paths_recursively;

use super::Project;


pub fn compile(_: &Project, project_path: &Path) -> Result<()> {
    // mvn compile
    let mut compile_command = Command::new("mvn")
        .current_dir(project_path)
        .arg("test-compile")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = compile_command.wait()?;
    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "'mvn test-compile' failed"));
    }

    Ok(())
}

pub fn list_tests(_: &Project) -> HashSet<String> {
    let test_dir = skel_dir().join("src").join("test").join("java");
    let test_dir_str = test_dir.to_str().unwrap();
    let files = dir_list_absolute_file_paths_recursively(&test_dir);

    let mut out = HashSet::new();
    for file in files {
        if file.extension().map_or(false, |ext| ext != "java") {
            continue;
        }
        let file = file.strip_prefix(test_dir_str).unwrap();
        let file_name = file.to_string_lossy();
        let test_name = file_name.replace('/', ".");
        out.insert(test_name[..file_name.len() - 5].to_string());
    }

    out
}

/// Runs `mvn -Dtest={test_str} surefire:test`
/// Returns results file destination
pub fn run_test(_: &Project, project_path: &Path, test: &str) -> Result<()> {

    let mut run_tests_command = Command::new("mvn")
        .current_dir(project_path)
        .arg(format!("-Dtest={}", test))
        .arg("surefire:test")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    run_tests_command.wait()?;

    Ok(())
}

pub fn relocate_test_results(_: &Project, project_path: &Path, test: &str, dest_file: &Path) -> Result<()> {
    let results_filename_from = format!("TEST-{}.xml", test);
    let results_file_from = project_path
        .join("target")
        .join("surefire-reports")
        .join(results_filename_from);

    if !results_file_from.exists() {
        return Err(Error::new(ErrorKind::NotFound, "Results file was not found"));
    }
    rename(&results_file_from, dest_file).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to rename {:?} to {:?}: {}", results_file_from, dest_file, e)))?;
    Ok(())
}

pub fn parse_result_report(_: &Project, report_path: &Path, student: &str, test: &str) -> std::result::Result<Vec<TestResult>, TestResultError> {
    parse_surefire_report(report_path, student, test)
}

fn parse_surefire_report(
    report_path: &Path,
    student: &str,
    test: &str,
) -> std::result::Result<Vec<TestResult>, TestResultError> {
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
                    let type_ = get_attr(&attributes, "type").unwrap_or_else(|| {
                        panic!("{}: XML Failure must have type attribute", name)
                    });
                    let full_message = None;
                    msg = StatusMsg::Failure {
                        message,
                        type_,
                        full_message,
                    };
                } else if _name == error {
                    let message = get_attr(&attributes, "message");
                    let type_ = get_attr(&attributes, "type").unwrap_or_else(|| {
                        panic!("{}: XML Failure must have type attribute", name)
                    });
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
                return Err(TestResultError::IOError(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse {}'s {} test results: {}", student, test, e),
                )));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn get_attr(owned_attributes: &[OwnedAttribute], attr: &str) -> Option<String> {
    owned_attributes.iter().find_map(|a| {
        if a.name == OwnedName::from_str(attr).unwrap() {
            return Some(a.value.clone());
        }
        None
    })
}

// Parse test results

// Create universal test result format for report

// Display test results in report
