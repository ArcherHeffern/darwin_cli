use std::collections::HashSet;
use std::fs::{self, create_dir, create_dir_all};
use std::io::{copy, prelude::*, BufReader, BufWriter, Error, Result};
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{fs::File, io, path::Path};

use zip::result::ZipError;
use zip::ZipArchive;

use crate::{list_students, list_tests};

pub fn prompt_yn(prompt: &str) -> Result<bool> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("{}", prompt);
    stdout.flush()?;
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    println!("");
    return Ok(line.as_str() == "y\n");
}

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
                for entry in entries.into_iter().flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        files.push(path);
                    } else if path.is_dir() {
                        let mut sub_files = list_files_recursively(&path);
                        files.append(&mut sub_files);
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
        let index_of_dir_name = file_name.find(dir_name);
        if index_of_dir_name.is_none()
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

pub fn to_diff_path(darwin_path: &Path, student_name: &str) -> PathBuf {
    darwin_path.join("submission_diffs").join(student_name)
}

pub fn create_diff(original: &Path, deviant: &Path, dest_path: &Path) -> Result<()> {
    // Truncates dest_path if it exists

    if !original.exists() {
        return Err(Error::new(io::ErrorKind::NotFound, format!("Cannot create diff with non existing original path {:?} ", original)));
    }
    if !deviant.exists() {
        return Err(Error::new(io::ErrorKind::NotFound, format!("Cannot create diff with non existing deviant path {:?} ", original)));
    }
    _create_diff(original, deviant, dest_path)
}

fn _create_diff(original: &Path, deviant: &Path, dest_path: &Path) -> Result<()> {
    let output = Command::new("diff")
        .arg("-ruN")
        .arg(original)
        .arg(deviant)
        .stdout(Stdio::piped())
        .spawn()?;

    let diff_file = File::create(dest_path)?;
    let mut stdout_writer = BufWriter::new(diff_file);
    let mut stdout = output.stdout.unwrap();
    copy(&mut stdout, &mut stdout_writer)?;
    stdout_writer.flush()?;

    Ok(())
}

pub fn is_test(darwin_path: &Path, test: &str) -> bool {
    // validate list of tests is comma separated and all exist
    list_tests::list_tests(darwin_path).contains(test)
}

pub fn is_student(darwin_path: &Path, student: &str) -> bool {
    list_students::list_students(darwin_path).iter().any(|s|s==student)
}

pub fn initialize_project(darwin_path: &Path, project_path: &Path) -> Result<()> {
    // Invariants: 
    // - darwin_path is an existing .darwin project root directory
    // - project_path does not exist
    assert!( darwin_path.is_dir());
    assert!( !project_path.exists());

    create_dir_all(project_path)?;
    create_dir(project_path.join("src"))?;
    copy_dir_all(darwin_path.join("main"), project_path.join("src").join("main"), &HashSet::new())?;
    symlink(darwin_path.join("test").canonicalize()?, project_path.join("src").join("test"))?;

    Ok(())

}

pub fn set_active_project(
    darwin_path: &Path,
    project_path: &Path,
    diff_path: &Path,
) -> Result<()> {
    // Invariants: 
    // - darwin_path is an existing .darwin project root directory
    // - project_path is an existing, newly initialized project directory
    assert!(darwin_path.is_dir());
    assert!(project_path.is_dir());

    let project_main_path = project_path.join("src").join("main");
    patch(darwin_path.join("main").as_path(), diff_path, project_main_path.as_path())?;

    fs::rename(
        project_path
            .join("src")
            .join("main")
            .join("pom.xml"),
        project_path.join("pom.xml"),
    )?;

    Ok(())
}

pub fn patch(patch_path: &Path, diff_path: &Path, dest_path: &Path) -> Result<()> {
    // patch_path: Directory containing original files
    // diff_path: Diff to be patched into patch_path
    // Destination path
    copy_dir_all(
        patch_path,
        dest_path,
        &HashSet::new()
    )
    .unwrap();

    let mut output = Command::new("patch")
        .arg("-d")
        .arg(dest_path)
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

    Ok(())

}

pub fn file_contains_line(file: &Path, line: &str) -> Result<bool> {
    let file = File::open(file)?;
    let mut file = BufReader::new(file);
    let mut cur_line = String::new();
    loop {
        cur_line.clear();
        match file.read_line(&mut cur_line) {
            Ok(0) => {
                return Ok(false);
            }
            Ok(_) => {
                if line == &cur_line[0..cur_line.len()-1] {
                    return Ok(true);
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}