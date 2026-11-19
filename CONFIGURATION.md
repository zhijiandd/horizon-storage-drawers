# 储物抽屉配置

配置文件：游戏目录下 `horizon-mods/.horizon-mod-data/storagedrawers/features.json`。
修改后重启游戏生效。0.2.8 会补写新增配置的默认值，保留已有设置；JSON 格式错误时不会覆盖原文件。

这些是本次补齐的选项，并不代表 Java 的全部配置界面和第三方集成都已移植。

| 配置键 | 默认值 | 作用 |
| --- | --- | --- |
| `store_denylist` | `["storagedrawers:creative_vending_upgrade"]` | 禁止存入的物品 ID，或整个命名空间，如 `ae2`。作用于手动、GUI 和自动化入口，不删除已有库存 |
| `base_stack_storage` | `1` | 基础存储单位倍率 |
| `full_drawer_units` | `[32,16,8]` | 普通全深抽屉的单槽基础堆数，依次为单槽、双槽、四槽 |
| `half_drawer_units` | `[16,8,4]` | 普通半深抽屉的单槽基础堆数 |
| `compacting_units` / `half_compacting_units` | `32` / `16` | 压缩抽屉最紧凑物品视图的基础堆数 |
| `detached_units` | `32` | 新拆卸抽屉和扩容配方的基础堆数；已有拆卸物品保留保存的容量 |
| `storage_multipliers` | `[2,4,8,16,64,256,2048]` | 黑曜石至下界合金的升级倍率；安装多个升级时相加 |
| `compacting_enabled` | `true` | 是否为新初始化的压缩抽屉查找压缩关系 |
| `extra_compacting_rules` | `true` | Java 默认额外规则，包括黏土、雪、萤石、砖、下界砖、下界疣、石英、西瓜、竹 |
| `compacting_rules` | `[]` | 自定义规则，格式见下方；优先于动态配方查找 |
| `magnet_ranges` | `[[1,1,0],[4,2,0],[8,3,0]]` | 三个磁铁等级的 `[水平,向上,向下]` 范围 |
| `magnet_additive_range` | `true` | 多个磁铁的范围组合方式，按参考 Java 源码计算 |
| `magnet_max_range` | `[24,8,0]` | 组合范围上限。要吸下方物品，须同时调整向下范围和上限 |
| `magnet_active_ticks` | `[20,10,5]` | 三等级成功吸入后的检查间隔，单位 tick |
| `magnet_idle_ticks` | `[20,20,20]` | 无物品吸入时的间隔；另外增加 0–4 tick 的错峰 |
| `drop_mode` | `"KEEP"` | `KEEP`：库存随抽屉保存；`DROP`：库存和升级散落；`VOID`：清除库存和升级 |
| `drop_stack_limit` | `32` | `DROP` 模式最多散落的库存堆数，超过部分清除；升级不计入此限制 |
| `label_render_distance` | `20.0` | 正面物品标签的显示距离；`0` 不额外限制 |
| `quantity_render_distance` | `10.0` | 数量文字的显示距离；`0` 不额外限制 |
| `quantity_fade_distance` | `20.0` | 按参考 Java 源码的阈值和公式处理数量透明度 |
| `illumination_level` | `13` | 装照明升级时标签的最低方块光照，范围 0–15 |
| `minimum_illumination_level` | `1` | 未装照明升级时标签的最低光照 |
| `framed_translucent_materials` | `true` | 非不透明镶框材料是否使用半透明渲染通道 |

自定义压缩示例（物品必须已注册）：

```json
{
  "compacting_rules": ["example:compressed_ore, example:ore, 9"]
}
```

自动发现只接受可逆的 2×2／3×3 合成关系，且不接受带容器返还的配方，避免消耗和返还造成复制。
明确配置的额外规则可支持没有原生逆向配方的关系。无压缩配方的物品保留一个有效存储视图。
已有压缩抽屉保留其保存的换算关系，不会在升级或配方重载时偷偷改换已有物品。

降低容量配置不会清除旧库存，也不会使其无法读档；超容量时停止继续存入，仍可取出。
`DROP` 与 `VOID` 是主动选择会清除库存的模式，请按需要设置；默认始终是 `KEEP`。
