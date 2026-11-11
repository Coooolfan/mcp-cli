# 发布说明

每个版本使用独立文件：`docs/release_notes/<version>.md`。版本号不带 `v`，例如 `0.3.0.md`、`0.3.0-rc.1.md`。

发布说明必须非空，并在运行 CD 前提交到所选分支。正文会直接作为 GitHub Release 的内容，不需要重复版本标题。可使用仓库内的 [release-note-creator](../../.agents/skills/release-note-creator/SKILL.md) 根据实际提交生成说明。

CD 由 Actions 页面手动触发，输入 `version` 和 `latest`；详见 [README](../../README.md#发布)。
