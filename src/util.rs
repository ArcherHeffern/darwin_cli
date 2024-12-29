use std::collections::HashSet;
use std::fs;
use std::io::{copy, BufReader, BufWriter};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{fs::File, io, path::Path};

use zip::result::ZipError;
use zip::ZipArchive;

pub fn copy_dir_all(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    ignore: &HashSet<&str>,
) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()), ignore)?;
        } else if !ignore.contains(entry.file_name().to_str().unwrap()) {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn list_files_recursively(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            files.push(path);
                        } else if path.is_dir() {
                            let mut sub_files = list_files_recursively(&path);
                            files.append(&mut sub_files);
                        }
                    }
                }
            }
            Err(e) => println!("Failed to read directory {}: {}", dir.display(), e),
        }
    }

    files
}

pub fn extract_directory_from_zip(
    archive: &mut ZipArchive<File>,
    output_dir: &str,
    dir_name: &str,
    ignore_substrings: &HashSet<&str>,
) -> zip::result::ZipResult<()> {
    // Places all contents of dir_name, not including the directory name into output_dir
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();

        // Check if the file is in the specified directory
        let index_of_dir_name = index_of_substr(file_name, dir_name);
        if index_of_dir_name == None
            || ignore_substrings
                .iter()
                .any(|ignore| file_name.contains(ignore))
        {
            continue;
        }

        let out_path_s = &file_name[index_of_dir_name.unwrap() + dir_name.len() + 1..];
        let out_path = Path::new(output_dir).join(out_path_s);

        if file.is_dir() {
            // Create the directory
            if let Err(e) = std::fs::create_dir_all(&out_path) {
                eprintln!("Error creating directory and all parent directories of {:?} while extracting directory from zipfile: {}", out_path, e);
                eprintln!("{}", out_path_s);
                return Err(ZipError::from(e));
            }
        } else {
            // Write the file
            if let Some(parent) = out_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    // Create parent directories if needed
                    eprintln!("Error creating parent directories of {:?}: {}", out_path, e);
                }
            }
            match File::create(&out_path) {
                Err(e) => eprintln!("Error creating file {:?}: {}", out_path, e),
                Ok(mut outfile) => match io::copy(&mut file, &mut outfile) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Error copying {} to {:?}: {}", file.name(), outfile, e),
                },
            }
        }
    }
    Ok(())
}

fn index_of_substr(s: &str, substr: &str) -> Option<usize> {
    if s.len() < substr.len() {
        return None;
    }
    for i in 0..s.len() - substr.len() + 1 {
        if &s[i..i + substr.len()] == substr {
            return Some(i);
        }
    }
    return None;
}

pub fn find_student_diff_file(project_path: &Path, student_name: &str) -> Option<PathBuf> {
    let diff_path = project_path.join("submission_diffs").join(student_name);
    if diff_path.is_file() {
        return Some(diff_path);
    }

    None
}

pub fn is_valid_test_string(project_path: &Path, tests: &str) -> bool {
    // validate list of tests is comma separated and all exist
    let actual_tests = list_tests(project_path);

    for test in tests.split(',') {
        if !actual_tests.contains(&test.to_string()) {
            return false;
        }
    }
    return true;
}

pub fn list_tests(project_path: &Path) -> Vec<String> {
    let test_dir = project_path.join("project").join("src").join("test").join("java");
    let test_dir_str = test_dir.to_str().unwrap();
    let files = list_files_recursively(&test_dir);

    let mut out = Vec::new();
    for file in files {
        if !file.extension().map_or(true, |ext| ext != ".java") {
            continue;
        }
        let file = file.strip_prefix(test_dir_str).unwrap();
        let file_name = file.to_string_lossy();
        let test_name = file_name.replace('/', ".");
        out.push(test_name[..file_name.len() - 5].to_string());
    }

    out
}

pub fn set_active_project(
    project_path: &Path,
    diff_path: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> Result<(), io::Error> {
    // diff_path: Contains the full project relative path
    // rm -rf .darwin/project/src/main
    // rm -rf .darwin/project/target
    // cp -r .darwin/main/ .darwin/project/src/main
    // patch -d .darwin/project/src/main -p2 < .darwin/submission_diffs/<student_diff
    // mv .darwin/project/src/main/pom.xml .darwin/project/pom
    let project_main_path = project_path.join("project").join("src").join("main");
    if project_main_path.exists() {
        if let Err(e) = fs::remove_dir_all(&project_main_path) {
            eprintln!(
                "Error removing {} when setting active project: {}",
                project_main_path.to_str().unwrap(),
                e
            );
            return Err(e);
        }
    }

    let maybe_target_dir = project_path.join("project").join("target");
    if maybe_target_dir.exists() {
        fs::remove_dir_all(maybe_target_dir)?;
    }

    copy_dir_all(
        project_path.join("main"),
        &project_main_path,
        copy_ignore_set,
    )
    .unwrap();

    let mut output = Command::new("patch")
        .arg("-d")
        .arg(&project_main_path)
        .arg("-p2")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    match output.stdin.take() {
        Some(stdin) => {
            let mut stdin_writer = BufWriter::new(stdin);
            let patch_file = File::open(diff_path)?;
            let mut patch_reader = BufReader::new(patch_file);
            copy(&mut patch_reader, &mut stdin_writer)?;
        }
        None => {
            eprintln!("Cannot access stdin of patch process {}", output.id());
        }
    }

    let status = output.wait()?;
    if !status.success() {
        eprintln!("Patch command failed with status: {}", status);
    }

    fs::rename(
        project_path
            .join("project")
            .join("src")
            .join("main")
            .join("pom.xml"),
        project_path.join("project").join("pom.xml"),
    )?;

    Ok(())
}
