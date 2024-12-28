
pub mod utils {

    use std::collections::HashSet;
    use std::fs;
    use std::{fs::File, io, path::Path};

    use zip::result::ZipError;
    use zip::ZipArchive;

    pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, ignore: &HashSet<&str>) -> io::Result<()> {
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

    pub fn extract_directory_from_zip(archive: &mut ZipArchive<File>, output_dir: &str, dir_name: &str, ignore_substrings: &HashSet<&str>) -> zip::result::ZipResult<()> {
        // Places all contents of dir_name, not including the directory name into output_dir
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name();

            // Check if the file is in the specified directory
            let index_of_dir_name = index_of_substr(file_name, dir_name);
            if index_of_dir_name == None || ignore_substrings.iter().any(|ignore| file_name.contains(ignore)) {
                continue;
            }

            let out_path_s = &file_name[index_of_dir_name.unwrap() + dir_name.len()+1..]; 
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
                    if let Err(e) = std::fs::create_dir_all(parent) { // Create parent directories if needed
                        eprintln!("Error creating parent directories of {:?}: {}", out_path, e);
                    }
                }
                match File::create(&out_path) {
                    Err(e) => eprintln!("Error creating file {:?}: {}", out_path, e),
                    Ok(mut outfile) => match io::copy(&mut file, &mut outfile) {
                        Ok(_) => (),
                        Err(e) => eprintln!("Error copying {} to {:?}: {}", file.name(), outfile, e)
                    }
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
            if &s[i..i+substr.len()] == substr {
                return Some(i);
            }
        }
        return None;
    }
}

