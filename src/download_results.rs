use std::{fs::{File, OpenOptions}, io::{BufWriter, Result}, path::Path};

use crate::{
    list_students::list_students, list_tests::list_tests, types::TestResultError, view_student_results::parse_test_results
};

pub fn download_results_summary(
    darwin_path: &Path,
    f: File,
    test: &str,
) -> Result<()> {
    let f = BufWriter::new(f);
    let mut wtr = csv::Writer::from_writer(f);
    let headers = vec![
        String::from("Name"),
        String::from("Status"),
        String::from("Correct"),
        String::from("Errored"),
        String::from("Failed"),
    ];

    wtr.write_record(&headers)?;
    for student in list_students() {
        let mut cur_row = vec![String::new(); headers.len()];
        cur_row[0] = student.clone();
        match parse_test_results(darwin_path, &student, test) {
            Ok(res) => {
                let summary = res.summary();
                if summary.0 {
                    cur_row[1] = String::from("Compile Error")
                } else {
                    cur_row[2] = format!("{}", summary.1);
                    cur_row[3] = format!("{}", summary.2);
                    cur_row[4] = format!("{}", summary.3);
                }
            }
            Err(e) => {
                cur_row[1] = match e {
                    TestResultError::IOError(er) => er.to_string(),
                    TestResultError::TestsNotRun => {
                        String::from("Tests not run")
                    }
                };
            }
        }
        wtr.write_record(cur_row)?;
    }
    Ok(())
}

pub fn download_results_by_classname(
    darwin_path: &Path,
    out_file: &Path,
    test: &str,
) -> Result<()> {
    let out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_file)?;
    let out_file = BufWriter::new(out_file);
    let mut wtr = csv::Writer::from_writer(out_file);
    let tests = list_tests();
    let mut headers = vec![String::from("Name"), String::from("Error")];
    for test in tests {
        headers.push(test);
    }

    wtr.write_record(&headers)?;

    for student in list_students() {
        let mut cur_row = vec![String::new(); headers.len()];
        cur_row[0] = student.clone();
        match parse_test_results(darwin_path, &student, test) {
            Ok(res) => {
                let summary = res.summarize_by_classname();
                match summary {
                    Some(summary) => {
                        for (i, header) in headers.iter().enumerate() {
                            if summary.contains_key(header) {
                                cur_row[i] = format!("{}", summary[header].0);
                            }
                        }
                    }
                    None => {
                        cur_row[1] = String::from("Compilation Error");
                    }
                }
            }
            Err(e) => {
                cur_row[1] = match e {
                    TestResultError::IOError(er) => er.to_string(),
                    TestResultError::TestsNotRun => {
                        String::from("Tests not run")
                    }
                };
            }
        }
        wtr.write_record(cur_row)?;
    }
    Ok(())
}
