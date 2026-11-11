# Release Notes 书写规范

使用简体中文编写 mcp-cli 的发布说明，面向用户说明最终行为，保留可追溯的提交或 PR 链接。

## 结构

- 省略一级版本标题，GitHub Release 页面会显示版本号
- 使用二级标题 `## feat`、`## fix`、`## chore / docs`；必要时单列 `## perf`，没有内容的分组省略
- 条目使用 `*`，每条一句话，句末不加句号
- 描述后直接附上完整提交或 PR URL；使用本地 git 核实 SHA，不编造链接
- 最后一行写 `Full Changelog: https://github.com/Coooolfan/mcp-cli/compare/<prev>...<curr>`，替换为实际标签名；没有前序标签时省略

## 内容

- 使用“动词 + 对象 + 结果”，描述 CLI、Rust 库、MCP 连接、配置、平台兼容性等最终变化
- 不写 `feat:`、`fix(client):` 这类提交前缀
- 同一功能的多个提交合并成一条，可并列多个链接
- 过滤单纯版本更新、合并分支等无用户价值的记录
- `style`、`refactor`、`ci`、依赖和文档维护归入 `chore / docs`
- 涉及不兼容行为或迁移要求时，在正文前使用一到两个必要的 GitHub 警报块，例如 `> [!WARNING]`；不要堆叠泛泛的提醒
- 不额外引入 HTML、图片或表格

发布说明保存到 `docs/release_notes/<version>.md`，新版本号不带 `v`。历史比较基准若带 `v`，Full Changelog 中应保留它。发布说明应包含选定比较范围的完整变化，不能只重复最终一个提交的标题。
