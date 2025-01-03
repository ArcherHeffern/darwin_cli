use std::path::PathBuf;


pub fn darwin_path() -> PathBuf {
    PathBuf::from(".darwin")
}

pub fn tests_ran_path() -> PathBuf {
    darwin_path().join("tests_ran")
}

pub fn diff_dir() -> PathBuf {
    darwin_path().join("submission_diffs")
}

pub fn student_diff_path(student: &str) -> PathBuf {
    diff_dir().join(student)
}

pub fn student_project_path(student: &str) -> PathBuf {
    darwin_path().join(student)
}