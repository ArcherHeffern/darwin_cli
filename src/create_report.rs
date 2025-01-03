use std::{fs::{self, create_dir, remove_dir_all}, io::{Error, ErrorKind, Result}, path::{Path, PathBuf}};

use serde::Serialize;
use tempfile::tempdir;
use tinytemplate::TinyTemplate;

use crate::{config::{darwin_root, student_diff_file, tests_ran_file}, list_students::list_students, list_tests::list_tests, util::{file_contains_line, flatten_move_recursive, list_files_recursively, recreate_student_main }};


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

fn _create_report(report_root: &Path, tests: &Vec<String>) -> Result<()> {
    report_initialize(report_root).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to initialize report: {}", e)))?;
    let students = list_students();
    if students.is_empty() {
        return Ok(());
    }
    create_report_student_list(&report_root.join("index.html"), &students)?;
    let mut prev_student = "";
    for i in 0..students.len()-1 {
        let student = students[i].as_str();
        create_student_report(report_root, tests, &prev_student, &student, &students[i+1])?;
        prev_student = student;
    }
    create_student_report(report_root, tests, &prev_student, &students[students.len()-1], "")?;
    Ok(())
}

#[derive(Serialize)]
struct StudentListContext<'a> {
    students: &'a[String]
}

fn report_initialize(report_root: &Path) -> Result<()> {
    create_dir(report_root)?;
    create_dir(report_root.join("results"))?;
    create_dir(report_root.join("file_trees"))?;
    create_dir(report_root.join("students"))?;
    create_dir(report_root.join("styles"))?;
    Ok(())
}

fn create_report_student_list(dest: &Path, students: &[String]) -> Result<()> {

    let mut tt = TinyTemplate::new();
    tt.add_template("student_list", include_str!("../template/index.html")).map_err(|e|Error::new(ErrorKind::Other, e.to_string()))?;

    let rendered = tt.render("student_list", &StudentListContext{ students }).map_err(|e|{Error::new(ErrorKind::Other, e.to_string())})?;

    fs::write(dest, rendered.as_bytes())
}

fn create_student_report(report_root: &Path, tests: &Vec<String>, prev_student: &str, student: &str, next_student: &str) -> Result<()> {
    _create_student_report(report_root, tests, prev_student, student, next_student)
}

fn _create_student_report(report_root: &Path, tests: &Vec<String>, prev_student: &str, student: &str, next_student: &str) -> Result<()> {
    let diff_path = student_diff_file(student);
    let student_dir = &report_root.join("students").join(student);
    let tmpdir = tempdir()?;
    recreate_student_main(&diff_path, tmpdir.path(), tmpdir.path())?;
    let file_paths = list_files_recursively(tmpdir.path());
    for file in file_paths.iter() {
        let code = fs::read_to_string(&file)?;
        let student_report = create_student_report_html(code, &file_paths, tests, prev_student, student, next_student);
        fs::write(file, student_report)?;
    }
    flatten_move_recursive(tmpdir.path(), student_dir, None)?;

    Ok(())
}

fn create_student_report_html(code: String, file_paths: &Vec<PathBuf>, tests: &Vec<String>, prev_student: &str, student: &str, next_student: &str) -> String {
    code
}