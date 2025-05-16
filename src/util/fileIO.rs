use std::{fs::read_to_string, path::PathBuf};

pub fn parseFiles(paths: Vec<PathBuf>, followSymlinks: bool) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();

    for path in paths {
        let pathStr = path.to_string_lossy();

        let exists = path.try_exists().map_err(|e| {
            format!(
                "Failed to determine if the path '{}' exists: {}",
                pathStr, e
            )
        })?;

        if !exists {
            return Err(format!("The specified file '{}' does not exist", pathStr));
        } else if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            let entries = path.read_dir().map_err(|e| {
                format!("Failed to read contents of directory '{}': {}", pathStr, e)
            })?;
            let entries = entries.map(|p| p.unwrap().path()).collect();
            files.append(&mut parseFiles(entries, followSymlinks)?);
        } else if path.is_symlink() && followSymlinks {
            let target = path
                .read_link()
                .map_err(|e| format!("Failed to read symlink '{}': {}", pathStr, e))?;
            files.push(target);
        } else {
            panic!();
        }
    }

    Ok(files)
}

pub fn parseFileListings(
    listings: Vec<PathBuf>,
    followSymlinks: bool,
) -> Result<Vec<PathBuf>, String> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for listing in listings {
        for line in read_to_string(listing.clone())
            .map_err(|e| {
                format!(
                    "Failed to read entry from listing '{:?}': {}",
                    listing.to_str(),
                    e
                )
            })?
            .lines()
        {
            let mut path = PathBuf::new();
            path.push(line);

            let pathStr = path.to_string_lossy();
            let exists = path.try_exists().map_err(|e| {
                format!(
                    "Failed to determine if the path '{}' exists: {}",
                    pathStr, e
                )
            })?;

            if !exists {
                return Err(format!("The specified file '{}' does not exist", pathStr));
            } else {
                paths.push(path);
            }
        }
    }

    parseFiles(paths, followSymlinks)
}

#[cfg(test)]
mod test {
    use std::os::unix::fs::symlink;

    use super::*;
    use tempfile::{tempdir, tempdir_in, NamedTempFile};

    #[test]
    fn testParseFiles() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file = NamedTempFile::new_in(&dir).expect("Failed to create temp file");
        let filePath = file.path().to_path_buf();
        let subDir = tempdir_in(&dir).unwrap();
        let nestedFile = NamedTempFile::new_in(&subDir).unwrap();
        let nestedFilePath = nestedFile.path().to_path_buf();
        let subDirPath = subDir.path().to_path_buf();

        let symlinkPath = dir.path().join("symlinkToFile");
        symlink(&filePath, &symlinkPath).expect("Failed to create symlink");

        let inputPaths = vec![filePath.clone(), subDirPath.clone(), symlinkPath.clone()];
        let result = parseFiles(inputPaths.clone(), true).unwrap();

        assert!(result.contains(&filePath));
        assert!(result.contains(&nestedFilePath));
        assert!(result.contains(&filePath));
        assert_eq!(result.len(), 3);

        drop((file, nestedFile));
    }

    #[test]
    fn test_parse_file_listings() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::{tempdir, NamedTempFile};

        let dir = tempdir().expect("Failed to create temp dir");
        let file1 = NamedTempFile::new_in(&dir).expect("Failed to create temp file");
        let file2 = NamedTempFile::new_in(&dir).expect("Failed to create temp file");
        let file1Path = file1.path().to_path_buf();
        let file2Path = file2.path().to_path_buf();

        let listingPath = dir.path().join("listing.txt");
        let mut listingFile = File::create(&listingPath).expect("Failed to create listing file");
        writeln!(listingFile, "{}", file1Path.to_string_lossy()).unwrap();
        writeln!(listingFile, "{}", file2Path.to_string_lossy()).unwrap();

        let result = parseFileListings(vec![listingPath.clone()], false).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&file1Path));
        assert!(result.contains(&file2Path));
    }
}
