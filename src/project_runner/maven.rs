use std::collections::HashSet;
use std::fs::rename;
use std::path::{Path, PathBuf};
use std::io::{Result, Error, ErrorKind};
use std::process::{Command, Stdio};

use crate::config::skel_dir;
use crate::util::list_files_recursively;

use super::Project;


pub fn compile(_: &Project, project_path: &Path) -> Result<()> {
    // mvn compile
    let mut compile_command = Command::new("mvn")
        .current_dir(project_path)
        .arg("test-compile")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = compile_command.wait()?;
    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "'mvn test-compile' failed"));
    }

    Ok(())
}

pub fn list_tests(_: &Project) -> HashSet<String> {
    let test_dir = skel_dir().join("src").join("main").join("java");
    let test_dir_str = test_dir.to_str().unwrap();
    let files = list_files_recursively(&test_dir);

    let mut out = HashSet::new();
    for file in files {
        if file.extension().map_or(false, |ext| ext != "java") {
            continue;
        }
        let file = file.strip_prefix(test_dir_str).unwrap();
        let file_name = file.to_string_lossy();
        let test_name = file_name.replace('/', ".");
        out.insert(test_name[..file_name.len() - 5].to_string());
    }

    out
}

/// Runs `mvn -Dtest={test_str} surefire:test`
/// Returns results file destination
pub fn run_test(_: &Project, project_path: &Path, test: &str) -> Result<()> {

    let mut run_tests_command = Command::new("mvn")
        .current_dir(project_path)
        .arg(format!("-Dtest={}", test))
        .arg("surefire:test")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    run_tests_command.wait()?;

    Ok(())
}

pub fn relocate_test_results(_: &Project, project_path: &Path, test: &str, dest_file: &Path) -> Result<()> {
    let results_filename_from = format!("TEST-{}.xml", test);
    let results_file_from = project_path
        .join("target")
        .join("surefire-reports")
        .join(results_filename_from);

    if !results_file_from.exists() {
        return Err(Error::new(ErrorKind::NotFound, "Results file was not found"));
    }
    rename(&results_file_from, dest_file).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to rename {:?} to {:?}: {}", results_file_from, dest_file, e)))?;
    Ok(())
}

// Parse test results

// Create universal test result format for report

// Display test results in report
