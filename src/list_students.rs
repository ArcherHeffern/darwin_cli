use std::{collections::HashSet, path::Path};

pub fn list_students(project_path: &Path) -> HashSet<String> {
    let mut students = HashSet::new();
    for entry in project_path.join("submission_diffs").read_dir().unwrap() {
        students.insert(entry.unwrap().file_name().to_str().unwrap().to_string());
    }
    return students;
}