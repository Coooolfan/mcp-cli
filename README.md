# mcp-cli

轻量的 Rust 命令行工具和库，用来和 [MCP Model Context Protocol](https://modelcontextprotocol.io/) 服务器互动。

> 简体中文 | [English](./README.en.md)

* * *

## 功能特色

- **轻量**：纯 Rust 实现，依赖少，启动快。
- **独立可执行文件**：可通过 Cargo 编译成完整优化的独立可执行文件，不需要外部运行时。
- **Rust 库**：可在其他 Rust 应用程序中使用相同的 MCP 客户端功能。
- **适合 Shell 使用**：`call` 命令输出 JSON，适合搭配 `jq`、管道和 shell 脚本。
- **面向 Agent**：设计给 AI coding agents 使用，例如 Gemini CLI、Claude Code 等。
- **通用连接**：支持 stdio 与 HTTP MCP 服务器。
- **连接池**：按需启动后台进程以复用连接，默认空闲 60 秒后退出。
- **工具过滤**：可通过配置限制每个服务器允许或禁用的工具。
- **服务器说明**：可在输出中显示 MCP 服务器提供的使用说明。
- **可操作错误信息**：错误包含结构化代码、可用服务器与修复建议。

![mcp-cli](./comparison.jpeg)

* * *

## 快速开始

### 1. 安装

```bash
curl -fsSL https://raw.githubusercontent.com/Coooolfan/mcp-cli/main/install.sh | bash
```

或：

```bash
# 需要先安装 Cargo
cargo install --git https://github.com/Coooolfan/mcp-cli
```

Windows x64 和 ARM64 可以使用上面的 Cargo 安装命令，也可以从 [Releases](https://github.com/Coooolfan/mcp-cli/releases) 下载对应的 `mcp-cli-win-x64.exe` 或 `mcp-cli-win-arm64.exe`，重命名为 `mcp-cli.exe`，并将所在目录加入 `PATH`。本项目不再通过 npm 分发。

### 2. 建立配置文件

在当前目录或 `~/.config/mcp/` 建立 `mcp_servers.json`：

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "."
      ]
    },
    "deepwiki": {
      "url": "https://mcp.deepwiki.com/mcp"
    }
  }
}
```

Windows 中 `~` 对应 `USERPROFILE` 用户目录，未设置时使用 `HOME`。配置也可以用 `-c` 显式指定，多个配置文件不会合并。JSON 中的 Windows 路径应写成 `C:\tools\server.exe` 或 `C:/tools/server.exe`。

Windows 支持 `.exe`、`.com`、`.cmd`、`.bat` 命令，也支持从 `PATH` 中查找 `npx` / `uvx` 等程序。命令与参数分开填写，不要把整条 shell 命令写进 `command`。脚本需要相应的运行环境；运行本工具本身不需要 Node.js。

### 3. 查看可用工具

```bash
# 列出所有 servers 与 tools
mcp-cli

# 显示 descriptions
mcp-cli -d
```

### 4. 调用工具

```bash
# 先查看 tool schema
mcp-cli info filesystem read_file

# 调用 tool
mcp-cli call filesystem read_file '{"path": "./README.md"}'
```

* * *

### Windows PowerShell 示例

PowerShell 可以通过标准输入传递 JSON，避免不同版本的命令行引号处理差异：

```powershell
mcp-cli -c "$env:USERPROFILE/.config/mcp/mcp_servers.json" info filesystem
'{"path":"./README.md"}' | mcp-cli call filesystem read_file
$env:MCP_DEBUG = "1"
$env:MCP_NO_DAEMON = "1" # 按需关闭后台连接复用
```

下文的管道和循环示例采用 Bash 语法，需要 Bash 与相应的 `jq` 等工具。

## 使用方式

```text
mcp-cli [options]                             列出所有 servers 与 tools
mcp-cli [options] info <server>               显示 server tools 与参数
mcp-cli [options] info <server> <tool>        显示 tool schema
mcp-cli [options] grep <pattern>              用 glob pattern 搜索 tools
mcp-cli [options] call <server> <tool>        调用 tool；未提供 args 时从 stdin 读取 JSON
mcp-cli [options] call <server> <tool> <json> 使用 JSON arguments 调用 tool
```

`info <server> <tool>` 与 `info <server>/<tool>` 两种格式都可以使用。

> [!TIP]
> 在任何命令加上 `-d` 可显示 descriptions。

### 选项

| 选项 | 说明 |
|---|---|
| `-h, --help` | 显示 help message |
| `-v, --version` | 显示版本号 |
| `-d, --with-descriptions` | 显示 tool descriptions |
| `-c, --config <path>` | 指定配置文件路径 |

### 输出

| Stream | 内容 |
|---|---|
| stdout | Tool results 与 human-readable info |
| stderr | Errors 与 diagnostics |

### Rust 库

在另一个 Rust project 加入 crate：

```toml
[dependencies]
mcp-cli = { git = "https://github.com/Coooolfan/mcp-cli" }
```

使用 `McpClient` 执行常见流程：

```rust
use mcp_cli::McpClient;

#[tokio::main]
async fn main() -> Result<(), mcp_cli::CliError> {
    let client = McpClient::load(None)?;

    for server in client.server_names() {
        let tools = client.list_tools(&server).await?;
        println!("{server}: {} tools", tools.len());
    }

    let result = client
        .call_tool(
            "filesystem",
            "read_file",
            serde_json::json!({ "path": "./README.md" }),
        )
        .await?;

    println!("{}", mcp_cli::output::format_tool_result(&result));
    Ok(())
}
```

这个 crate 也提供较底层的 modules，例如 `mcp_cli::client`、`mcp_cli::config`、`mcp_cli::errors` 与 `mcp_cli::output`，适合需要直接控制连接或格式化的调用端使用。

* * *

## 命令示例

### 列出服务器

```bash
# 基本列表
$ mcp-cli
github
  • search_repositories
  • get_file_contents
  • create_or_update_file
filesystem
  • read_file
  • write_file
  • list_directory

# 显示 descriptions
$ mcp-cli --with-descriptions
github
  • search_repositories - Search for GitHub repositories
  • get_file_contents - Get contents of a file or directory
filesystem
  • read_file - Read the contents of a file
  • write_file - Write content to a file
```

### 搜索 Tools

```bash
# 在所有 servers 中寻找 file 相关 tools
$ mcp-cli grep "*file*"
github/get_file_contents
github/create_or_update_file
filesystem/read_file
filesystem/write_file

# 搭配 descriptions 搜索
$ mcp-cli grep "*search*" -d
github/search_repositories - Search for GitHub repositories
```

### 查看服务器详细信息

```bash
$ mcp-cli info github
Server: github
Transport: stdio
Command: npx -y @modelcontextprotocol/server-github

Tools (12):
  search_repositories
    Search for GitHub repositories
    Parameters:
      • query (string, required) - Search query
      • page (number, optional) - Page number
  ...
```

### 查看工具参数定义

```bash
# 两种格式都可以：
$ mcp-cli info github search_repositories
$ mcp-cli info github/search_repositories

Tool: search_repositories
Server: github

Description:
  Search for GitHub repositories

Input Schema:
  {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Search query" },
      "page": { "type": "number" }
    },
    "required": ["query"]
  }
```

### 调用工具

```bash
# 使用 inline JSON
$ mcp-cli call github search_repositories '{"query": "mcp server", "per_page": 5}'

# call 命令默认输出 JSON，适合 pipe 给 jq
$ mcp-cli call github search_repositories '{"query": "mcp"}' | jq '.content[0].text'

# 从 stdin 读取 JSON，不需要使用 -
$ echo '{"path": "./README.md"}' | mcp-cli call filesystem read_file
```

### 复杂命令

如果 JSON arguments 包含单引号、特殊字元或长文字，建议使用 stdin，避免 shell escaping 问题：

```bash
# 使用 heredoc；call subcommand 不需要 -
mcp-cli call server tool <<EOF
{"content": "Text with 'single quotes' and \"double quotes\""}
EOF

# 从文件读取
cat args.json | mcp-cli call server tool

# 使用 jq 建立复杂 JSON
jq -n '{query: "mcp", filters: ["active", "starred"]}' | mcp-cli call github search
```

**使用 stdin 的原因**：shell 会解读 `{}`、引号与特殊字元，因此需要谨慎 escaping；stdin 可以完全避开 shell 解析。

### 进阶串接示例

可以用 pipes 与 shell tools 串接多个 MCP calls：

```bash
# 1. 搜索并读取：找出符合 pattern 的文件，然后读取第一个
mcp-cli call filesystem search_files '{"path": "src/", "pattern": "*.ts"}' \
  | jq -r '.content[0].text | split("\n")[0]' \
  | xargs -I {} mcp-cli call filesystem read_file '{"path": "{}"}'

# 2. 处理多笔结果：读取所有符合条件的文件
mcp-cli call filesystem search_files '{"path": ".", "pattern": "*.md"}' \
  | jq -r '.content[0].text | split("\n")[]' \
  | while read file; do
      echo "=== $file ==="
      mcp-cli call filesystem read_file "{\"path\": \"$file\"}" | jq -r '.content[0].text'
    done

# 3. 撷取与转换：取得 repo 信息并撷取 URL
mcp-cli call github search_repositories '{"query": "mcp server", "per_page": 5}' \
  | jq -r '.content[0].text | fromjson | .items[].html_url'

# 4. 条件式执行：读取前先检查文件是否存在
mcp-cli call filesystem list_directory '{"path": "."}' \
  | jq -e '.content[0].text | contains("README.md")' \
  && mcp-cli call filesystem read_file '{"path": "./README.md"}'

# 5. 将输出保存到文件
mcp-cli call github get_file_contents '{"owner": "user", "repo": "project", "path": "src/main.ts"}' \
  | jq -r '.content[0].text' > main.ts

# 6. 在 scripts 中处理错误
if result=$(mcp-cli call filesystem read_file '{"path": "./config.json"}' 2>/dev/null); then
  echo "$result" | jq '.content[0].text | fromjson'
else
  echo "File not found, using defaults"
fi

# 7. 汇整多个 servers 的结果
{
  mcp-cli call github search_repositories '{"query": "mcp", "per_page": 3}'
  mcp-cli call filesystem list_directory '{"path": "./src"}'
} | jq -s '.'
```

串接时的注意事项：

- 使用 `jq -r` 取得 raw output，避免字串外层引号。
- 使用 `jq -e` 做条件检查；false 会回传 exit code 1。
- 测试时可用 `2>/dev/null` 隐藏错误输出。
- 使用 `| jq -s '.'` 合并多笔 JSON 输出。

* * *

## 配置

### 配置文件格式

CLI 使用 `mcp_servers.json`，格式相容 Claude Desktop、Gemini 与 VS Code：

```json
{
  "mcpServers": {
    "local-server": {
      "command": "node",
      "args": ["./server.js"],
      "env": {
        "API_KEY": "${API_KEY}"
      },
      "cwd": "/path/to/directory"
    },
    "remote-server": {
      "url": "https://mcp.example.com",
      "headers": {
        "Authorization": "Bearer ${TOKEN}"
      }
    }
  }
}
```

**环境变量替换**：配置文件中任何位置都可使用 `${VAR_NAME}` 语法，CLI 加载时会替换成环境变量值。默认情况下，缺少环境变量会产生清楚的错误信息。配置 `MCP_STRICT_ENV=false` 则会改用空值并显示 warning。

### 工具过滤

可使用 `allowedTools` 与 `disabledTools` 限制 server 可用的 tools：

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
      "allowedTools": ["read_file", "list_directory"],
      "disabledTools": ["delete_file"]
    }
  }
}
```

规则：

- `allowedTools`：只有符合 patterns 的 tools 可用，支持 glob：`*`、`?`。
- `disabledTools`：符合 patterns 的 tools 会被排除。
- **`disabledTools` 优先于 `allowedTools`**。
- Filtering 会套用到所有 CLI operations，包括 info、grep 与 call。

示例：

```json
// 只允许 read operations
"allowedTools": ["read_*", "list_*", "search_*"]

// 允许全部，但排除破坏性 operations
"disabledTools": ["delete_*", "write_*", "create_*"]

// 组合使用：允许 file operations，但停用 delete
"allowedTools": ["*file*"],
"disabledTools": ["delete_file"]
```

### 配置文件解析顺序

CLI 会依序寻找以下配置来源：

1. `-c/--config` 命令行参数
2. `MCP_CONFIG_PATH` 环境变量
3. `./mcp_servers.json`，目前目录
4. `~/.mcp_servers.json`
5. `~/.config/mcp/mcp_servers.json`

### 环境变量

| Variable | 说明 | 默认值 |
|---|---|---|
| `MCP_CONFIG_PATH` | 配置文件路径 | 无 |
| `MCP_DEBUG` | 启用 debug output | `false` |
| `MCP_TIMEOUT` | Request timeout，单位为秒 | `1800`，30 分钟 |
| `MCP_CONCURRENCY` | 平行处理的 servers 数量，不是总量限制 | `5` |
| `MCP_MAX_RETRIES` | 短暂错误的 retry 次数，`0` 表示停用 | `3` |
| `MCP_RETRY_DELAY` | Retry base delay，单位为毫秒 | `1000` |
| `MCP_STRICT_ENV` | 配置文件中 `${VAR}` 缺失时是否报错 | `true` |
| `MCP_NO_DAEMON` | 停用连接快取，强制每次建立新连接 | `false` |
| `MCP_DAEMON_TIMEOUT` | Cached connections 的 idle timeout，单位为秒 | `60` |

* * *

## 搭配 AI Agents 使用

`mcp-cli` 的设计目标之一，是让 AI coding agents 访问 MCP servers。MCP 可让 AI models 通过标准化 protocol 与外部 tools、APIs 与 data sources 互动。

### 为什么使用 MCP + CLI

传统 MCP integration 会把完整 tool schemas 加载 AI 的 context window，可能耗用数千 tokens。CLI 模式的特点如下：

- **按需加载**：只有需要时才取得 schema。
- **节省 tokens**：context overhead 很低。
- **可与 shell 组合**：可搭配 `jq`、pipes 与 scripts。
- **可 script 化**：AI 可以撰写 shell scripts 处理复杂工作流程。

### 方式 1：整合到 System Prompt

把以下内容加入 AI agent 的 system prompt，即可让 agent 直接使用 CLI：

````xml
## MCP Servers

You have access to MCP servers via the `mcp-cli` CLI.

Commands:

```bash
mcp-cli info                        # List all servers
mcp-cli info <server>               # Show server tools
mcp-cli info <server> <tool>        # Get tool schema
mcp-cli grep "<pattern>"            # Search tools
mcp-cli call <server> <tool>        # Call tool (stdin auto-detected)
mcp-cli call <server> <tool> '{}'   # Call with JSON args
```

**Both formats work:** `info <server> <tool>` or `info <server>/<tool>`

Workflow:

1. **Discover**: `mcp-cli info` to see available servers
2. **Inspect**: `mcp-cli info <server> <tool>` to get the schema
3. **Execute**: `mcp-cli call <server> <tool> '{}'` with arguments

### Examples

```bash
# Call with inline JSON
mcp-cli call github search_repositories '{"query": "mcp server"}'

# Pipe from stdin (no '-' needed)
echo '{"path": "./file"}' | mcp-cli call filesystem read_file

# Heredoc for complex JSON
mcp-cli call server tool <<EOF
{"content": "Text with 'quotes'"}
EOF
```

### Common Errors

| Wrong | Error | Fix |
|---|---|---|
| `mcp-cli server tool` | AMBIGUOUS | Use `call server tool` |
| `mcp-cli run server tool` | UNKNOWN_SUBCOMMAND | Use `call` |
| `mcp-cli list` | UNKNOWN_SUBCOMMAND | Use `info` |
````

### 方式 2：Agents Skill

支持 Agents Skills 的 code agents，例如 Gemini CLI、OpenCode 或 Claude Code，可以使用 mcp-cli skill 与 MCP servers 互动。Skill 可在 [SKILL.md](./SKILL.md) 取得。

请在你的 skills directory 建立 `mcp-cli/SKILL.md`。

* * *

## 架构

### 连接池 Daemon

Linux/macOS 使用 Unix socket；Windows 使用本机命名管道，记录文件存放在系统用户临时目录的 `mcp-cli` 子目录。Windows 后台进程使用独立进程组，Stdio 子进程由 Windows Job Object 管理，在连接关闭或后台进程退出时清理子进程树。下图以 Unix socket 为例。


CLI 默认使用 **lazy-spawn connection pooling**，避免重复启动 MCP server 造成延迟：

```text
┌────────────────────────────────────────────────────────────────────┐
│                        First CLI Call                              │
│   $ mcp-cli info server                                            │
└────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌────────────────────────────────────────────────────────────────────┐
│ Check: /tmp/mcp-cli-{uid}/server.sock exists?                      │
└────────────────────────────────────────────────────────────────────┘
         │                                    │
         │ NO                                 │ YES
         ▼                                    ▼
┌─────────────────────────┐      ┌───────────────────────────────────┐
│ Fork background daemon  │      │ Connect to existing socket        │
│ ├─ Connect to MCP server│      │ ├─ Send request via IPC           │
│ ├─ Create Unix socket   │      │ ├─ Receive response               │
│ └─ Start 60s idle timer │      │ └─ Daemon resets idle timer       │
└─────────────────────────┘      └───────────────────────────────────┘
         │                                    │
         └────────────────┬───────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────────────┐
│ On idle timeout (60s): Daemon self-terminates, cleans up files     │
└────────────────────────────────────────────────────────────────────┘
```

主要特性：

- **自动化**：不需要手动 start 或 stop。
- **每个 server 独立**：每个 MCP server 都有自己的 daemon。
- **Stale detection**：配置变更会触发重新 spawn。
- **快速 fallback**：spawn timeout 为 5 秒，超时后改用 direct connection。

通过环境变量控制：

```bash
MCP_NO_DAEMON=1 mcp-cli info      # 强制建立新连接
MCP_DAEMON_TIMEOUT=120 mcp-cli    # 2 分钟 idle timeout
MCP_DEBUG=1 mcp-cli info          # 显示 daemon debug output
```

### 直接连接模型

停用 daemon 时，`MCP_NO_DAEMON=1`，CLI 会使用 **lazy, on-demand connection strategy**。只有需要时才建立 server connections，使用完立即关闭。

```text
┌─────────────────────────────────────────────────────────────────┐
│                         USER REQUEST                            │
└─────────────────────────────────────────────────────────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
              ▼                 ▼                 ▼
    ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
    │   mcp-cli info  │ │ mcp-cli grep    │ │ mcp-cli call    │
    │   (list all)    │ │   "*pattern*"   │ │  server tool {} │
    └─────────────────┘ └─────────────────┘ └─────────────────┘
              │                 │                 │
              ▼                 ▼                 ▼
    ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
    │  Connect to ALL │ │  Connect to ALL │ │  Connect to ONE │
    │  servers (N)    │ │  servers (N)    │ │  server only    │
    └─────────────────┘ └─────────────────┘ └─────────────────┘
              │                 │                 │
              ▼                 ▼                 ▼
         List tools       Search tools       Execute tool
              │                 │                 │
              ▼                 ▼                 ▼
    ┌─────────────────────────────────────────────────────────────┐
    │                    CLOSE CONNECTIONS                        │
    └─────────────────────────────────────────────────────────────┘
```

Server 连接时机：

| Command | 连接的 servers |
|---|---|
| `mcp-cli info` | 平行连接所有 N 个 servers |
| `mcp-cli grep "*pattern*"` | 平行连接所有 N 个 servers |
| `mcp-cli info <server>` | 只连接指定 server |
| `mcp-cli info <server> <tool>` | 只连接指定 server |
| `mcp-cli call <server> <tool> '{}'` | 只连接指定 server |

### 错误处理与 Retry

CLI 对短暂性 failures 内建 **exponential backoff 自动 retry**。

会自动 retry 的短暂错误：

- Network：`ECONNREFUSED`、`ETIMEDOUT`、`ECONNRESET`
- HTTP：`502`、`503`、`504`、`429`

不会 retry、会立即失败的错误：

- Config：Invalid JSON、missing fields
- Auth：`401`、`403`
- Tool：Validation errors、not found

* * *

## 开发

### 前置需求

- [Rust](https://www.rust-lang.org/) 稳定版工具链；Windows 使用 MSVC 工具链及 Visual Studio C++ Build Tools
- [Make](https://www.gnu.org/software/make/) 可选，用于执行 Makefile tasks

### 配置

```bash
git clone https://github.com/Coooolfan/mcp-cli.git
cd mcp-cli
```

### Makefile 任务

项目包含 `Makefile`，用于常见的开发、测试、安装与发布工作。执行 `make` 或 `make help` 可查看所有可用命令：

```bash
$ make help
mcp-cli Management Tasks:
--------------------------------------------------------
all                Build the application in release mode (fully optimized)
build              Build the application in debug mode
build-release      Build the application in release mode (fully optimized)
clean              Remove compiled target files
clippy             Run clippy for static analysis and lint checks
fmt                Check and enforce Rust formatting standards
fmt-fix            Format all Rust source files automatically
help               Show this help menu with descriptions of each command
install            Install the compiled release binary to ~/.local/bin
release            Trigger a new release (usage: make release VERSION=X.Y.Z)
test               Run the native Rust unit tests
uninstall          Remove the mcp-cli binary from ~/.local/bin
--------------------------------------------------------
```

### 本机安装与测试

所有支持的平台都可以直接使用 Cargo，无需 Make：

```text
cargo install --path .
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

以下 Makefile 安装方式适用于 Linux 和 macOS。


若要编译并安装 CLI 到本机 `~/.local/bin`，执行：

```bash
make install
```

请确认 `~/.local/bin` 已在你的 `PATH` 中，例如放在 `~/.zshrc` 或 `~/.bashrc`：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

安装完成后，可从任何位置执行 CLI：

```bash
mcp-cli --help
```

移除 binary：

```bash
make uninstall
```

### 发布

Releases 通过 GitHub Actions 自动化。可使用 Makefile task 触发 release，该 task 会调用 `scripts/release.sh`：

```bash
make release VERSION=0.3.0
```

### 错误信息

所有错误都包含可操作的修复建议，适合人类与 AI agents 使用：

```text
Error [AMBIGUOUS_COMMAND]: Ambiguous command: did you mean to call a tool or view info?
  Details: Received: mcp-cli filesystem read_file
  Suggestion: Use 'mcp-cli call filesystem read_file' to execute, or 'mcp-cli info filesystem read_file' to view schema

Error [UNKNOWN_SUBCOMMAND]: Unknown subcommand: "run"
  Details: Valid subcommands: info, grep, call
  Suggestion: Did you mean 'mcp-cli call'?

Error [SERVER_NOT_FOUND]: Server "github" not found in config
  Details: Available servers: filesystem, sqlite
  Suggestion: Use one of: mcp-cli info filesystem, mcp-cli info sqlite

Error [TOOL_NOT_FOUND]: Tool "search" not found in server "filesystem"
  Details: Available tools: read_file, write_file, list_directory (+5 more)
  Suggestion: Run 'mcp-cli info filesystem' to see all available tools

Error [INVALID_JSON_ARGUMENTS]: Invalid JSON in tool arguments
  Details: Parse error: Unexpected identifier "test"
  Suggestion: Arguments must be valid JSON. Use single quotes: '{"key": "value"}'
```

* * *

## Credits

- 感谢 [doggy8088/mcp-cli](https://github.com/doggy8088/mcp-cli)，本项目基于其工作继续开发。
- [Philipp Schmid](https://github.com/philschmid)：原始且优秀的 [Bun/TypeScript mcp-cli](https://github.com/philschmid/mcp-cli) 项目的作者。
- [Antigravity](https://github.com/google-deepmind)：重新设计代码，并完整改写为高度优化、无需外部运行时的纯 Rust 命令行工具。

* * *

## 许可证

采用 MIT 许可证，详细内容请见 [LICENSE](LICENSE)。

* * *

## 参与贡献

欢迎通过 Pull Request 参与贡献。
