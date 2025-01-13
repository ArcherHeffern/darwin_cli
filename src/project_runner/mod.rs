use std::{
    collections::{HashMap, HashSet},
    fs::{self, create_dir_all, remove_file, File},
    io::{Error, ErrorKind, Result},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use zip::ZipArchive;

use crate::{
    config::{darwin_root, diff_exclude_dir, skel_dir},
    types::{TestResult, TestResultError},
    util::{self, create_diff, extract_zipfile, patch, path_remove_trailing_slash, project_root_in_zip},
};

mod maven;

#[derive(Clone)]
pub struct Project {
    /// Maps directories and files in project skeleton, to location they should be stored to when diffing and running tests
    skel_mapping: HashMap<PathBuf, PathBuf>,

    /// Maps directories and files in student submission, to location they should be stored to when diffing and running tests
    submission_zipfile_mapping: HashMap<PathBuf, PathBuf>,

    /// Not used
    _ignore: HashSet<String>,

    /// Paths that are not stored while diffing. This is calculated as skel_mapping - submission_zipfile_mapping
    /// This is useful for entries that should not be modified by students, for example testfiles.
    diff_exclude: HashSet<PathBuf>,

    /// Given a normalized skeleton (access via `skeleton_dir()`), lists all test names to be used as input for 
    /// * run_test_fn
    /// * relocate_test_results_fn
    /// * parse_result_report_fn
    list_tests_fn: fn(&Project) -> HashSet<String>,

    /// Given a normalized project (`&Path`), compiles the project
    compile_fn: fn(&Project, &Path) -> Result<()>,

    /// Given a normalized project (`&Path`), and a test name (`&str`), run tests and produce a test report
    run_test_fn: fn(&Project, &Path, &str) -> Result<()>,

    /// May remove
    relocate_test_results_fn: fn(&Project, &Path, &str, &Path) -> Result<()>, // project, project_path, test, dest_file

    /// Given a test report (&Path), parse into `Vec<TestResult>`
    parse_result_report_fn:
        fn(&Project, &Path, &str, &str) -> std::result::Result<Vec<TestResult>, TestResultError>,
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

    Project::new(
        skel_mapping,
        submission_zipfile_mapping,
        ignore,
        maven::compile,
        maven::list_tests,
        maven::run_test,
        maven::relocate_test_results,
        maven::parse_result_report,
    )
}

impl Project {
    pub fn new(
        skel_mapping: HashMap<PathBuf, PathBuf>,
        submission_zipfile_mapping: HashMap<PathBuf, PathBuf>,
        _ignore: HashSet<String>,
        compile_fn: fn(&Project, &Path) -> Result<()>,
        list_tests_fn: fn(&Project) -> HashSet<String>,
        run_test_fn: fn(&Project, &Path, &str) -> Result<()>,
        relocate_test_results_fn: fn(&Project, &Path, &str, &Path) -> Result<()>, // project, project_path, test, dest_file
        parse_result_report_fn: fn(
            &Project,
            &Path,
            &str,
            &str,
        )
            -> std::result::Result<Vec<TestResult>, TestResultError>,
    ) -> Self {
        let skel_destinations: HashSet<PathBuf> = skel_mapping.values().cloned().collect();
        let submission_destinations: HashSet<PathBuf> =
            submission_zipfile_mapping.values().cloned().collect();
        let diff_exclude = skel_destinations
            .iter()
            .cloned()
            .filter(|skel_dest| !submission_destinations.contains(skel_dest))
            .collect();

        Project {
            skel_mapping,
            submission_zipfile_mapping,
            _ignore,
            diff_exclude,
            compile_fn,
            list_tests_fn,
            run_test_fn,
            relocate_test_results_fn,
            parse_result_report_fn,
        }
    }

    pub fn init_skeleton(&self, skeleton_path: &Path) -> Result<()> {
        for (from, to) in self.skel_mapping.iter() {
            if !skeleton_path.join(from).exists() {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Expected {:?} to exist in skeleton", from),
                ));
            }

            let from = skeleton_path.join(from);
            let to = match self.diff_exclude.contains(to) {
                true => {
                    diff_exclude_dir()
                }
                false => skel_dir(),
            }
            .join(to);
            if !from.to_string_lossy().ends_with('/') {
                if !from.is_file() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a file", &from),
                    ));
                } else {
                    if let Some(parent) = to.parent() {
                        create_dir_all(parent)?;
                    }
                    fs::copy(&from, &to).map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Failed to copy {:?} to {:?}: {}", from, to, e),
                        )
                    })?;
                }
            }
            if from.to_string_lossy().ends_with('/') {
                if !from.is_dir() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a directory", from),
                    ));
                } else {
                    util::copy_dir_all(&from, &to, None).map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Failed to copy {:?} to {:?}: {}", from, to, e),
                        )
                    })?;
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
            return Err(Error::new(
                ErrorKind::NotFound,
                "Expected zip to have contents",
            ));
        }

        let root = project_root_in_zip(zip, &self.submission_zipfile_mapping.keys().collect())?;

        let files_to_extract: HashMap<usize, PathBuf> = (0..zip.len()) // Index -> Dest
            .filter_map(|i| {
                let file = zip.by_index(i).ok()?;
                let filename = PathBuf::from(file.name());
                let file_name = filename.file_name()?.to_str()?;
                if copy_ignore_set.is_some_and(|s| s.contains(file_name)) {
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

    pub fn create_normalized_project_diff(
        &self,
        normalized_project: &Path,
        diff_dest: &Path,
    ) -> Result<()> {
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

        patch(&skel_dir(), diff_path, project_path, true)?;

        for excluded in self.diff_exclude.iter() {
            let original = diff_exclude_dir().join(excluded).canonicalize()?;
            let mut link = project_path.join(excluded);
            if let Some(parent) = link.parent() {
                fs::create_dir_all(parent)?;
            }
            link = path_remove_trailing_slash(&link);
            symlink(&original, &link).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to symlink {:?} to {:?}: {}", link, original, e),
                )
            })?;
        }

        Ok(())
    }

    pub fn compile(&self, project_path: &Path) -> Result<()> {
        (self.compile_fn)(self, project_path)
    }

    pub fn list_tests(&self) -> HashSet<String> {
        if let Err(e) = self.denormalize_skel() {
            eprintln!("{}", e);
        }
        let mut res = HashSet::new();
        match self.normalize_skel() {
            Ok(()) => {
                res = (self.list_tests_fn)(self);
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
        if let Err(e) = self.denormalize_skel() {
            eprintln!("{}", e);
        }
        res
    }

    fn normalize_skel(&self) -> Result<()> {
        for excluded in self.diff_exclude.iter() {
            let original = diff_exclude_dir().join(excluded).canonicalize()?;
            let mut link = skel_dir().join(excluded);
            link = path_remove_trailing_slash(&link);
            symlink(&original, &link).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("BBBFailed to symlink {:?} to {:?}: {}", link, original, e),
                )
            })?;
        }

        Ok(())
    }

    fn denormalize_skel(&self) -> Result<()> {
        for excluded in self.diff_exclude.iter() {
            let remove = skel_dir().join(excluded);
            let remove = path_remove_trailing_slash(&remove);
            if !remove.exists() {
                continue;
            }
            remove_file(&remove).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to remove what should be a symlink {:?}: {}", &remove, e),
                )
            })?;
        }

        Ok(())

    }

    pub fn run_test(&self, project_path: &Path, test: &str) -> Result<()> {
        (self.run_test_fn)(self, project_path, test)
    }

    pub fn relocate_test_results(
        &self,
        project_path: &Path,
        test: &str,
        dest_file: &Path,
    ) -> Result<()> {
        (self.relocate_test_results_fn)(self, project_path, test, dest_file)
    }

    pub fn parse_result_report(
        &self,
        report_path: &Path,
        student: &str,
        test: &str,
    ) -> std::result::Result<Vec<TestResult>, TestResultError> {
        (self.parse_result_report_fn)(self, report_path, student, test)
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
        project.init_skeleton(Path::new("./skel")).unwrap();
    }

    #[test]
    fn test_zip_submission_to_normalized_form() {
        let file = File::open("./testing/test.zip").unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let dest_dir = Path::new("./testing/dest_dir_test");
        if dest_dir.exists() {
            remove_dir_all(&dest_dir).unwrap();
        }
        let project = maven_project();
        project
            .zip_submission_to_normalized_form(&mut zip, dest_dir, None)
            .unwrap();
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
        project
            .zip_submission_to_normalized_form(&mut zip, &dest_dir, None)
            .unwrap();
        project
            .create_normalized_project_diff(&dest_dir, &diff_dest)
            .unwrap();
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
        project
            .zip_submission_to_normalized_form(&mut zip, &dest_dir, None)
            .unwrap();
        project
            .create_normalized_project_diff(&dest_dir, &diff_dest)
            .unwrap();
        project
            .recreate_normalized_project(&project_dest_path, &diff_dest)
            .unwrap();
    }
}
