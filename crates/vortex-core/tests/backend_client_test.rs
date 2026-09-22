use std::fs;
use tempfile::tempdir;
use vortex_core::backend::BackendClient;

#[test]
fn test_backend_client_ping() {
    let client = BackendClient::new().expect("Failed to create BackendClient");
    assert!(client.ping().unwrap());
}

#[test]
fn test_backend_client_read_and_write_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.tex");
    let file_path_str = file_path.to_str().unwrap();

    let client = BackendClient::new().unwrap();
    let content = "\\section{Hello Rust}";

    assert!(client.write_file(file_path_str, content).unwrap());
    let read_back = client.read_file(file_path_str).unwrap();
    assert_eq!(read_back, content);
}

#[test]
fn test_backend_client_tree_and_project_init() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("chapters");
    fs::create_dir(&sub).unwrap();

    let main_tex = dir.path().join("main.tex");
    fs::write(&main_tex, "\\section{Main}\n\\label{sec:main}\n\\cite{paper1}").unwrap();

    let ch1 = sub.join("ch1.tex");
    fs::write(&ch1, "\\subsection{Chapter 1}\n\\label{sec:ch1}").unwrap();

    let client = BackendClient::new().unwrap();
    let root_str = dir.path().to_str().unwrap();

    let tree = client.read_tree(root_str).unwrap();
    assert!(!tree.is_empty());

    // Init project
    assert!(client.init_project(root_str).unwrap());

    // Fuzzy search labels
    let matches = client.fuzzy_search_labels("main", None).unwrap();
    assert!(!matches.is_empty());

    let stats = client.get_stats().unwrap();
    assert!(stats.files_indexed >= 2);
    assert_eq!(stats.labels, 2);
}
