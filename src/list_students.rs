use crate::config::diff_dir;

pub fn list_students() -> Vec<String> {
    let mut students = Vec::new();
    for entry in diff_dir().read_dir().unwrap() {
        students.push(entry.unwrap().file_name().to_str().unwrap().to_string());
    }
    students.sort();
    students
}
