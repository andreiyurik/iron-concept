//! Reading a project from disk: `config/` and `test/sim/scenarios/`.

use crate::validate::Files;
use std::io;
use std::path::Path;

pub fn load(root: &Path) -> io::Result<Files> {
    if !root.join("config/tags").is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not an IRON project: config/tags/ not found", root.display())));
    }
    let mut files = Files::new();
    walk(root, &root.join("config"), &mut files)?;
    let scenarios = root.join("test/sim/scenarios");
    if scenarios.is_dir() {
        walk(root, &scenarios, &mut files)?;
    }
    Ok(files)
}

fn walk(root: &Path, dir: &Path, files: &mut Files) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, files)?;
        } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            let rel: Vec<String> =
                path.strip_prefix(root).unwrap_or(&path).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            // Name the file: the bare io error ("stream did not contain valid UTF-8") does not.
            let text = std::fs::read_to_string(&path).map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
            files.insert(rel.join("/"), text);
        }
    }
    Ok(())
}
