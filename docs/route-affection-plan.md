# 路线选择与好感度系统 — 深度分析报告

## 0. 核心发现

**`affection_change` 在 ASB 中不存在**。整个"好感度"系统通过 `local_work` 计数器实现。选择是原生 ASB 标签（`sel_init`/`sel_text`/`select`/`Select`/`exswitch`），而非 `calllua` 调用。

---

## 1. 全面 ASB 扫描统计

| 指标 | 数值 |
|------|------|
| ASB 文件总数 | 203 |
| 唯一 ASB 标签种类 | 103 |
| `calllua` 总数 | **16** (均为 UI 层函数，无剧情相关) |
| `affection_change` calllua | **0** |
| `tags.choice` calllua | **0** |
| `sel_init` (原生标签) | 17 |
| `sel_text` (原生标签) | 34 |
| `select` (原生标签) | 17 |
| `Select` (原生标签) | 16 |
| `exswitch` (分支标签) | 15 |
| 含选择的文件数 | 17 |

**关键结论**：ASB 中没有一个 `calllua` 与选择或好感度相关。16 个 calllua 全部是 UI 函数：`title_init`、`config_exit2`、`backlog_exit2`、`$t.file` 等。

---

## 2. 选择系统真实架构

### 2.1 ASB 中的选择流程（原生标签）

```
sel_init     {"name": "SelectItemXXXXXX"}    // 初始化选择（创建命名选择）
sel_text × N {"label": "SelectItemXXXXXX",   // 注册选项（可重复 N 次）
              "text": "显示文本",
              "exp": "t.ens:0"}              // exp = 选项对应的脚本变量值
select       {}                               // 渲染 UI 并等待玩家选择
                                              // → 玩家选择 → 引擎设 t.tmp = exp 值
[block: SelectItemXXXXXX]
Select       {"0": "2", "1": "SelectItemXXXXXX"}  // 选择完成标记
exswitch     {"data": "t.tmp<>0:target_A<>1:target_B<>default:target_C"} // 按值分支
```

### 2.2 `exswitch` 分支机制

`exswitch` 读取 `t.tmp`（即玩家选择的 exp 值），跳转到不同目标：
- `t.tmp` = 0 → 目标 A（如 `label001_0`）
- `t.tmp` = 1 → 目标 B（如 `label001_1`）
- default → 目标 C

目标可以是同一块内的标签，也可以是独立块。这取决于 ASB 文件的组织方式。

### 2.3 两种选择模式

#### 模式 A：完全分支（如 aiy00060）
```
sel_init → sel_text×2 → select
[block: SelectItem0000601]
  Select → exswitch
[block: label001_0]  // 选项 0 的分支
  对话 A
  jump label001
[block: label001_1]  // 选项 1 的分支
  对话 B
  jump label001
[block: label001]    // 汇合点
```

#### 模式 B：好感度累加（如 aiy10050）
```
StoreValueToLocalWork {"0": "1", "1": "0"}   // work[1] = 0
sel_init → sel_text×2 → select
[block: SelectItem0100501]
  Select → exswitch
  jump label001                                // 选项 0: 跳过累加
  LoadValueFromLocalWork {"0": "1"}            // 选项 1: t.tmp = work[1]
  StoreValueToLocalWork {"0": "1", "1": "t.tmp+1"} // work[1] = work[1] + 1
  jump label001
```

### 2.4 选项 exp 值系统

| 选项 | exp 属性 | `t.ens` 值 | `t.tmp` 值 | 含义 |
|------|----------|------------|------------|------|
| 选项 A | `"t.ens:0"` | 0 | 0 | 不累加好感 |
| 选项 B | `"t.ens:1"` | 1 | 1 | 累加好感 |

`exp` 格式：`t.ens:<value>` — 选择后引擎设 `t.ens` 为对应值，后续命令通过 `t.tmp` 读取。

---

## 3. 好感度系统真实架构

### 3.1 好感度 = local_work 计数器

**无独立的 `AffectionChange` 命令**。好感度通过 `StoreValueToLocalWork` / `LoadValueFromLocalWork` 实现：

| 命令 | 作用 |
|------|------|
| `StoreValueToLocalWork {"0": "N", "1": "value"}` | 设 `local_work[N] = value` |
| `LoadValueFromLocalWork {"0": "N"}` | 读 `local_work[N] → flags["tmp"]` |

好感度选择后累加模式：
```
LoadValueFromLocalWork {"0": "1"}              // t.tmp = work[1] (当前好感)
StoreValueToLocalWork {"0": "1", "1": "t.tmp+1"}  // work[1] = old + 1
```

### 3.2 local_work 索引 → 女主角映射

| local_work 索引 | 女主角 | 对应 ASB 文件 |
|-----------------|--------|--------------|
| 1 | 菲奥奈 (Fione) | aiy10050, aiy10140, aiy10170 |
| 2 | 艾莉斯 (Eris) | aiy20140, aiy20200, aiy20210 |
| 3 | 柯蕾特 (Colette) | (推断) |
| 4 | 莉西亚 (Lysia) | aiy40110, aiy40150, aiy40290 |
| 5 | 拉薇 (Lavi) | (推断) |
| 9 | 临时决策值 | aiy10190, aiy20210 |

### 3.3 main.iet 对好感度的消费

`root/scenario/main.iet` 通过 `LoadValueFromLocalWork` + `Condition` 读取这些计数器：

```
[LoadValueFromLocalWork 9]   // 读 temp 决策值
[if estimate="$t.tmp == 0"]
  [CallScript aiy10230]      // → Fione 家族路线
[else]
  [CallScript aiy10200]      // → Fione 主线
```

每条路线末尾：
```
[CallScript aiyXXXXX]        // 执行路线剧本
[TerminateExecutionOfScript] // 脚本终止，控制回归 main.iet
```

### 3.4 路线解锁 (route.lua)

`root/system/ui/route.lua` 检查 global flags 51-55：
- flag 51 → 菲奥奈路线解锁
- flag 52 → 艾莉斯路线解锁
- flag 53 → 柯蕾特路线解锁
- flag 54 → 拉薇路线解锁
- flag 55 → 莉西亚路线解锁

这些 flag 在 ASB 中通过 `SetGlobalFlag` 设置（共 68 次 SetGlobalFlag 调用）。

---

## 4. 当前 Mapper 缺失清单

### 4.1 已知缺失标签

| ASB 标签 | 出现次数 | mapper 状态 | Runner 支持 |
|----------|---------|------------|------------|
| `sel_init` | 17 | ❌ 无处理 | — |
| `sel_text` | 34 | ❌ 无处理 | — |
| `select` | 17 | ❌ 无处理 | — |
| `Select` | 16 | ❌ 无处理 | — |
| `exswitch` | 15 | ❌ 无处理 | — |
| `SetLocalFlag` | 13 | ❌ 无处理 | ✅ (runner 602行) |
| `GetLocalFlag` | 34 | ❌ 无处理 | ✅ (runner 603行) |
| `SetJumpLabel` | 61 | ❌ 无处理 | ✅ |
| `LoadValueFromLocalWork` | 13 | ❌ 无处理 | ✅ |
| `StoreValueToLocalWork` | 15 | ❌ 无处理 | ✅ |
| `GetGlobalFlag` | 6 | ❌ 显式跳过 | ✅ |
| `SetGlobalFlag` | 68 | ✅ | ✅ |
| `RouteFlag` | 23 | ✅ | ✅ (硬编码 103-167) |
| `Ending_Base` | 6 | ❌ | ❌ 无此命令 |
| `NextDay` | 71 | ❌ | ❌ 无此命令 |
| `View` | 45 | ❌ 无处理 | ✅ (runner 637行) |
| `ViewEnd` | 36 | ❌ | ✅ |
| `SetValidityOfLoading` | 26 | ❌ | ❌ |
| `SetValidityOfSaving` | 26 | ❌ | ❌ |
| `SetValidityOfInput` | 7 | ❌ | ❌ |
| `Size` | 206 | ❌ | ❌ |
| `Window` | 1190 | ❌ 无处理 | ✅ |
| `Back` | 1435 | ❌ | ❌ (背景切换) |
| `Blackout` | 144 | ❌ | ❌ |
| `WhiteoutBySA` | 105 | ❌ | ❌ |
| `Hcg` | 300 | ❌ | ❌ |
| `Refresh` | 31407 | ❌ 被忽略 | ✅ (text refresh) |
| `Jishin` | 9 | ❌ | ❌ |
| `exif` | 79 | ❌ | ❌ |
| `sys_trans` | 17 | ❌ | ❌ |
| `lyc2` | 5 | ❌ | ❌ |

### 4.2 伪缺失标签（被 mapper 忽略但 Runner 支持）

| 标签 | Runner 支持 | Mapper 状态 |
|------|------------|------------|
| `Condition` | ✅ (284-313行) | ❌ mapper 从不生成 |
| `AffectionCondition` | ✅ (338-355行) | ❌ mapper 从不生成 |
| `SetFlag` | ✅ (315-316行) | ❌ mapper 从不生成 |
| `GetLocalFlag` | ✅ (325-327行) | ❌ mapper 从不生成 |
| `StoreValueToLocalWork` | ✅ (318-319行) | ❌ mapper 从不生成 |
| `LoadValueFromLocalWork` | ✅ (321-323行) | ❌ mapper 从不生成 |
| `SetJumpLabel` | ✅ | ❌ mapper 从不生成 |
| `Halt` | ✅ (329-334行) | ✅ |
| `SavePoint` | ✅ (357行) | ✅ (作为 calllua save_point) |
| `UnlockCg` | ✅ (358-360行) | ✅ (作为 calllua unlock_cg) |

---

## 5. 当前 `choice` mapper 实现分析

### 5.1 mapper.rs 中有关选择的代码 (476-483)

```rust
s if s.contains("choice") || s.contains("tags.choice") => {
    Some(vec![ScriptCmd::Choice {
        options: vec![ChoiceOption {
            text: format!("[choice in {}]", func),
            affection_change: None,
            goto: None,
        }],
    }])
}
```

**问题**：
1. 匹配的是 `calllua` 中的函数名 → 但 ASB 从不用 calllua 做选择
2. 只生成一个占位选项 — 没有实际文本、好感变化或跳转目标
3. `affection_change` 和 `goto` 总是 None

### 5.2 正确实现方案

正确的 mapper 需要在 `map_command` 中添加：

```rust
// map_script 中需要的状态
let mut pending_choice_opts: Vec<ChoiceOption> = Vec::new();

// map_command 中:
"sel_init" => {
    pending_choice_opts.clear(); // 重置选项缓冲区
    None // 无直接输出
}
"sel_text" => {
    let text = cmd.attrs.get("text").cloned().unwrap_or_default();
    // exp 值转换为好感变化
    let affection_change = parse_exp_to_affection(&cmd.attrs);
    // TODO: 确定 goto 目标（通过 exp 值的 exswitch 分析）
    pending_choice_opts.push(ChoiceOption {
        text,
        affection_change,
        goto: None, // 需要跨命令分析 exswitch
    });
    None
}
"select" => {
    let options = std::mem::take(&mut pending_choice_opts);
    Some(vec![ScriptCmd::Choice { options }])
}
"Select" => {
    // 选择完成标记，无直接输出
    None
}
"exswitch" => {
    // 需要分析 data 属性并将分支映射到 choice 的 goto
    None
}
```

但 `exswitch` 在 `select` 之后才出现（在 `Select` 块中），所以不能简单地将 `goto` 映射到选择选项。需要后处理或更复杂的跨命令状态。

---

## 6. `exswitch` 分支深度分析

### 6.1 格式解析

```
exswitch {"data": "t.tmp<>0:label001_0<>1:label001_1<>default:label001"}
```

格式：`<variable><>value1:target1<>value2:target2<>default:default_target`

- `<>` 是大小写分隔符
- `:` 分隔值和目标
- `default` 是默认分支关键字

### 6.2 在端口中的处理

对于 Bevy 端口，选项 A：**不实现 exswitch**。改为将选择建模为显式的 `ChoiceOption { text, goto, affection_change }`：

```
Choice {
    options: [
        ChoiceOption {
            text: "选项 A",
            affection_change: None,      // 如果 exp:t.ens:0
            goto: Some("label001"),      // 汇合点
        },
        ChoiceOption {
            text: "选项 B",
            affection_change: Some(("01", 1)),  // 如果 exp:t.ens:1 且后有 StoreValueToLocalWork
            goto: Some("label001"),
        },
    ],
}
```

但这需要将 `exswitch` 分支结构与选择选项关联——跨 ASB 块分析，在 mapper 中需要更复杂的处理。

---

## 7. 当前运行时的完整用法映射

### 7.1 `ScriptEngine` 状态模型

```rust
pub struct ScriptEngine {
    pub flags: HashMap<String, i32>,        // t.ens, t.tmp, 等
    pub local_work: HashMap<i32, i32>,      // work[N]
    pub local_flags: HashMap<String, i32>,  // local flags
    pub global_flags: HashMap<i32, i32>,    // global flags
    pub current_line: usize,
    pub call_stack: Vec<ScriptCallFrame>,
    pub finished: bool,
}
```

### 7.2 变量交互

| 引擎变量 | 设置方式 | 读取方式 | ASB 对应 |
|---------|---------|---------|---------|
| `flags["tmp"]` | `LoadValueFromLocalWork` | `StoreValueToLocalWork` 中的 `t.tmp` | 好感度累加中间值 |
| `flags["ens"]` | 选择系统 | `exswitch` 前的 `t.ens` | 选择 exp 值 |
| `local_work[N]` | `StoreValueToLocalWork` | `LoadValueFromLocalWork` | 好感度累计器 |
| `local_flags[X]` | `SetLocalFlag` | `GetLocalFlag` | 章节内分支标志 |
| `global_flags[N]` | `SetGlobalFlag` | `GetGlobalFlag` / `RouteFlag` | 路线解锁、CG 解锁 |

---

## 8. 具体实施路线（修订版）

### Phase 1: 选择数据提取（2-3 天）

```
1. map_command 添加 sel_init/sel_text/select/Select/exswitch 映射
2. map_script 添加 cross-command 状态 pending_choice_opts: Vec<ChoiceOption>
3. 从 sel_text 提取 text 字段 → ChoiceOption.text
4. 从 exp 字段映射到 affection_change（需判断 t.ens:N 含义）
5. exswitch 后处理：分析分支目标，映射到 ChoiceOption.goto
6. 重新生成所有 .bscript.ron，验证 choice 数据
```

### Phase 2: local_work 好感度系统（1-2 天）

```
1. StoreValueToLocalWork/LoadValueFromLocalWork mapper 补全
2. SetLocalFlag/GetLocalFlag mapper 补全
3. 验证好感累加模式（Index 1-5 → 女主角映射）
4. 存档扩展：local_work 完整存取
```

### Phase 3: 路线选择系统（2-3 天）

```
1. 通用化 RouteFlag（从配置读取 flag 索引）
2. 添加 RouteConfig 资源 + assets/routes.ron
3. 路线解锁界面（route.lua 端口）
4. 路线完成跟踪（UnlockState.lib.routes_cleared）
```

### Phase 4: 路线分支与结尾（2-3 天）

```
1. main.iet 条件分支验证（LoadValueFromLocalWork + Condition）
2. TerminateExecutionOfScript 处理 → 路线结束标记
3. Ending ScriptCmd 添加（可选，看需求）
4. 完整流程测试：标题 → 所有路线
```

### Phase 5: 剩余命令补全（1-2 天）

```
1. Back/Blackout/WhiteoutBySA/Hcg 等视觉命令
2. View/ViewEnd 角色视角
3. SetValidityOfLoading/Saving/Input 系统命令
4. Size/Window 布局命令
5. NextDay 时间推进
```

---

## 9. 关键决策

### Q1: 选择数据提取 vs 手写？

**结论**：从 ASB 提取。17 个选择文件共 34 个 `sel_text`，提取工作明确且完整。只需实现跨命令状态。

### Q2: `exswitch` 分支如何处理？

**结论**：后处理分析。mapper 需要：
1. 收集所有的 `exswitch` + 后续块
2. 分析分支模式 → 生成带 goto 的 ChoiceOption
3. 对无法自动映射的分支标记为 TODO

### Q3: `local_work` vs `AffectionMap` 资源？

**结论**：两者共存。`local_work` 是原始引擎的好感度存储。`AffectionMap` 保留了但映射自 `local_work[1-5]`。存档需要同时保存。

### Q4: `Ending` ScriptCmd 是否必要？

**结论**：当前 `TerminateExecutionOfScript` + `return_main` 组合已处理路线结束。添加 `Ending` 可选但不是关键路径。优先完成 Phase 1-3。

### Q5: `NextDay` 如何处理？

**结论**：71 个 `NextDay` 调用需要在脚本引擎中添加处理（简单的时间推进 + 保存点）。当前可以忽略（跳过）但不理想。

---

## 10. 数据文件汇总

### 10.1 关键 ASB 文件

| 文件 | 特点 | 关键内容 |
|------|------|---------|
| `aiy00060` | 模式 A 选择（完全分支） | 2 选项 → exswitch → 独立对话分支 |
| `aiy00140` | 模式 A 选择（完全分支） | 缇娅选择 |
| `aiy10050` | 模式 B 选择（好感累加） | Fione 路线选择 → work[1]++ |
| `aiy10190` | 好感 + temp 模式 | work[1] 累加 + work[9] 决策值 |
| `aiy20210` | 多变量操作 | work[2] + work[9] + SetLocalFlag 202 |
| `aiy40290` | 增量 +2 | work[04] ← t.tmp+2 |
| `aiy30220` | 路线解锁 | SetGlobalFlag 3 + 选择 → Colette 路线 |
| `aiy20240` | 路线终止 | TerminateExecutionOfScript + SetGlobalFlag 206 |

### 10.2 女主角与标志映射

| 女主角 | local_work 索引 | route.lua flag | 路线脚本 |
|--------|----------------|----------------|---------|
| 菲奥奈 (Fione) | 1 | 51 | aiy10010 等 |
| 艾莉斯 (Eris) | 2 | 52 | aiy20010 等 |
| 柯蕾特 (Colette) | 3 | 53 | aiy30010 等 |
| 莉西亚 (Lysia) | 4 | 54 | aiy40010 等 |
| 拉薇 (Lavi) | 5 | 55 | aiy50010 等 |

### 10.3 重要但不紧急的命令

| 命令 | 出现次数 | 用途 | 优先级 |
|------|---------|------|--------|
| `NextDay` | 71 | 时间推进 | LOW |
| `Back` | 1435 | 背景切换 | LOW |
| `Blackout` | 144 | 黑场 | MEDIUM |
| `WhiteoutBySA` | 105 | 白场 | MEDIUM |
| `Hcg` | 300 | H 场景 | LOW (非关键路径) |
| `Refresh` | 31407 | 画面刷新 | 已忽略 (文本刷新) |
| `View` | 45 | 角色视角切换 | MEDIUM |
| `SetValidityOfLoading` | 26 | 存档可用性 | LOW |
| `SetValidityOfSaving` | 26 | 读档可用性 | LOW |
