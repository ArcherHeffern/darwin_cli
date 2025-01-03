use std::collections::HashSet;

use crate::{config::test_dir, util::list_files_recursively};

pub fn list_tests() -> HashSet<String> {
    let test_dir = test_dir().join("java");
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
