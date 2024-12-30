use std::{fs::OpenOptions, io::{self, BufReader}, path::Path};

use xml::{reader::XmlEvent, EventReader};

use crate::{util::file_contains_line, TestResult};


pub fn parse_test_results(project_path: &Path, student: &str, test: &str) -> Result<Option<TestResult>, io::Error> {
    let test_result = TestResult { student: student.to_string(), test: test.to_string(), s: crate::TestOk::CompileError };
    let compile_error_path = Path::new(project_path).join("results").join("compile_errors");

    if file_contains_line(&compile_error_path, student)? {
        return Ok(Some(test_result));
    }

    let test_results_file_path = Path::new(project_path).join("results").join(format!("{}_{}", student, test));
    if !test_results_file_path.is_file() {
        return Ok(None);
    }

    let test_results_file = OpenOptions::new().read(true).open(test_results_file_path)?;
    let test_results_file = BufReader::new(test_results_file);

    let parser = EventReader::new(test_results_file);
    let mut depth = 0;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                println!("{:spaces$}+{name}", "", spaces = depth * 2);
                depth += 1;
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                println!("{:spaces$}-{name}", "", spaces = depth * 2);
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    Ok(None)
}