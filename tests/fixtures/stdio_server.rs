use std::io::{self, BufRead, Write};

fn main() {
    if std::env::args().any(|arg| arg == "--child") {
        std::thread::sleep(std::time::Duration::from_secs(60));
        return;
    }
    if let Ok(path) = std::env::var("CHILD_PID_FILE") {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--child").spawn().unwrap();
        std::fs::write(path, child.id().to_string()).unwrap();
    }

    if let Ok(expected) = std::env::var("EXPECT_ARGS") {
        assert_eq!(std::env::args().skip(1).collect::<Vec<_>>().join("|"), expected);
    }
    if let Ok(expected) = std::env::var("EXPECT_CWD") {
        assert_eq!(std::env::current_dir().unwrap(), std::path::PathBuf::from(expected));
    }
    if let Ok(log) = std::env::var("START_LOG") {
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log).unwrap();
        writeln!(file, "{}", std::process::id()).unwrap();
    }
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let Some(start) = line.find("\"id\":") else { continue };
        let id: String = line[start + 5..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
        let result = if line.contains("\"initialize\"") {
            r#"{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}"#.to_owned()
        } else if line.contains("\"tools/list\"") {
            r#"{"tools":[{"name":"pid","description":"Process identity","inputSchema":{"type":"object"}}]}"#.to_owned()
        } else if line.contains("\"tools/call\"") {
            format!(r#"{{"content":[{{"type":"text","text":"{}"}}]}}"#, std::process::id())
        } else { "{}".to_owned() };
        println!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#);
        io::stdout().flush().unwrap();
    }
}
