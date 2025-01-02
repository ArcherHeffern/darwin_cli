use std::{io::{Error, Result}, path::Path};

use crate::{list_students, util::patch};

// Should we coerce into working, or return error? 

pub fn view_student_submission(darwin_path: &Path, student: &str, dest: &Path) -> Result<()> {
    // Enforces:
    // student exists
    // dest does not exist
    if !list_students::list_students(darwin_path)
        .iter()
        .any(|s| s == student)
    {
        return Err(Error::new(std::io::ErrorKind::NotFound, format!("Student '{}' not found", student)));
    }
    let student_diff_path = darwin_path.join("submission_diffs").join(student);
    if !student_diff_path.is_file() {
        return Err(Error::new(std::io::ErrorKind::NotFound, format!("{}'s diff '{}' does not exist or is not a file", student, student_diff_path.to_string_lossy())));
    }

    if dest.exists() {
        return Err(Error::new(std::io::ErrorKind::AlreadyExists, "Dest should not exist"));
    }

    patch(
        darwin_path.join("main").as_path(),
        student_diff_path.as_path(),
        dest,
    )
}