use std::{error::Error, fs::File, io::BufWriter, path::Path};

use crate::{list_students::list_students, list_tests::list_tests, view_student_results::parse_test_results};

pub fn download_results_summary(project_path: &Path, f: File, test: &str) -> Result<(), Box<dyn Error>> {
    let f = BufWriter::new(f);
    let mut wtr = csv::Writer::from_writer(f);
    let headers = vec![String::from("Name"), String::from("Error"), String::from("Correct"), String::from("Errored"), String::from("Failed")];

    wtr.write_record(&headers)?;
    for student in list_students(project_path) {
        let mut cur_row = vec![String::new(); headers.len()];
        cur_row[0] = student.clone();
        match parse_test_results(project_path, &student, test) {
            Ok(res) => {
                let summary = res.summary();
                cur_row[2] = format!("{}", summary.0);
                cur_row[3] = format!("{}", summary.1);
                cur_row[4] = format!("{}", summary.2);
            }
            Err(e) => {
                cur_row[1] = match e {
                    crate::view_student_results::TestResultError::IOError(er) => {
                        er.to_string()
                    }
                    crate::view_student_results::TestResultError::TestsNotRun => {
                        String::from("Tests not run")
                    }
                    crate::view_student_results::TestResultError::CompilationError => {
                        String::from("Compilation error")
                    }
                };
            }
        }
        wtr.write_record(cur_row)?;
    }
    Ok(())
}

pub fn download_results_by_classname(project_path: &Path, f: File, test: &str) -> Result<(), Box<dyn Error>> {
    let f = BufWriter::new(f);
    let mut wtr = csv::Writer::from_writer(f);
    let tests = list_tests(project_path);
    let mut headers = vec![String::from("Name"), String::from("Error")];
    for test in tests {
        headers.push(test);
    }

    wtr.write_record(&headers)?;

    for student in list_students(project_path) {
        let mut cur_row = vec![String::new(); headers.len()];
        cur_row[0] = student.clone();
        match parse_test_results(project_path, &student, test) {
            Ok(res) => {
                let summary = res.summarize_by_classname();
                for (i, header) in headers.iter().enumerate() {
                    if summary.contains_key(header) {
                        cur_row[i] = format!("{}", summary[header].0);
                    }
                }
            }
            Err(e) => {
                cur_row[1] = match e {
                    crate::view_student_results::TestResultError::IOError(er) => {
                        er.to_string()
                    }
                    crate::view_student_results::TestResultError::TestsNotRun => {
                        String::from("Tests not run")
                    }
                    crate::view_student_results::TestResultError::CompilationError => {
                        String::from("Compilation error")
                    }
                };
            }
        }
        wtr.write_record(cur_row)?;
    }
    Ok(())
}