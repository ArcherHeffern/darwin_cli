use std::collections::{HashMap, HashSet};
use std::fs::{self, create_dir_all, rename, OpenOptions};
use std::io::{copy, prelude::*, BufReader, BufWriter, Error, ErrorKind, Result};
use std::num::ParseIntError;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{fs::File, io, path::Path};

use tempfile::NamedTempFile;
use zip::read::ZipFile;
use zip::ZipArchive;

use crate::list_students;
use crate::project_runner::Project;

pub fn prompt_digit<T: FromStr<Err = ParseIntError> + ToString>(prompt: &str) -> Result<T> {
    let line = input(prompt)?;
    let line = &line[..line.len() - 1];
    T::from_str(line).map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))
}

pub fn prompt_yn(prompt: &str) -> Result<bool> {
    let line = input(prompt)?;
    Ok(line.as_str()[..line.len() - 1].to_lowercase() == "y")
}

fn input(prompt: &str) -> Result<String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("{} ", prompt);
    stdout.flush()?;
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line)
}

/// Recursively copies all files from src into dest, ignoring files contained in `ignore`
/// 
/// create_dir_all's dst
/// 
/// # Errors
/// 
/// Dest MUST not exist
pub fn copy_dir_all(
    src: &Path,
    dst: &Path,
    ignore: Option<&HashSet<&str>>,
) -> io::Result<()> {
    _copy_dir_all(src, dst, ignore).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to copy {:?} to {:?}: {}", src, dst, e)))
}
pub fn _copy_dir_all(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    ignore: Option<&HashSet<&str>>,
) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            _copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()), ignore)?;
        } else if ignore.map_or(true, |i|!i.contains(entry.file_name().to_str().unwrap())) {
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

/// Given main path /etc/conf/hold/home, and subpath, /conf/hold, returns parent of subpath, /etc/
pub fn subpath_parent(path: &Path, sub_path: &Path) -> Option<PathBuf> {
    let path_parts: Vec<String> = path.iter().map(|os_str|os_str.to_string_lossy().to_string()).collect();
    let sub_path_parts: Vec<String> = sub_path.iter().map(|os_str|os_str.to_string_lossy().to_string()).collect();
    let index = find_subarray_index(&path_parts, &sub_path_parts)?;
    let out = (0..index).fold(PathBuf::new(), |buf, index|buf.join(path_parts.get(index).unwrap()));
    Some(out)
}

pub fn find_subarray_index<T:PartialEq>(haystack: &[T], needle: &[T]) -> std::option::Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }
    for i in 0..haystack.len()-needle.len()+1 {
        if haystack[i..i+needle.len()] == needle[..] {
            return Some(i);
        }
    }
    None
}

/// Finds the root that matches the structure contained in project_structure
/// Given a ziparchive, find the directory that directly contains all of project_structure
pub fn project_root_in_zip(zip: &mut ZipArchive<File>, project_structure: &HashSet<&PathBuf>) -> Result<PathBuf> {
    let mut project_structure_mappings: HashMap<PathBuf, HashSet<&PathBuf>> = HashMap::new();
    for i in 0..zip.len() {
        if let Some(file_name) = zip.by_index(i)?.enclosed_name() {
            for part in project_structure.iter().cloned() {
                if let Some(parent) = subpath_parent(&file_name, part) {
                    project_structure_mappings.entry(parent).or_insert_with(HashSet::new).insert(part);
                }
            }
        }
    }
    let roots: Vec<&PathBuf> = project_structure_mappings.iter().filter(|(_, v)|v.len()==project_structure.len()).map(|(k,_)|k).collect();

    match roots.len() {
        0 => {
            Err(Error::new(ErrorKind::Other, "No project root found in zipfile matching project_structure"))
        }
        1 => {
            Ok(roots[0].clone())
        }
        _ => {
            Err(Error::new(ErrorKind::Other, format!("Too many project roots found in zipfile matching project_structure: {:?}", roots)))
        }
    }
}

/// Truncates dest_path if it exists. Creates dest_path if not.
/// 
/// # Errors
/// * original does not exist
/// * deviant does not exist
pub fn create_diff(original: &Path, deviant: &Path, dest_path: &Path) -> Result<()> {

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

pub fn is_test(project: &Project, test: &str) -> bool {
    // validate list of tests is comma separated and all exist
    project.list_tests().contains(test)
}

pub fn is_student(student: &str) -> bool {
    list_students::list_students().iter().any(|s| s == student)
}


pub fn patch(patch_path: &Path, diff_path: &Path, dest_path: &Path, silent: bool) -> Result<()> {
    // patch_path: Directory containing original files
    // diff_path: Diff to be patched into patch_path
    // Destination path
    copy_dir_all(patch_path, dest_path, Some(&HashSet::new())).unwrap();

    let stdout = match silent {
        true => Stdio::null(),
        false => Stdio::inherit()
    };

    let mut output = Command::new("patch")
        .arg("-d")
        .arg(dest_path)
        .arg("-p2")
        .stdin(Stdio::piped())
        .stdout(stdout)
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


/// Extracts file from ZipArchive
/// 
/// # Errors
/// 
/// * file pointed to by file_name does not exist
/// * dest is not writable 
/// * dest does not exist
pub fn extract_file(
    zip: &mut ZipArchive<File>,
    file_name: &str,
    dest: &mut File,
) -> Result<()> {
    let mut file_in_zip = zip.by_name(file_name)?;
    let mut writer = BufWriter::new(dest);
    io::copy(&mut file_in_zip, &mut writer)?;
    writer.flush()?; // Ensure all data is written to the underlying file
    Ok(())
}

/// Extracts file or directory from zipfile
pub fn extract_zipfile(zipfile: ZipFile, dest: &Path) -> Result<()> {
    if zipfile.is_dir() {
        create_dir_all(dest)?;
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        create_dir_all(parent)?;
    }
    let dest_file = File::create(&dest)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Creating Dest file: {}", e)))?;
    let mut reader = BufReader::new(zipfile);
    let mut writer = BufWriter::new(dest_file);

    // Copy the contents of the zip entry to the new file
    copy(&mut reader, &mut writer)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Copying zip entry to dest: {}", e)))?;

    // Flush the writer to ensure all data is written
    writer
        .flush()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Flushing buffer: {}", e)))?;
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
    })?;
    writer.into_inner()?.persist(filename)?;

    Ok(())
}

pub fn buffer_flatmap<R: std::io::Read, W: std::io::Write>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    func: impl Fn(&str) -> Option<String>,
) -> Result<()> {
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
                if let Some(res) = func(&buf) {
                    writer.write_all(res.as_bytes())?;
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
    use std::{collections::HashMap, fs::File, io::Read, path::{Path, PathBuf}};

    use zip::ZipArchive;

    use crate::{project_runner::maven_project, util::buffer_flatmap};

    use super::{file_replace_line, project_root_in_zip, subpath_parent, BufReader, BufWriter, Write};

    #[test]
    fn buffer_filter_test() {
        let source = String::from("Hello\nworld\nHow\nAre");
        let dest = Vec::new();
        let mut reader = BufReader::new(source.as_bytes());
        let mut writer = BufWriter::new(dest);
        buffer_flatmap(&mut reader, &mut writer, |line| {
            if line.contains("Hello") {
                return None;
            }
            Some(line.to_string())
        })
        .unwrap();
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

    #[test]
    fn test_subpath_parent() {
        let path = Path::new("etc").join("home").join("turtle").join("frog");
        assert_eq!(subpath_parent(&path, Path::new("etc")), Some(PathBuf::from("")));
        assert_eq!(subpath_parent(&path, Path::new("home")), Some(PathBuf::from("etc")));
        assert_eq!(subpath_parent(&path, Path::new("turtle")), Some(PathBuf::from("etc/home")));
        assert_eq!(subpath_parent(&path, Path::new("frog")), Some(PathBuf::from("etc/home/turtle")));
        assert_eq!(subpath_parent(&path, Path::new("not_exists")), None);
    }

    #[test]
    fn test_project_root_in_zip() {
        let zip_path = Path::new("./testing/test.zip");
        let zip_file = File::open(zip_path).unwrap();
        let mut zip = ZipArchive::new(zip_file).unwrap();
        let mut submission_zipfile_mapping = HashMap::new();
        submission_zipfile_mapping.insert(PathBuf::from("pom.xml"), PathBuf::from("pom.xml"));
        submission_zipfile_mapping.insert(PathBuf::from("src/main/"), PathBuf::from("src/main/"));

        let root = project_root_in_zip(&mut zip, &submission_zipfile_mapping.keys().collect()).unwrap();
        assert!(root == PathBuf::from("TestPA1")); 
    }
}
