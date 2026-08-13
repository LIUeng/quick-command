---
description: 这是一个快速执行用户输入命令的桌面客户端应用
---

# Quick Command / 快速执行命令

使用全局快捷键唤起，通过短命令快速匹配并打开本地项目。

> 当前状态：MVP 开发中。详细任务进度见 [`docs/PROGRESS.md`](docs/PROGRESS.md)。

## 背景

> 目前执行命令的过程

比如

新建一个项目，需要用 VSCode 或者其他已安装的命令打开项目，进行项目开发

- 打开终端
- 进入某个文件夹 `cd project`
- `mkdir example`
- `cd example`
- `code .`

等等其他类似的命令操作，比较繁琐

## 目标

做成一个桌面端应用，使用快键键打开，直接快速输入，类似 MacOS 的聚焦搜索

### 主要功能点

当输入

```bash
code example
```

时，可以直接打开项目

- 项目地址不是绝对路径，像 autojump 插件一样，可以快速匹配历史权重的地址
- 如果没有可以提示是否直接创建该目录并打开

### 其他功能点

- 保留历史记录（前30条），方便快速打开
- 设置
  - 快捷键
- 不仅仅是列举的 `code` 全局命令，还有其他类似的操作都可以

## 开发

### 技术

使用 Rust + Tauri + React + TailwindCSS

### 注意事项

- 界面开发，安装依赖使用 `pnpm`

### 本地运行

```bash
pnpm install
pnpm tauri dev
```

### 开发文档

- Agent 开发约束：[`AGENTS.md`](AGENTS.md)
- 完整需求：[`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md)
- 技术架构：[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- 任务进度：[`docs/PROGRESS.md`](docs/PROGRESS.md)
