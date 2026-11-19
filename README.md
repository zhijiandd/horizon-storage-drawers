# Storage Drawers for Horizon

[中文](#中文) | [English](#english)

## 中文

这是 [Storage Drawers](https://github.com/jaquadro/StorageDrawers) 的非官方 [Horizon](https://github.com/horizon-rust/horizon) 原生模组移植，参考 Java 1.21 版本。模组提供中文和英文资源，包含多种抽屉、压缩抽屉、升级、钥匙、管理器、自动化接口以及镶框桌和自定义外观。

该移植仍在完善，部分行为和兼容性尚需游戏内验证，不能视为与 Java 版完全一致。

### 安装

1. 下载 [储物抽屉 0.3.7 模组文件](dist/storagedrawers-horizon-0.3.7.hmod)。
2. 将文件放入 Horizon 游戏目录下的 `horizon-native-mods` 文件夹。
3. 在模组页启用储物抽屉，再用 Horizon 启动器 0.2.78 为实例进行原生构建。0.3.7 对应 2026 年 11 月 18 日的 Nightly `0b14f8a`（游戏 26.3）。

`.hmod` 包含原生代码和资源。首次安装或更改原生模组组合时，启动器使用预编译游戏，只构建模组部分。[旧版 0.2.8](dist/storagedrawers-horizon-0.2.8.hnmod) 对应 11 月 13 日的 Nightly `9f42064` 和旧式构建流程。

### 使用与配置

抽屉支持使用升级、钥匙和管理器；镶框桌可使用方块材料制作自定义外观。配置文件位于游戏目录下的 `horizon-mods/.horizon-mod-data/storagedrawers/features.json`。

新增配置和默认值见 [配置说明](CONFIGURATION.md)。

### 开发

`native/mixins/storagedrawers/recipe_engine/src/` 保存共享的库存与网络核心。`sdk/` 是新版 Horizon 的适配层，`native/` 保留旧版适配层，`content/` 包含游戏资源。

执行 `python tools/package_sdk.py --output dist/storagedrawers-horizon-0.3.7.hmod` 生成新版包。直接从源码构建时，先用 `python tools/stage_sdk.py --output <新目录>` 生成独立 SDK 项目，再将该目录传给启动器的 `native-build`。保留 `sdk/Cargo.lock` 中与目标 SDK 匹配的依赖版本。

重新生成资源需要准备下方指定提交的 Java 原版源码，并通过 `python tools/build_bundle.py --java-source <源码目录>` 指定路径；也可设置 `STORAGEDRAWERS_JAVA_SOURCE` 环境变量。未指定时默认使用项目同级的 `references/StorageDrawers-1.21`。随后使用 `python tools/generate_hooks.py` 和 `python tools/package_native.py` 生成注入入口和模组包。`tools/package_native.py --output-dir <目录>` 可指定输出位置。

`package_sdk.py` 会替换指定的输出文件。`package_native.py` 需要显式传入 `--overwrite` 才会覆盖现有包。发布改动时请使用新版本号。

Horizon 源码和构建环境需要单独准备。可用 `python tools/check_horizon_revision.py <Horizon Git 仓库目录>` 检查源码提交是否与本模组目标一致；匹配提交也不代表已验证所有模组组合。`python tools/build_installed_mods.py --help` 列出安装目录和启动器路径参数。

已发布的 0.2.7 模组包对应的源码快照：[8687194](https://github.com/zhijiandd/horizon-storage-drawers/tree/868719441f6d14615027466970233907c36395e1)。

## English

This is an unofficial native [Horizon](https://github.com/horizon-rust/horizon) port of [Storage Drawers](https://github.com/jaquadro/StorageDrawers), based on the Java 1.21 version. It includes Chinese and English resources, several drawer types, compacting drawers, upgrades, keys, controllers, automation interfaces, a framing table, and customizable appearances.

The port is still being developed. Some behavior and compatibility require in-game verification; full parity with the Java version is not guaranteed.

### Installation

1. Download [Storage Drawers 0.3.7](dist/storagedrawers-horizon-0.3.7.hmod).
2. Place the file in the `horizon-native-mods` folder in your Horizon game directory.
3. Enable Storage Drawers on the Mods page and build the instance's native mods using Horizon Launcher 0.2.78. This release targets the November 18, 2026 Nightly `0b14f8a` (game 26.3).

The `.hmod` contains native code and resources. Initial installation and changes to the native mod set require a build against the prebuilt game. [Legacy 0.2.8](dist/storagedrawers-horizon-0.2.8.hnmod) targets Nightly `9f42064` from November 13 and uses the older build process.

### Usage and configuration

Drawers support upgrades, keys, and controllers. The framing table uses block materials to create custom appearances. Configuration is stored at `horizon-mods/.horizon-mod-data/storagedrawers/features.json` in the game directory.

### Development

`native/mixins/storagedrawers/recipe_engine/src/` contains the shared storage and network core. The SDK adapter lives in `sdk/`, the legacy adapter in `native/`, and the game resources in `content/`.

Run `python tools/package_sdk.py --output dist/storagedrawers-horizon-0.3.7.hmod` to create the SDK package. To build from source, stage the project with `python tools/stage_sdk.py --output <new-folder>`, then pass that folder to the launcher's `native-build` command. Keep the SDK's dependency versions pinned in `sdk/Cargo.lock`.

To regenerate resources, prepare a checkout of the Java source at the revision credited below and run `python tools/build_bundle.py --java-source <checkout>`, or set `STORAGEDRAWERS_JAVA_SOURCE`. If unspecified, the default is `references/StorageDrawers-1.21` next to the project. Then run `python tools/generate_hooks.py` and `python tools/package_native.py` to generate hooks and package the mod. Use `tools/package_native.py --output-dir <directory>` to select the destination.

`package_sdk.py` replaces the specified output file. `package_native.py` requires `--overwrite` to replace an existing package. Use a new version for each release.

Prepare the Horizon source and build environment separately. Run `python tools/check_horizon_revision.py <Horizon Git checkout>` to compare its revision with the mod target; a matching revision does not imply all mod combinations have been verified. See `python tools/build_installed_mods.py --help` for launcher and installation paths.

Source snapshot corresponding to the published 0.2.7 package: [8687194](https://github.com/zhijiandd/horizon-storage-drawers/tree/868719441f6d14615027466970233907c36395e1).

## Credits / 致谢

Original [Storage Drawers](https://github.com/jaquadro/StorageDrawers) by Justin Aquadro, MIT license; Java 1.21 reference commit `62a33ab1aa3e79e0c13e149f34fca4d30f07df8a`. See [LICENSE](LICENSE) and the original [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt). Horizon: [horizon-rust/horizon](https://github.com/horizon-rust/horizon). This project is not an official release of either project.

原版 Storage Drawers 由 Justin Aquadro 创作，采用 MIT 许可证，详见 [LICENSE](LICENSE) 和原版 [LICENSE-StorageDrawers.txt](LICENSE-StorageDrawers.txt)。本项目不是 Storage Drawers 或 Horizon 的官方发行版本。
