use std::{
    io::{Error, Result},
    path::Path,
};

use crate::{
    config::student_diff_file, list_students, project_runner::Project};

// Should we coerce into working, or return error?

pub fn view_student_submission(project: &Project, student: &str, dest: &Path) -> Result<()> {
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

    project.recreate_normalized_project(dest, &student_diff_path)?;
    project.recreate_original_project(dest, true)?;

    Ok(())
}
