use std::{collections::HashSet, path::Path};

use crate::{
    create_project, list_students, run_tests::process_diff_tests, util::{self, is_valid_test_string}, view_student_results, TestResult
};

pub fn create_darwin(
    project_path: &Path,
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) {
    create_project::init_darwin(
        project_path,
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    )
    .unwrap();
}
pub fn list_students(project_path: &Path) {
    for student in list_students::list_students(project_path) {
        println!("{}", student);
    }
}

pub fn list_tests(project_path: &Path) {
    for test in crate::list_tests::list_tests(project_path) {
        println!("{}", test);
    }
}

pub fn run_tests_for_student(project_path: &Path, student: &str, tests: &str, copy_ignore_set: &HashSet<&str>) {
    if !is_valid_test_string(project_path, tests) {
        eprintln!("Expected comma separated list of valid tests. eg: 'test1,test2,test3");
        return;
    }
    if !list_students::list_students(project_path).contains(student) {
        eprintln!("Student {} was not found", student);
        return;
    }

    match util::find_student_diff_file(project_path, student).take() {
        Some(diff_path) => {
            // Ensure tests are valid tests
            if let Err(e) = process_diff_tests(
                project_path,
                student,
                &Path::new(&diff_path),
                tests,
                &copy_ignore_set,
            ) {
                eprintln!("{}", e);
            }
        }
        None => {
            eprintln!("Not a student");
        }
    }
}

pub fn run_tests(project_path: &Path, tests: &str, copy_ignore_set: &HashSet<&str>) {
    if !is_valid_test_string(project_path, tests) {
        eprintln!("Expected comma separated list of valid tests. eg: 'test1,test2,test3");
        return;
    }

    for diff_path in project_path.join("submission_diffs").read_dir().unwrap() {
        let diff_path = diff_path.unwrap().path();
        println!("Processing {}", diff_path.file_name().unwrap().to_str().unwrap());
        if let Err(e) = process_diff_tests(
            project_path,
            diff_path.file_name().unwrap().to_str().unwrap(),
            &Path::new(&diff_path),
            tests,
            &copy_ignore_set,
        ) {
            eprintln!("{}", e);
        }
    }
}

pub fn view_results() {}
pub fn view_student_submission() {}
pub fn view_student_result(project_path: &Path, student: &str, test: &str) {
     if !list_students::list_students(project_path).contains(student) {
        eprintln!("Student '{}' not recognized", student);
        return;
     }

     println!("{:?}", view_student_results::parse_test_results(project_path, student, test));
}
pub fn download_results() {}
