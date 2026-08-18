# Quick Command

面向 macOS 的本地项目快捷启动器。通过全局快捷键唤起，用简短、可预测的命令查找项目、浏览文件，并交给常用开发工具打开。

[English](README.en.md) · [开发文档](docs/ARCHITECTURE.md) · [发布说明](docs/RELEASE.md)

<!-- 在这里添加 Quick Command 主界面截图。 -->

## 为什么做 Quick Command

打开一个本地项目，通常需要先启动终端、切换目录，再输入编辑器命令。项目越多，路径记忆和重复导航带来的中断就越明显。

Quick Command 把这段流程收进一个 Spotlight 风格的轻量窗口：按下全局快捷键，输入项目名或受支持的命令，然后直接继续工作。它关注的是高频、本地、可验证的开发操作，而不是在桌面界面中重新实现完整终端。

## 主要功能

- **快速唤起**：通过可配置的全局快捷键显示或隐藏启动器，失去焦点时自动收起。
- **项目搜索**：索引用户选择的多个工作区，并结合匹配度与使用频率查找本地项目。
- **开发工具启动**：支持 `code`、`cursor`、`idea`、`webstorm`、`zed` 和 `open`，以结构化参数打开项目或文件。
- **上下文工作区**：命令需要目录时，可从已授权的工作区中选择；`cd` 用于切换应用内部的当前上下文。
- **友好内容展示**：`ls`、`ll` 和 `cat` 使用原生界面呈现目录与文本内容，而不是直接展示终端原始输出。
- **安全目录操作**：`mkdir` 和缺失项目目录的创建都先展示目标并等待确认，且只能发生在已配置的工作区内。
- **历史与偏好**：记录成功操作、提升常用项目排序，并支持单条历史删除和快捷键设置。
- **手动更新**：在设置中检查更新，查看版本说明并安装经过 Tauri 签名验证的更新包。

<!-- 在这里添加 Quick Command 功能演示视频或 GIF。 -->

## 安装

### Homebrew

当公开 Tap 中已经生成 Quick Command Cask 后，可以使用：

```bash
brew install --cask LIUeng/tap/quick-command
```

首次 Cask 会在包含 DMG 的公开 GitHub Release 发布后自动生成。如果 Homebrew 暂时找不到该 Cask，请改用下方的直接下载方式。

### GitHub Releases

1. 前往 [GitHub Releases](https://github.com/LIUeng/quick-command/releases)。
2. 下载最新版本的 macOS DMG。
3. 打开 DMG，并将 Quick Command 拖入“应用程序”目录。

当前版本尚未完成 Apple Developer ID 签名与公证。macOS 如果阻止首次打开，请前往“系统设置 → 隐私与安全性”，确认应用来源后选择“仍要打开”。无需也不建议全局关闭 Gatekeeper。

## 基本使用

先在设置中添加一个或多个工作区，然后通过全局快捷键唤起 Quick Command。

```text
code example
code x-pro/test01
ll
cat README.md
cd project
mkdir demo
```

- `code example` 会优先匹配已索引的项目；目标不存在时，可选择文件意图或确认创建项目目录。
- `code x-pro/test01` 支持在已授权工作区中预览并创建多级项目目录。
- `ls`、`ll` 和 `cat` 会在应用内显示结构化结果。
- `cd` 只更新 Quick Command 的目录上下文，不会启动子 Shell。
- `mkdir` 会在写入文件系统前显示完整目标并请求确认。

## 安全边界

Quick Command 不是通用终端，只接受内置可信命令目录中的命令。当前支持：

```text
code  cursor  idea  webstorm  zed  open  ls  ll  cat  cd  mkdir
```

应用不会把输入交给 `sh -c`、`bash -c` 或其他 Shell 求值，也不支持管道、重定向、`&&`、`;` 等任意 Shell 语法。外部程序始终通过“可执行文件 + 参数数组”的方式启动，文件系统创建操作必须位于用户授权的工作区中。

## 更新

打开“设置 → 软件更新”，点击“检查更新”。如果发现新版本，Quick Command 会显示版本与发布说明，并在用户确认后下载、验证、安装和重新启动。

应用内更新使用独立的 Tauri 更新签名验证包完整性。它与 macOS 的 Apple Developer ID 签名、公证是两套不同机制。

## 本地开发

环境要求：macOS、Rust、Node.js，以及 pnpm。

```bash
pnpm install
pnpm tauri dev
```

提交前运行：

```bash
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

## 项目文档

- [Agent 开发约束](AGENTS.md)
- [需求说明](docs/REQUIREMENTS.md)
- [技术架构](docs/ARCHITECTURE.md)
- [开发进度](docs/PROGRESS.md)
- [打包、更新与排查](docs/RELEASE.md)

## 平台状态

Quick Command 当前以 macOS 为首要平台。其他桌面平台将在核心交互与发布流程稳定后再评估。
