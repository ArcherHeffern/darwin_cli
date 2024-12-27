
pub mod utils {

    use std::fs;
    use std::{fs::File, io, path::Path};

    use zip::ZipArchive;

    pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    pub fn extract_directory_from_zip(archive: &mut ZipArchive<File>, output_dir: &str, dir_name: &str) -> zip::result::ZipResult<()> {
        // Places all contents of dir_name, not including the directory name into output_dir
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name();

            // Check if the file is in the specified directory
            if !file_name.starts_with(dir_name) {
                continue;
            }

            let out_path_s = &file_name[dir_name.len()+1..];
            let out_path = Path::new(output_dir).join(out_path_s);

            if file.is_dir() {
                // Create the directory
                std::fs::create_dir_all(&out_path)?;
            } else {
                // Write the file
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?; // Create parent directories if needed
                }
                let mut outfile = File::create(&out_path)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(())
    }
}

