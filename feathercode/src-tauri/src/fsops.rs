use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub line: u64,
    pub text: String,
}

#[tauri::command]
pub fn fs_list(path: String) -> Result<Vec<FileEntry>, String> {
    let mut out = Vec::new();
    let rd = fs::read_dir(&path).map_err(|e| format!("cannot list {path}: {e}"))?;
    for e in rd.flatten() {
        let p = e.path();
        let is_dir = p.is_dir();
        let size = if is_dir { 0 } else { e.metadata().map(|m| m.len()).unwrap_or(0) };
        out.push(FileEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir,
            size,
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

#[tauri::command]
pub fn fs_read(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[tauri::command]
pub fn fs_write(path: String, content: String) -> Result<(), String> {
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).map_err(|e| format!("cannot write {path}: {e}"))
}

#[tauri::command]
pub fn fs_create(path: String, is_dir: bool) -> Result<(), String> {
    if is_dir {
        fs::create_dir_all(&path).map_err(|e| format!("cannot create {path}: {e}"))
    } else {
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::File::create(&path)
            .map(|_| ())
            .map_err(|e| format!("cannot create {path}: {e}"))
    }
}

#[tauri::command]
pub fn fs_delete(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        fs::remove_file(p).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn fs_rename(from: String, to: String) -> Result<(), String> {
    fs::rename(&from, &to).map_err(|e| format!("cannot rename: {e}"))
}

#[tauri::command]
pub fn fs_search(root: String, query: String, case_sensitive: bool) -> Result<Vec<SearchHit>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    walk(Path::new(&root), &query, case_sensitive, &mut hits, 0);
    Ok(hits)
}

const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "dist", "build", "__pycache__", ".venv", "venv"];

fn walk(dir: &Path, query: &str, cs: bool, hits: &mut Vec<SearchHit>, depth: u32) {
    if depth > 10 || hits.len() >= 300 {
        return;
    }
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(&p, query, cs, hits, depth + 1);
        } else {
            if let Ok(content) = fs::read_to_string(&p) {
                for (i, line) in content.lines().enumerate() {
                    let (hay, needle) = if cs {
                        (line.to_string(), query.to_string())
                    } else {
                        (line.to_lowercase(), query.to_lowercase())
                    };
                    if hay.contains(&needle) {
                        hits.push(SearchHit {
                            path: p.to_string_lossy().to_string(),
                            line: (i + 1) as u64,
                            text: line.chars().take(200).collect(),
                        });
                        break; // one hit per file keeps results diverse
                    }
                }
            }
        }
        if hits.len() >= 300 {
            return;
        }
    }
}
