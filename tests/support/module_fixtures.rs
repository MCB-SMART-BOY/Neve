use std::fs;
use std::path::Path;

pub fn create_test_module(dir: &Path, path: &[&str], content: &str) {
    let mut full_path = dir.to_path_buf();
    for (i, segment) in path.iter().enumerate() {
        full_path.push(segment);
        if i < path.len() - 1 {
            fs::create_dir_all(&full_path).unwrap();
        }
    }
    full_path.set_extension("neve");
    fs::write(full_path, content).unwrap();
}
