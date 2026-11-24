# My-World-add-ons · 我的世界小工具

面向 Windows 的《我的世界》Java 版指令工具，使用 Rust 和 egui 编写。支持 **1.20.1** 与 **1.21.1**，生成的指令由你复制到游戏中使用，工具不会连接游戏或自动执行指令。

## 功能

- 查询原版物品 ID，扫描整合包 `mods` 目录中的中文语言文件。搜索结果可直接填入指令生成器。
- 生成无限元件、附魔装备、药水、刷怪蛋和自定义实体等指令。部分指令需要对应模组或 Forge。
- 自定义快捷指令、首页收藏和传送点；可保存成套的世界规则。
- 记录指令的生成版本，支持按版本选择历史记录并导出 `.mcfunction` 或数据包。
- 支持本机备份槽、恢复快照及 JSON 配置导入、导出。

更多操作说明见[使用说明](使用说明.txt)。游戏内执行指令需要开启作弊或具有相应权限。

## 从源码构建

需要 64 位 Windows、Rust 工具链及相应的 Windows 链接器。在仓库根目录执行：

```powershell
cargo test --locked
cargo build --release --locked
```

生成的程序位于 `target/release/minecraft_tool.exe`。仓库不提交编译产物、个人配置或导出的数据包。

## 许可证

见 [LICENSE](LICENSE)。本项目不是 Mojang 或 Microsoft 的官方工具。
