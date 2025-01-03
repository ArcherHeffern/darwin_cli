use std::{fs::{self, create_dir, remove_dir, remove_dir_all, remove_file}, io::{Error, ErrorKind, Result}, path::Path};

use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::{config::{darwin_root, student_diff_file, student_project_file, tests_ran_file}, list_students::list_students, list_tests::list_tests, util::{file_contains_line, initialize_project, set_active_project}};


pub fn create_report(report_path: &Path, tests: &Vec<String>) -> Result<()> {
    if !darwin_root().is_dir() {
        return Err(Error::new(ErrorKind::NotFound, "Darwin project not initialized"));
    }
    if report_path.exists() {
        return Err(Error::new(ErrorKind::AlreadyExists, "report_path exists"));
    }
    if tests.is_empty() {
        return Err(Error::new(ErrorKind::AlreadyExists, "expected at least one test"));
    }
    let actual_tests = list_tests();
    for test in tests {
        if !actual_tests.contains(test) {
            return Err(Error::new(ErrorKind::NotFound, format!("{} is not a test", test)))
        }
        if !file_contains_line(&tests_ran_file(), test)? {
            println!("Warning! {} is a test but it wasn't run for all students", test);
            // return Err(Error::new(ErrorKind::NotFound, format!("{} is a test but wasn't run for all students", test)))
        }
    }
    _create_report(report_path, tests).map_err(|e| {
        if report_path.exists() {
            remove_dir_all(report_path).expect("Create report and deleting report directory during cleanup failed");
        }
        e
    })
}

fn _create_report(report_path: &Path, tests: &Vec<String>) -> Result<()> {
    create_dir(report_path)?;
    let students = list_students();
    if students.is_empty() {
        return Ok(());
    }
    create_report_student_list(&report_path.join("index.html"), &students)?;
    let mut prev_student = "";
    for i in 0..students.len()-1 {
        let student = students[i].as_str();
        create_student_report(report_path, tests, &prev_student, &student, &students[i+1])?;
        prev_student = student;
    }
    create_student_report(report_path, tests, &prev_student, &students[students.len()-1], "")?;
    Ok(())
}

#[derive(Serialize)]
struct StudentListContext<'a> {
    students: &'a[String]
}

fn create_report_student_list(dest: &Path, students: &[String]) -> Result<()> {

    let mut tt = TinyTemplate::new();
    tt.add_template("student_list", include_str!("../template/index.html")).map_err(|e|Error::new(ErrorKind::Other, e.to_string()))?;

    let rendered = tt.render("student_list", &StudentListContext{ students }).map_err(|e|{Error::new(ErrorKind::Other, e.to_string())})?;

    fs::write(dest, rendered.as_bytes())
}

fn create_student_report(report_root: &Path, tests: &Vec<String>, prev_student: &str, student: &str, next_student: &str) -> Result<()> {
    let student_project_path = student_project_file(student);
    if student_project_path.is_dir() {
        remove_dir_all(&student_project_path)?;
    } else if student_project_path.is_file() {
        remove_file(&student_project_path)?;
    }
    _create_student_report(report_root, tests, prev_student, student, &student_project_path, next_student)
}

fn _create_student_report(report_root: &Path, tests: &Vec<String>, prev_student: &str, student: &str, student_project_path: &Path, next_student: &str) -> Result<()> {
    let diff_path = student_diff_file(student);
    initialize_project(&student_project_path)?;
    set_active_project(&student_project_path, &diff_path)?;
    // Create the file hierarchy
    for test in tests {
        create_student_report_for_test(report_root, test, prev_student, student, next_student);
    }

    remove_dir_all(student_project_path)?;

    Ok(())
}

fn create_student_report_for_test(report_root: &Path, test: &str, prev_student: &str, student: &str, next_student: &str) {

}