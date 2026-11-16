# Storage Drawers for Horizon

[中文](#中文) | [English](#english)

## 中文

这是 [Storage Drawers](https://github.com/jaquadro/StorageDrawers) 的非官方 Horizon 原生模组移植，参考 Java 1.21 版本。模组提供中文和英文资源，包含多种抽屉、压缩抽屉、升级、钥匙、管理器、自动化接口以及镶框桌和自定义外观。

该移植仍在完善，部分行为和兼容性尚需游戏内验证，不能视为与 Java 版完全一致。

### 安装

1. 下载 [Storage Drawers 0.2.7 模组文件](dist/storagedrawers-horizon-0.2.7.hnmod)。
2. 将文件放入 Horizon 游戏目录下的 `horizon-native-mods` 文件夹。
3. 在 Horizon 启动器中启用该模组并进行原生构建，然后启动游戏。

本版本适配 Horizon `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`；更换 Horizon 版本后可能需要重新适配。`.hnmod` 包含原生代码和资源，首次安装或更改模组组合时需要构建。

### 使用与配置

抽屉支持使用升级、钥匙和管理器；镶框桌可使用方块材料制作自定义外观。配置文件位于游戏目录下的 `horizon-mods/.horizon-mod-data/storagedrawers/features.json`。

### 开发

`src/` 包含库存与网络核心，`native/` 包含 Horizon 原生适配层，`content/` 包含游戏资源。`tools/build_bundle.py`、`tools/generate_hooks.py` 和 `tools/package_native.py` 分别用于生成资源、注入入口和模组包。脚本目前针对本机 Windows/F 盘布局编写；重新生成资源需要在 `F:/HorizonMods/references/StorageDrawers-1.21` 准备指定的 Java 源码，并准备对应的 Horizon 构建环境。换用其他路径时需要调整脚本。

## English

This is an unofficial native Horizon port of [Storage Drawers](https://github.com/jaquadro/StorageDrawers), based on the Java 1.21 version. It includes Chinese and English resources, several drawer types, compacting drawers, upgrades, keys, controllers, automation interfaces, a framing table, and customizable appearances.

The port is still being developed. Some behavior and compatibility require in-game verification; full parity with the Java version is not guaranteed.

### Installation

1. Download [Storage Drawers 0.2.7](dist/storagedrawers-horizon-0.2.7.hnmod).
2. Place the file in the `horizon-native-mods` folder in your Horizon game directory.
3. Enable the mod in the Horizon launcher, run a native build, and start the game.

This version targets Horizon revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`. Other Horizon revisions may require changes. The `.hnmod` contains native code and resources; a build is needed on initial installation or when changing the native mod set.

### Usage and configuration

Drawers support upgrades, keys, and controllers. The framing table uses block materials to create custom appearances. Configuration is stored at `horizon-mods/.horizon-mod-data/storagedrawers/features.json` in the game directory.

### Development

`src/` contains the storage and network core, `native/` the Horizon native adapter, and `content/` the game resources. `tools/build_bundle.py`, `tools/generate_hooks.py`, and `tools/package_native.py` generate resources, injection hooks, and the mod package, respectively. The scripts currently target a Windows/F: drive layout; regenerating resources requires the referenced Java source at `F:/HorizonMods/references/StorageDrawers-1.21` and the matching Horizon build environment. Adjust the scripts for other paths.

## Credits / 致谢

Original [Storage Drawers](https://github.com/jaquadro/StorageDrawers) by Justin Aquadro, MIT license; Java 1.21 reference commit `62a33ab1aa3e79e0c13e149f34fca4d30f07df8a`. See [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt). Horizon: [horizon-rust/horizon](https://github.com/horizon-rust/horizon). This project is not an official release of either project.

原版 Storage Drawers 由 Justin Aquadro 创作，采用 MIT 许可证，详见 [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt)。本项目不是 Storage Drawers 或 Horizon 的官方发行版本。
