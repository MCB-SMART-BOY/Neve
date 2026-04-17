use neve_derive::Hash;
use neve_fetch::git::hash_directory;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub fn init_local_git_repo() -> (TempDir, String, String) {
    let temp = TempDir::new().unwrap();
    let repo_dir = temp.path().join("repo");

    let status = Command::new("git")
        .args(["init", "--quiet", "-b", "main"])
        .arg(&repo_dir)
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");

    for (key, value) in [
        ("user.email", "tests@example.com"),
        ("user.name", "Neve Tests"),
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["config", key, value])
            .status()
            .expect("git config should run");
        assert!(status.success(), "git config should succeed");
    }

    fs::create_dir_all(repo_dir.join("nested")).unwrap();
    fs::write(repo_dir.join("source.txt"), b"fetch-git-content").unwrap();
    fs::write(repo_dir.join("nested").join("more.txt"), b"more-content").unwrap();

    let status = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["add", "."])
        .status()
        .expect("git add should run");
    assert!(status.success(), "git add should succeed");

    let status = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["commit", "--quiet", "-m", "initial"])
        .status()
        .expect("git commit should run");
    assert!(status.success(), "git commit should succeed");

    let checkout_dir = temp.path().join("expected-checkout");
    let status = Command::new("git")
        .args(["clone", "--quiet"])
        .arg(&repo_dir)
        .arg(&checkout_dir)
        .status()
        .expect("git clone should run");
    assert!(status.success(), "git clone should succeed");

    let status = Command::new("git")
        .arg("-C")
        .arg(&checkout_dir)
        .args(["checkout", "--quiet", "main"])
        .status()
        .expect("git checkout should run");
    assert!(status.success(), "git checkout should succeed");

    fs::remove_dir_all(checkout_dir.join(".git")).unwrap();
    let expected_hash = hash_directory(&checkout_dir).unwrap().to_hex();

    (temp, repo_dir.to_string_lossy().into_owned(), expected_hash)
}

pub fn start_local_http_fixture(body: &[u8]) -> (String, String, thread::JoinHandle<()>) {
    let temp = TempDir::new().unwrap();
    let route = format!(
        "{}.txt",
        temp.path()
            .file_name()
            .expect("temp dir name should exist")
            .to_string_lossy()
    );
    let expected_hash = Hash::of(body).to_hex();
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    listener
        .set_nonblocking(true)
        .expect("listener should become nonblocking");
    let addr = listener
        .local_addr()
        .expect("listener should have local addr");
    let response_body = body.to_vec();
    let response_route = route.clone();

    let server = thread::spawn(move || {
        let started = Instant::now();
        let mut served = false;
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buffer = [0u8; 4096];
                    let _ = stream.read(&mut buffer);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",
                        response_body.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("response head should write");
                    stream
                        .write_all(&response_body)
                        .expect("response body should write");
                    stream.flush().expect("response should flush");
                    served = true;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if served && started.elapsed() >= Duration::from_millis(200) {
                        break;
                    }
                    if !served && started.elapsed() >= Duration::from_secs(5) {
                        panic!("fixture server for {response_route} timed out waiting for request");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("fixture server for {response_route} failed: {err}"),
            }
        }
    });

    (format!("http://{addr}/{route}"), expected_hash, server)
}
