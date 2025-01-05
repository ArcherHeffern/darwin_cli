use std::{
    collections::HashSet,
    fs::rename,
    io::{Error, Result},
    path::Path,
};

use crate::{
    config::{main_dir, student_diff_file, test_dir},
    list_students,
    util::{copy_dir_all, patch},
};

// Should we coerce into working, or return error?

pub fn view_student_submission(student: &str, dest: &Path) -> Result<()> {
    // Enforces:
    // student exists
    // dest does not exist
    if !list_students::list_students().iter().any(|s| s == student) {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            format!("Student '{}' not found", student),
        ));
    }
    let student_diff_path = student_diff_file(student);
    if !student_diff_path.is_file() {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "{}'s diff '{}' does not exist or is not a file",
                student,
                student_diff_path.to_string_lossy()
            ),
        ));
    }

    if dest.exists() {
        return Err(Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Dest should not exist",
        ));
    }

    _view_student_submission(dest, &student_diff_path)
}

fn _view_student_submission(dest: &Path, student_diff_path: &Path) -> Result<()> {
    patch(&main_dir(), student_diff_path, &dest.join("src"))?;
    rename(dest.join("src").join("pom.xml"), dest.join("pom.xml"))?;
    copy_dir_all(test_dir(), dest.join("src").join("test"), &HashSet::new())?;
    Ok(())
}
