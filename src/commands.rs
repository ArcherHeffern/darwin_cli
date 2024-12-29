use std::{collections::HashSet, path::Path};

use crate::{create_project, run_tests::process_diff_tests, util::{self, is_valid_test_string}};

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
    for entry in project_path.join("submission_diffs").read_dir().unwrap() {
        println!("{:?}", entry.unwrap().file_name());
    }
}

pub fn list_tests(project_path: &Path) {
    for test in util::list_tests(project_path) {
        println!("{}", test);
    }
}
pub fn run_test(project_path: &Path, student: &str, tests: &str, copy_ignore_set: &HashSet<&str>) {
    if !is_valid_test_string(project_path, tests) {
        eprintln!("Expected comma separated list of valid tests. eg: 'test1,test2,test3");
        return;
    }
    match util::find_student_diff_file(project_path, student).take() {
        Some(diff_path) => {
            // Ensure tests are valid tests
            if let Err(e) = process_diff_tests(
                project_path,
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
pub fn run_tests(tests: Vec<String>) {}
pub fn view_results() {}
pub fn view_student_submission() {}
pub fn view_result(student: String) {}
pub fn download_results() {}
