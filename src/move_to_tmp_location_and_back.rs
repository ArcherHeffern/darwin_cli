use std::{collections::HashSet, fs::{create_dir_all, remove_dir_all, rename}, io::{Error, ErrorKind, Result}, path::{Path, PathBuf}};


pub struct MoveToTempLocationAndBack<'a> {
    source_location: &'a Path,
    temp_location: &'a Path,
    files_to_move: &'a HashSet<PathBuf>,
}

impl <'a>MoveToTempLocationAndBack<'a> {
    pub fn create(source_location: &'a Path, temp_location: &'a Path, files_to_move: &'a HashSet<PathBuf>) -> Self {
        MoveToTempLocationAndBack {
            source_location,
            temp_location, 
            files_to_move,
        }
    }

    pub fn move_to_temp_location(&self) -> Result<()> {
        for file_to_move in self.files_to_move.iter() {
            let src = self.source_location.join(file_to_move);
            let dest = self.temp_location.join(file_to_move);
            if let Some(parent) = dest.parent() {
                create_dir_all(parent)?;
            }
            rename(&src, &dest).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to move {:?} to {:?}: {}", src, dest, e)))?;
        }
        Ok(())
    }
}

impl <'a>Drop for MoveToTempLocationAndBack<'a> {
    fn drop(&mut self) {
        for file_to_move in self.files_to_move {
            let src = self.temp_location.join(file_to_move);
            let dest = self.source_location.join(file_to_move);
            rename(&src, &dest).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to move {:?} to {:?}: {}", src, dest, e))).unwrap();
        }
        remove_dir_all(self.temp_location).unwrap();
        create_dir_all(self.temp_location).unwrap();
    }
}