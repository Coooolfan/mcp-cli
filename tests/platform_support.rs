use mcp_cli::config::{get_pid_path, get_socket_path};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};
use tokio::process::Command;

fn fixture() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("mcp fixture 中文 {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let executable = dir.join(format!("server{}", std::env::consts::EXE_SUFFIX));
        let status = std::process::Command::new("rustc")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stdio_server.rs"))
            .args(["--edition=2021", "-o"])
            .arg(&executable)
            .status()
            .unwrap();
        assert!(status.success());
        executable
    })
}

struct Server {
    name: String,
    dir: PathBuf,
    config: PathBuf,
}
impl Server {
    fn new(server: Value) -> Self {
        let name = format!("test-{}-{}", std::process::id(), rand::random::<u64>());
        let dir = std::env::temp_dir().join(&name);
        std::fs::create_dir_all(&dir).unwrap();
        let config = dir.join("配置 file.json");
        let result = Self { name, dir, config };
        result.write(server);
        result
    }
    fn write(&self, server: Value) {
        std::fs::write(
            &self.config,
            json!({"mcpServers": {&self.name: server}}).to_string(),
        )
        .unwrap();
    }
    async fn call(&self, daemon: bool, args: &str) -> String {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mcp-cli"));
        command
            .args([
                "-c",
                self.config.to_str().unwrap(),
                "call",
                &self.name,
                "pid",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .env("MCP_NO_DAEMON", if daemon { "0" } else { "1" })
            .env("MCP_DAEMON_TIMEOUT", "2")
            .env("MCP_MAX_RETRIES", "0");
        let mut child = command.kill_on_drop(true).spawn().unwrap();
        use tokio::io::AsyncWriteExt;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(args.as_bytes())
            .await
            .unwrap();
        let output = tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(pid) = mcp_cli::daemon::read_pid_file(&self.name) {
            mcp_cli::daemon::kill_process(pid.pid);
        }
        mcp_cli::daemon::remove_pid_file(&self.name);
        mcp_cli::daemon::remove_socket_file(&self.name);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn stdio_direct_and_daemon_reuse_restart_and_large_requests() {
    let server = Server::new(json!({"command": fixture()}));
    let direct = server.call(false, "{}").await;
    assert!(!get_pid_path(&server.name).exists());
    let first = server.call(true, "{}").await;
    assert_ne!(direct, first);
    assert!(
        get_pid_path(&server.name).exists(),
        "must use daemon, not fallback"
    );
    assert_eq!(
        first,
        server
            .call(true, &json!({"large": "x".repeat(160_000)}).to_string())
            .await
    );
    let pid = mcp_cli::daemon::read_pid_file(&server.name).unwrap().pid;
    assert!(mcp_cli::daemon::is_process_running(pid));
    server.write(json!({"command": fixture(), "env": {"REVISION": "2"}}));
    let second = server.call(true, "{}").await;
    assert_ne!(first, second, "config changes must restart daemon");
    let pid = mcp_cli::daemon::read_pid_file(&server.name).unwrap().pid;
    assert!(mcp_cli::daemon::kill_process(pid));
    // A dead daemon with a leftover PID file must recover on the next call.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_ne!(second, server.call(true, "{}").await);
    tokio::time::timeout(Duration::from_secs(8), async {
        while get_pid_path(&server.name).exists() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("idle daemon should clean up its PID file");
    #[cfg(unix)]
    assert!(!get_socket_path(&server.name).exists());
    #[cfg(windows)]
    assert!(get_socket_path(&server.name)
        .to_string_lossy()
        .starts_with(r"\\.\pipe\"));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_batch_command_resolves_path_and_preserves_arguments() {
    let server = Server::new(json!({"command": fixture()}));
    let script = server.dir.join("fixture runner.cmd");
    // The fixture records startup through the configured environment.
    std::fs::write(
        &script,
        format!("@echo off\r\n\"{}\" %*\r\n", fixture().display()),
    )
    .unwrap();
    let log = server.dir.join("start.log");
    server.write(json!({"command": "fixture runner", "cwd": server.dir,
        "args": ["space value", "中文", "a&b", "a\"b"],
        "env": {"PATH": server.dir, "START_LOG": log,
            "EXPECT_ARGS": "space value|中文|a&b|a\"b", "EXPECT_CWD": server.dir}}));
    let first = server.call(true, "{}").await;
    assert_eq!(first, server.call(true, "{}").await);
    assert_eq!(std::fs::read_to_string(log).unwrap().lines().count(), 1);
}

#[tokio::test]
async fn http_direct_and_daemon_keep_session() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = Server::new(
        json!({"url": format!("http://{}/mcp", listener.local_addr().unwrap()), "headers": {"Authorization": "Bearer test"}}),
    );
    let initialized = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = initialized.clone();
    let task = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let count = count.clone();
            tokio::spawn(async move {
                let mut stream = BufReader::new(stream);
                let mut headers = String::new();
                loop {
                    let mut line = String::new();
                    stream.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    headers.push_str(&line);
                }
                let lower = headers.to_lowercase();
                assert!(lower.contains("authorization: bearer test"));
                let length = lower
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                let mut body = vec![0; length];
                stream.read_exact(&mut body).await.unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                let method = request["method"].as_str().unwrap();
                let result = match method {
                    "initialize" => {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        json!({"protocolVersion":"2024-11-05", "capabilities":{}, "serverInfo":{"name":"http-fixture","version":"1"}})
                    }
                    _ => {
                        assert!(lower.contains("mcp-session-id: fixture-session"));
                        json!({"content":[{"type":"text", "text":"http-ok"}]})
                    }
                };
                let response = if method == "notifications/initialized" {
                    "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_string()
                } else {
                    let body =
                        json!({"jsonrpc":"2.0", "id":request["id"], "result":result}).to_string();
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: fixture-session\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                };
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    assert_eq!(server.call(false, "{}").await, "http-ok");
    assert_eq!(server.call(true, "{}").await, "http-ok");
    assert_eq!(server.call(true, "{}").await, "http-ok");
    assert!(get_pid_path(&server.name).exists());
    assert_eq!(initialized.load(std::sync::atomic::Ordering::SeqCst), 2);
    task.abort();
}

#[tokio::test]
async fn user_configuration_search_uses_home_directory() {
    let server = Server::new(json!({"command": fixture()}));
    std::fs::copy(&server.config, server.dir.join(".mcp_servers.json")).unwrap();
    let empty = server.dir.join("empty");
    std::fs::create_dir(&empty).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mcp-cli"))
        .args(["info", &server.name])
        .current_dir(empty)
        .env("HOME", &server.dir)
        .env("USERPROFILE", &server.dir)
        .env_remove("MCP_CONFIG_PATH")
        .env("MCP_NO_DAEMON", "1")
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("pid"));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_closes_server_process_tree() {
    let server = Server::new(json!({"command": fixture()}));
    let child_file = server.dir.join("child.pid");
    server.write(json!({"command": fixture(), "env": {"CHILD_PID_FILE": child_file}}));
    for daemon in [false, true] {
        server.call(daemon, "{}").await;
        let child: i32 = std::fs::read_to_string(&child_file)
            .unwrap()
            .parse()
            .unwrap();
        if daemon {
            let pid = mcp_cli::daemon::read_pid_file(&server.name).unwrap().pid;
            assert!(mcp_cli::daemon::is_process_running(child));
            assert!(mcp_cli::daemon::kill_process(pid));
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while mcp_cli::daemon::is_process_running(child) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("server descendants must exit with the connection");
    }
}
