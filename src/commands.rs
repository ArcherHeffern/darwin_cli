use std::{collections::HashSet, fs::{create_dir, remove_dir, remove_file, OpenOptions}, io::{stdin, BufRead}, path::Path, process::exit};

use crate::{
    create_project, download_results, list_students::{self}, list_tests, run_tests::process_diff_tests, util::{self, is_valid_test_string, patch}, view_student_results::{self, TestResultError}
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
    if !list_students::list_students(project_path).iter().any(|s| s==student) {
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

pub fn view_student_result(project_path: &Path, student: &str, test: &str, summarize: bool) {
    if !list_students::list_students(project_path).iter().any(|s| s==student) {
        eprintln!("Student '{}' not recognized", student);
        return;
    }

    if !list_tests::list_tests(project_path).contains(test) {
        eprintln!("Test '{}' not recognized", test);
        return;
    }

    match view_student_results::parse_test_results(project_path, student, test) {
        Ok(result) => {
            if summarize {
                println!("{}", result.summarize());
            } else {
                println!("{}", result.print());
            }
        },
        Err(e) => {
            match e {
                TestResultError::IOError(er) => {
                    eprintln!("{}", er);
                }
                TestResultError::CompilationError => {
                    eprintln!("Compilation Error");
                }
                TestResultError::TestsNotRun => {
                    eprintln!("Tests have not been run for this student");
                }
            }
        }
    }
}

pub fn view_all_results(project_path: &Path, test: &str, summarize: bool) {
     if !list_tests::list_tests(project_path).contains(test) {
        eprintln!("Test '{}' not recognized", test);
        return;
     }
    for student in list_students::list_students(project_path) {
        view_student_result(project_path, student.as_str(), test, summarize);
    }
}

pub fn download_results_summary(project_path: &Path, test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists() {
        println!("{} Exists. Continue? (Y/N)", outfile);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    let out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_file_path).unwrap();
    download_results::download_results_summary(project_path, out_file, test).unwrap();

}
pub fn download_results_by_classname(project_path: &Path, test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists() {
        println!("{} Exists. Continue? (Y/N)", outfile);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    let out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_file_path).unwrap();
    download_results::download_results_by_classname(project_path, out_file, test).unwrap();
}

pub fn view_student_submission(project_path: &Path, student: &str, copy_ignore_set: &HashSet<&str>) {
    if !list_students::list_students(project_path).iter().any(|s|s==student) {
        eprintln!("Student '{}' does not exist\n", student);
        exit(1);
    }
    let student_diff_path = project_path.join("submission_diffs").join(student);
    if !student_diff_path.is_file() {
        eprintln!("Student diff '{}' does not exist or is not a file", student_diff_path.to_string_lossy());
        exit(1);
    }
    let dest = Path::new(student);
    if dest.exists() {
        println!("'{}' Exists. Continue? (Y/N)", student);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    if dest.is_file() {
        remove_file(dest).expect(&format!("Can remove file '{}'", student));
    } else if dest.is_dir() {
        remove_dir(dest).expect(&format!("Can remove directory '{}'", student));
    }

    create_dir(dest).expect(&format!("Can create directory '{}", student));
    patch(project_path.join("main").as_path(), student_diff_path.as_path(), dest, copy_ignore_set).unwrap();
}