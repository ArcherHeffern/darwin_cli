use std::{io::{Error, ErrorKind, Result}, path::Path, fs::create_dir};

use crate::commands::list_students;


pub fn create_report(darwin_path: &Path, report_path: &Path) -> Result<()> {
    if !darwin_path.is_dir() {
        return Err(Error::new(ErrorKind::NotFound, "Darwin project not initialized"));
    }
    if report_path.exists() {
        return Err(Error::new(ErrorKind::AlreadyExists, "report_path exists"));
    }
    _create_report(darwin_path, report_path)
}

fn _create_report(darwin_path: &Path, dest_path: &Path) -> Result<()> {
    create_dir(dest_path)?;
    create_report_student_list(darwin_path, &dest_path.join("index.html"));
    Ok(())
}

fn create_report_student_list(darwin_path: &Path, dest: &Path) {
    let students = list_students(darwin_path);
}