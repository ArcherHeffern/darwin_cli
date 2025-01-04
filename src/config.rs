use std::path::PathBuf;

pub fn darwin_root() -> PathBuf {
    PathBuf::from(".darwin")
}

pub fn tests_ran_file() -> PathBuf {
    darwin_root().join("tests_ran")
}

pub fn diff_dir() -> PathBuf {
    darwin_root().join("submission_diffs")
}

pub fn student_diff_file(student: &str) -> PathBuf {
    diff_dir().join(student)
}

pub fn projects_dir() -> PathBuf {
    darwin_root().join("projects")
}

pub fn student_project_file(student: &str) -> PathBuf {
    projects_dir().join(student)
}

pub fn results_dir() -> PathBuf {
    darwin_root().join("results")
}

pub fn compile_errors_file() -> PathBuf {
    darwin_root().join("compile_errors")
}

pub fn test_dir() -> PathBuf {
    darwin_root().join("test")
}

pub fn student_result_file(student: &str, test: &str) -> PathBuf {
    results_dir().join(format!("{}_{}", student, test))
}

pub fn main_dir() -> PathBuf {
    darwin_root().join("main")
}
