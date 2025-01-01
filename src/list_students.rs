use std::path::Path;

pub fn list_students(project_path: &Path) -> Vec<String> {
    let mut students = Vec::new();
    for entry in project_path.join("submission_diffs").read_dir().unwrap() {
        students.push(entry.unwrap().file_name().to_str().unwrap().to_string());
    }
    students.sort();
    return students;
}