use std::{
    collections::{HashMap, HashSet}, fs::{self, create_dir_all, File}, io::{Error, ErrorKind, Result}, os::unix::fs::symlink, path::{Path, PathBuf}
};

use zip::ZipArchive;

use crate::{
    config::{darwin_root, skel_dir, tmp_dir}, move_to_tmp_location_and_back::MoveToTempLocationAndBack, util::{self, create_diff, extract_zipfile, patch, project_root_in_zip}
};

mod maven;

// Report Include Map
// Skel Mapping
// Zipfile Mapping

// If in zipfile mapping or both, diff
// If only in Skel, symlink

// Remove copy_ignore_set in place of ignore

#[derive(Clone)]
pub struct Project {
    skel_mapping: HashMap<PathBuf, PathBuf>,
    submission_zipfile_mapping: HashMap<PathBuf, PathBuf>,
    ignore: HashSet<String>,
    diff_exclude: HashSet<PathBuf>,
    compile_fn: fn(&Project, &Path) -> Result<()>,
    list_tests_fn: fn(&Project) -> HashSet<String>,
    run_test_fn: fn(&Project, &Path, &str) -> Result<()>,
    relocate_test_results_fn: fn(&Project, &Path, &str, &Path) -> Result<()>, // project, project_path, test, dest_file
    // Result target: Should be a !format of the test name and student name
}

pub fn maven_project() -> Project {
    let mut skel_mapping = HashMap::new();
    skel_mapping.insert(PathBuf::from("src/main/"), PathBuf::from("src/main/"));
    skel_mapping.insert(PathBuf::from("src/test/"), PathBuf::from("src/test/"));
    skel_mapping.insert(PathBuf::from("pom.xml"), PathBuf::from("pom.xml"));

    let mut submission_zipfile_mapping = HashMap::new();
    submission_zipfile_mapping.insert(PathBuf::from("pom.xml"), PathBuf::from("pom.xml"));
    submission_zipfile_mapping.insert(PathBuf::from("src/main/"), PathBuf::from("src/main/"));

    let mut ignore = HashSet::new();
    ignore.insert(String::from(".DS_Store"));
    ignore.insert(String::from("doc"));
    ignore.insert(String::from(".settings"));
    ignore.insert(String::from(".project"));
    ignore.insert(String::from(".classpath"));
    ignore.insert(String::from(".git"));
    ignore.insert(String::from(".gitignore"));

    let skel_destinations: HashSet<PathBuf> = skel_mapping.values().cloned().collect();
    let submission_destinations: HashSet<PathBuf> = submission_zipfile_mapping.values().cloned().collect();
    let diff_exclude = skel_destinations.iter().cloned().filter(|skel_dest|!submission_destinations.contains(skel_dest)).collect();

    Project {
        skel_mapping,
        submission_zipfile_mapping,
        ignore,
        diff_exclude,
        compile_fn: maven::compile,
        list_tests_fn: maven::list_tests,
        run_test_fn: maven::run_test,
        relocate_test_results_fn: maven::relocate_test_results,
    }
}


impl Project {
    pub fn init_skeleton(
        &self,
        skeleton_path: &Path,
        copy_ignore_set: Option<&HashSet<&str>>,
    ) -> Result<()> {
        for (from, to) in self.skel_mapping.iter() {
            if !skeleton_path.join(from).exists() {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Expected {:?} to exist in skeleton", from),
                ));
            }
            if !from.to_string_lossy().ends_with('/') {
                if !skeleton_path.join(from).is_file() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a file", from),
                    ));
                } else {
                    let from = skeleton_path.join(from);
                    let to = skel_dir().join(to);
                    if let Some(parent) = to.parent() {
                        create_dir_all(parent)?;
                    }
                    fs::copy(&from, &to).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to copy {:?} to {:?}: {}", from, to, e)))?;
                }
            }
            if from.to_string_lossy().ends_with('/') {
                if !skeleton_path.join(from).is_dir() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a directory", from),
                    ));
                } else {
                    util::copy_dir_all(
                        &skeleton_path.join(from),
                        &skel_dir().join(to),
                        copy_ignore_set,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn zip_submission_to_normalized_form(
        &self,
        zip: &mut ZipArchive<File>,
        dest_dir: &Path,
        copy_ignore_set: Option<&HashSet<&str>>,
    ) -> Result<()> {
        if zip.is_empty() {
            return Err(Error::new(ErrorKind::NotFound, "Expected zip to have contents"));
        }

        let root = project_root_in_zip(zip, &self.submission_zipfile_mapping.keys().collect())?;

        let files_to_extract: HashMap<usize, PathBuf> = (0..zip.len()) // Index -> Dest
            .filter_map(|i| {
                let file = zip.by_index(i).ok()?;
                let filename = PathBuf::from(file.name());
                let file_name = filename.file_name()?.to_str()?;
                if copy_ignore_set.is_some_and(|s|s.contains(file_name)) {
                    return None;
                }
                let filename = PathBuf::from(filename.strip_prefix(&root).ok()?);

                for (k, v) in self.submission_zipfile_mapping.iter() {
                    if filename.starts_with(k) {
                        if filename.ends_with(Path::new("/")) {
                            let dest = v.join(filename.strip_prefix(k).ok()?);
                            return Some((i, dest));
                        } else {
                            return Some((i, filename));
                        }
                    }
                }
                None
            })
            .collect();

        for (i, dest) in files_to_extract {
            let zipfile = zip.by_index(i).unwrap();
            extract_zipfile(zipfile, &dest_dir.join(&dest))?;
        }

        Ok(())
    }

    pub fn create_normalized_project_diff(&self, normalized_project: &Path, diff_dest: &Path) -> Result<()> {
        let skd = skel_dir();
        let td = tmp_dir();
        let tmp_mover = MoveToTempLocationAndBack::create(&skd, &td, &self.diff_exclude);
        tmp_mover.move_to_temp_location()?;

        create_diff(&skel_dir(), normalized_project, diff_dest)?;

        Ok(())

    }

    /// If in diff_exclude, temporarily move out skel, patch, then move back, and symlink all entrys of diff_exclude in
    /// Invariants:
    /// - darwin_path is an existing .darwin project root directory
    /// - project_path does not exist
    pub fn recreate_normalized_project(&self, project_path: &Path, diff_path: &Path) -> Result<()> {
        assert!(darwin_root().is_dir());
        assert!(!project_path.exists());

        let skd = skel_dir();
        let td = tmp_dir();
        let tmp_mover = MoveToTempLocationAndBack::create(&skd, &td, &self.diff_exclude);
        tmp_mover.move_to_temp_location()?;

        patch(&skel_dir(), diff_path, project_path, true)?;

        drop(tmp_mover); // Move all entrys back

        for to_exclude in self.diff_exclude.iter() {
            let original = skel_dir().join(to_exclude).canonicalize()?;
            let link = project_path.join(to_exclude);
            if let Some(parent) = link.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut link = link.to_str().expect("Path to be valid unicode");
            if link.ends_with('/') {
                link = &link[0..link.len()-1];
            }
            symlink(
                &original,
                &link,
            ).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to symlink {:?} to {:?}: {}", link, original, e)))?;
        }

        Ok(())
    }

    pub fn compile(&self, project_path: &Path) -> Result<()> {
        (self.compile_fn)(self, project_path)
    }

    pub fn list_tests(&self) -> HashSet<String> {
        (self.list_tests_fn)(self)
    }

    pub fn run_test(&self, project_path: &Path, test: &str) -> Result<()> {
        (self.run_test_fn)(self, project_path, test)
    }

    pub fn relocate_test_results(&self, project_path: &Path, test: &str, dest_file: &Path) -> Result<()> {
        (self.relocate_test_results_fn)(self, project_path, test, dest_file)
    }
}


pub fn recreate_original_project(diff_path: &Path, dest: &Path) -> Result<()> {
    Ok(())
}

pub mod test {
    use std::fs::{remove_file, File};
    use std::io::BufReader;
    use std::path::PathBuf;
    use std::{fs::remove_dir_all, path::Path};

    use zip::ZipArchive;

    use crate::config::darwin_root;
    use crate::project_runner::Project;

    use super::maven_project;


    #[test]
    fn test_init_skeleton() {
        if darwin_root().exists() {
            remove_dir_all(darwin_root()).unwrap();
        }
        let project = maven_project();
        project.init_skeleton(Path::new("./skel"), None).unwrap();
    }

    #[test]
    fn test_zip_submission_to_normalized_form() {
        let file = File::open("./testing/test.zip").unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let dest_dir = Path::new("dest_dir_test");
        remove_dir_all(&dest_dir).unwrap();
        let project = maven_project();
        project.zip_submission_to_normalized_form(&mut zip, dest_dir, None).unwrap();
    }

    #[test]
    fn test_create_normalized_project_diff() {
        let file = File::open("./testing/test.zip").unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let dest_dir = Path::new("testing").join("dest_dir_test");
        let diff_dest = Path::new("testing").join("diff_dest");
        remove_dir_all(&dest_dir).unwrap();
        remove_file(&diff_dest).unwrap();
        let project = maven_project();
        project.zip_submission_to_normalized_form(&mut zip, &dest_dir, None).unwrap();
        project.create_normalized_project_diff(&dest_dir, &diff_dest).unwrap();
    }

    #[test]
    fn test_recreate_normalized_project() {
        let file = File::open("./testing/test.zip").unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let dest_dir = Path::new("testing").join("dest_dir_test");
        let diff_dest = Path::new("testing").join("diff_dest");
        let project_dest_path = Path::new("testing").join("project");
        if dest_dir.exists() {
            remove_dir_all(&dest_dir).unwrap();
        }
        if diff_dest.exists() {
            remove_file(&diff_dest).unwrap();
        }
        if project_dest_path.exists() {
            remove_dir_all(&project_dest_path).unwrap();
        }
        let project = maven_project();
        project.zip_submission_to_normalized_form(&mut zip, &dest_dir, None).unwrap();
        project.create_normalized_project_diff(&dest_dir, &diff_dest).unwrap();
        project.recreate_normalized_project(&project_dest_path, &diff_dest).unwrap();
    }
}
