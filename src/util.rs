use std::collections::HashSet;
use std::fs::{self, create_dir, create_dir_all, rename, OpenOptions};
use std::io::{copy, prelude::*, BufReader, BufWriter, Error, ErrorKind, Result};
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{fs::File, io, path::Path};

use tempfile::NamedTempFile;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::config::{darwin_root, main_dir};
use crate::{list_students, list_tests};

pub fn prompt_yn(prompt: &str) -> Result<bool> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("{} ", prompt);
    stdout.flush()?;
    let mut line = String::new();
    let size = stdin.lock().read_line(&mut line)?;
    Ok(line.as_str()[..size - 1].to_lowercase() == "y")
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

pub fn flatten_move_recursive(
    src: &Path,
    dst: &Path,
    ignore: Option<&HashSet<&str>>,
) -> io::Result<()> {
    // Invariant: dst does not exist
    if dst.exists() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "flatten_move_all expects dst to not exist",
        ));
    }
    create_dir_all(dst)?;
    _flatten_move_recursive(src, dst, ignore)
}

fn _flatten_move_recursive(
    src: &Path,
    dst: &Path,
    ignore: Option<&HashSet<&str>>,
) -> io::Result<()> {
    if src.is_dir() {
        match fs::read_dir(src) {
            Ok(entries) => {
                for entry in entries.into_iter().flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let mut name = path.clone();
                        name.set_extension("html");
                        if let Some(name) = name.file_name() {
                            if ignore
                                .is_some_and(|i| name.to_str().map_or(false, |n| i.contains(n)))
                            {
                                continue;
                            }
                            rename(&path, dst.join(name))?;
                        }
                    } else if path.is_dir() {
                        _flatten_move_recursive(&path, dst, ignore)?;
                    }
                }
            }
            Err(e) => println!("Failed to read directory {}: {}", src.display(), e),
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

pub fn create_diff(original: &Path, deviant: &Path, dest_path: &Path) -> Result<()> {
    // Truncates dest_path if it exists

    if !original.exists() {
        return Err(Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Cannot create diff with non existing original path {:?} ",
                original
            ),
        ));
    }
    if !deviant.exists() {
        return Err(Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Cannot create diff with non existing deviant path {:?} ",
                original
            ),
        ));
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

pub fn is_test(test: &str) -> bool {
    // validate list of tests is comma separated and all exist
    list_tests::list_tests().contains(test)
}

pub fn is_student(student: &str) -> bool {
    list_students::list_students().iter().any(|s| s == student)
}

pub fn create_student_project(project_path: &Path, diff_path: &Path) -> Result<()> {
    // Invariants:
    // - darwin_path is an existing .darwin project root directory
    // - project_path does not exist
    assert!(darwin_root().is_dir());
    assert!(!project_path.exists());

    create_dir_all(project_path)?;
    create_dir(project_path.join("src"))?;
    symlink(
        darwin_root().join("test").canonicalize()?,
        project_path.join("src").join("test"),
    )?;
    create_dir_all(project_path.join("src").join("main"))?;
    recreate_student_main(
        diff_path,
        &project_path.join("src").join("main"),
        project_path,
    )?;

    Ok(())
}

pub fn recreate_student_main(
    diff_path: &Path,
    main_dest_dir: &Path,
    pom_dest_dir: &Path,
) -> Result<()> {
    // Dest dir must be empty
    // pom.xml will also end up in this directory
    if !main_dest_dir.is_dir() {
        return Err(Error::new(ErrorKind::Other, "Expected dir"));
    }

    patch(&main_dir(), diff_path, main_dest_dir)?;
    fs::rename(main_dest_dir.join("pom.xml"), pom_dest_dir.join("pom.xml"))?;
    Ok(())
}

pub fn patch(patch_path: &Path, diff_path: &Path, dest_path: &Path) -> Result<()> {
    // patch_path: Directory containing original files
    // diff_path: Diff to be patched into patch_path
    // Destination path
    copy_dir_all(patch_path, dest_path, &HashSet::new()).unwrap();

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
            return Err(Error::new(
                ErrorKind::BrokenPipe,
                format!("Cannot access stdin of patch process {}", output.id()),
            ));
        }
    }

    if output.wait().is_err() {
        return Err(Error::new(
            ErrorKind::Other,
            "Failed to wait for patch process",
        ));
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
                if line == &cur_line[0..cur_line.len() - 1] {
                    return Ok(true);
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

pub fn file_append_line(file: &Path, line: &str) -> Result<()> {
    let mut f = OpenOptions::new().append(true).open(file)?;
    writeln!(f, "{}", line)?;
    Ok(())
}

pub fn file_replace_line(filename: &Path, prev: &str, new: &str) -> Result<()> {
    let tempfile = NamedTempFile::new()?;
    let reader = File::open(filename)?;
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(tempfile);

    buffer_flatmap(&mut reader, &mut writer, |line| {
        if line.contains(prev) {
            return Some(new.to_string());
        }
        Some(line.to_string())
    }
    )?;
    writer.into_inner()?.persist(filename)?;

    Ok(())
}

fn buffer_flatmap<R: std::io::Read, W: std::io::Write>(reader: &mut BufReader<R>, writer: &mut BufWriter<W>, func: impl Fn(&str)->Option<String>) -> Result<()> {
    // Returns the number of lines changed
    // Does not exclude the newline
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => {
                break;
            }
            Ok(_) => {
                match func(&buf[..buf.len()]) {
                    Some(res) => {
                        writer.write(res.as_bytes())?;
                    }
                    None => {}
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use crate::util::buffer_flatmap;

    use super::{file_replace_line, BufReader, BufWriter, Write};


    #[test]
    fn buffer_filter_test() {
        let source = String::from("Hello\nworld\nHow\nAre");
        let dest = Vec::new();
        let mut reader = BufReader::new(source.as_bytes());
        let mut writer = BufWriter::new(dest);
        buffer_flatmap(&mut reader, &mut writer, |line|{
            if line.contains("Hello") {
                return None;
            }
            return Some(line.to_string());
        }).unwrap();
        writer.flush().unwrap();

        let actual = writer.into_inner().unwrap();
        let expected = String::from("world\nHow\nAre");
        dbg!(std::str::from_utf8(&actual).unwrap());
        assert_eq!(actual, expected.as_bytes());
    }

    #[test]
    fn file_replace_line_test() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "a\nb\na\na\naa\n\nbb").unwrap();
        f.as_file_mut().flush().unwrap();
        file_replace_line(f.path(), "a", "c\n").unwrap();

        let mut actual_contents = String::new();
        let mut file = File::open(f.path()).unwrap();
        file.read_to_string(&mut actual_contents).unwrap(); // Read the contents after modification

    let expected_contents = "c\nb\nc\nc\nc\n\nbb";

    assert_eq!(actual_contents, expected_contents); 
    }
}