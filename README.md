# Storage Drawers for Horizon

[中文](#中文) | [English](#english)

## 中文

这是 [Storage Drawers](https://github.com/jaquadro/StorageDrawers) 的非官方 [Horizon](https://github.com/horizon-rust/horizon) 原生模组移植，参考 Java 1.21 版本。模组提供中文和英文资源，包含多种抽屉、压缩抽屉、升级、钥匙、管理器、自动化接口以及镶框桌和自定义外观。

该移植仍在完善，部分行为和兼容性尚需游戏内验证，不能视为与 Java 版完全一致。

### 安装

1. 下载 [Storage Drawers 0.2.7 模组文件](dist/storagedrawers-horizon-0.2.7.hnmod)。
2. 将文件放入 Horizon 游戏目录下的 `horizon-native-mods` 文件夹。
3. 在 Horizon 启动器中启用该模组并进行原生构建，然后启动游戏。

本版本针对 Horizon `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` 构建；其他提交的兼容性未经验证，可能需要重新适配。`.hnmod` 包含原生代码和资源，首次安装或更改模组组合时需要构建。

### 使用与配置

抽屉支持使用升级、钥匙和管理器；镶框桌可使用方块材料制作自定义外观。配置文件位于游戏目录下的 `horizon-mods/.horizon-mod-data/storagedrawers/features.json`。

### 开发

`native/mixins/storagedrawers/recipe_engine/src/` 是库存与网络核心的唯一源码，客户端库和游戏适配层共享同一个 crate；`native/` 还包含 Horizon 原生适配层，`content/` 包含游戏资源。

重新生成资源需要准备下方指定提交的 Java 原版源码，并通过 `python tools/build_bundle.py --java-source <源码目录>` 指定路径；也可设置 `STORAGEDRAWERS_JAVA_SOURCE` 环境变量。未指定时默认使用项目同级的 `references/StorageDrawers-1.21`。随后使用 `python tools/generate_hooks.py` 和 `python tools/package_native.py` 生成注入入口和模组包。`tools/package_native.py --output-dir <目录>` 可指定输出位置。

打包脚本默认不会覆盖现有文件；开发时可指定新的输出目录。发布改动时应使用新版本号，避免替换已发布文件。

Horizon 源码和构建环境需要单独准备。可用 `python tools/check_horizon_revision.py <Horizon Git 仓库目录>` 检查源码提交是否与本模组目标一致；匹配提交也不代表已验证所有模组组合。`python tools/build_installed_mods.py --help` 列出安装目录和启动器路径参数。

已发布的 0.2.7 模组包对应的源码快照：[8687194](https://github.com/zhijiandd/horizon-storage-drawers/tree/868719441f6d14615027466970233907c36395e1)。

## English

This is an unofficial native [Horizon](https://github.com/horizon-rust/horizon) port of [Storage Drawers](https://github.com/jaquadro/StorageDrawers), based on the Java 1.21 version. It includes Chinese and English resources, several drawer types, compacting drawers, upgrades, keys, controllers, automation interfaces, a framing table, and customizable appearances.

The port is still being developed. Some behavior and compatibility require in-game verification; full parity with the Java version is not guaranteed.

### Installation

1. Download [Storage Drawers 0.2.7](dist/storagedrawers-horizon-0.2.7.hnmod).
2. Place the file in the `horizon-native-mods` folder in your Horizon game directory.
3. Enable the mod in the Horizon launcher, run a native build, and start the game.

This version was built for Horizon revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`. Compatibility with other revisions has not been verified and may require changes. The `.hnmod` contains native code and resources; a build is needed on initial installation or when changing the native mod set.

### Usage and configuration

Drawers support upgrades, keys, and controllers. The framing table uses block materials to create custom appearances. Configuration is stored at `horizon-mods/.horizon-mod-data/storagedrawers/features.json` in the game directory.

### Development

`native/mixins/storagedrawers/recipe_engine/src/` is the single source for the storage and network core, shared by the client library and game adapter. `native/` also contains the Horizon adapter, and `content/` contains game resources.

To regenerate resources, prepare a checkout of the Java source at the revision credited below and run `python tools/build_bundle.py --java-source <checkout>`, or set `STORAGEDRAWERS_JAVA_SOURCE`. If unspecified, the default is `references/StorageDrawers-1.21` next to the project. Then run `python tools/generate_hooks.py` and `python tools/package_native.py` to generate hooks and package the mod. Use `tools/package_native.py --output-dir <directory>` to select the destination.

The packaging script does not overwrite an existing package by default; use a new output directory during development. Publish changes under a new version instead of replacing an existing release.

Prepare the Horizon source and build environment separately. Run `python tools/check_horizon_revision.py <Horizon Git checkout>` to compare its revision with the mod target; a matching revision does not imply all mod combinations have been verified. See `python tools/build_installed_mods.py --help` for launcher and installation paths.

Source snapshot corresponding to the published 0.2.7 package: [8687194](https://github.com/zhijiandd/horizon-storage-drawers/tree/868719441f6d14615027466970233907c36395e1).

## Credits / 致谢

Original [Storage Drawers](https://github.com/jaquadro/StorageDrawers) by Justin Aquadro, MIT license; Java 1.21 reference commit `62a33ab1aa3e79e0c13e149f34fca4d30f07df8a`. See [LICENSE](LICENSE) and the original [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt). Horizon: [horizon-rust/horizon](https://github.com/horizon-rust/horizon). This project is not an official release of either project.

原版 Storage Drawers 由 Justin Aquadro 创作，采用 MIT 许可证，详见 [LICENSE](LICENSE) 和原版 [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt)。本项目不是 Storage Drawers 或 Horizon 的官方发行版本。
