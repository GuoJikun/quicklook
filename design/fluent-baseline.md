# Fluent 2 视觉基准（颜色 · 字体 · 焦点）

> 适用范围：`design/preview-ui-design.md`、`design/settings-ui-design.md`、`design/audio-preview-design.md` 及其原型
> 目标代码：`src/assets/var.css`、`src/assets/var/layout.css`、`src/assets/base.css`
> 版本基线：v1.6.0（§7.2 补标题栏 `window` / `dialog` 两种变体与「对话框不叠第二层标题」：对话框把标题收进 44px 标题行，版本信息归内容区；§7「标题」行同步改为对话框标题并进标题行。v1.5.0 §7.2 新增「标题栏由前端自绘」：三窗口统一 `decorations(false)`，标题栏随 `html.dark` 明暗，拖动与边缘缩放需自补；v1.4.0 §3.3 行高改为按字号档位绑定绝对值并废除统一系数、§8 字重验收与 §3.4 对齐、补录按钮五态与 hover/danger-hover/选区 token、新增 `--badge-h`、收录 `--font-4xl` 与 `--color-close-hover`；v1.3.0 新增 `--color-stroke-control` 与 §7「窗口形态」，更新窗口改为 ContentDialog 结构；v1.2.0 图标来源收敛为 `@vicons/fluent`，新增 `--radius-xs` / `--radius-pill` / `--icon-xs` / `--icon-xl` / `--line-reading` / `--line-code`，清除字面量圆角与行高；v1.1.1 增补三级文字 `--color-text-tertiary`、按下态 `--elevation-pressed`、控件与图标尺寸阶梯；v1.1.0 引入品牌色拆分与字号收敛）
>
> **本文件是三份设计稿的取值来源。** 三份稿件中凡与本文件冲突的字号、色值，以本文件为准。
> 稿件的布局、间距、控件规格不变。

## 1. 为什么需要这份基准

v1.0.0 的视觉语言来自 Fluent 2（透明标题行、悬浮控件、单一底色），但色值与字号取自更早的 Fluent 1（UWP）习惯，且字号是逐处手写累积出来的。实测结果：

- 品牌蓝 `#0078d4` 同时承担「按钮底色」与「文字/图标色」两种职责，后者在深色模式下只有 **3.13:1**；放在品牌浅底上进一步降到 **2.60:1**。
- 字号体系为 12/13/14/15/16/18/24，其中 **13、15、18 不在 Fluent 2 阶梯内**；另有 49 处字面字号绕过变量，含 `12.5px`、`11.5px`、`10.5px` 等半像素值。

本文件把颜色拆成「前景 / 背景」两组语义 token，把字号收敛到 Fluent 2 的 Base 阶梯，并把行高与字号成对绑定。

## 2. 颜色

### 2.1 语义分层（核心变更）

Fluent 2 的品牌色不是一个值，而是按「承载什么」分成两组：

| 组 | 用途 | 浅色 | 深色 | Fluent 2 token |
| --- | --- | --- | --- | --- |
| **前景** `--color-brand-fg` | 文字、图标、描边、焦点环、指示条 | `#0F6CBD` | `#479EF5` | `BrandForeground1` |
| **背景** `--color-accent` | 实心底（其上必为白字）、开关轨道 | `#0078D4` | `#0078D4` | `BrandBackground` |

**规则：`--color-accent` 不得直接用作 `color`。** 需要品牌色文字时一律用 `--color-brand-fg`。

原因：`#0078D4` 在浅色白底上 4.53:1（刚好压线），在深色 `#2b2b2b` 上只有 3.13:1；`#0F6CBD` 在白底为 5.38:1，`#479EF5` 在 `#2b2b2b` 上为 5.04:1。深色不采用 Fluent 2 的 `#115EA3` 作按钮底，是因为 `#0078D4` 叠白字已达标（4.53:1）且为既有品牌值，改动无收益。

### 2.2 完整 token 表

| 变量 | 浅色 | 深色 | Fluent 2 对应 | 用途 |
| --- | --- | --- | --- | --- |
| `--color-bg` | `#f3f3f3` | `#1c1c1c` | NeutralBackground3 | 窗口底 |
| `--color-surface` | `#ffffff` | `#2b2b2b` | NeutralBackground1 | 卡片、浮层 |
| `--color-text-primary` | `#1a1a1a` | `#ffffff` | NeutralForeground1 | 主文字 |
| `--color-text-secondary` | `#555555` | `#cccccc` | NeutralForeground2 | 次要文字 |
| `--color-text-tertiary` | `#616161` | `#adadad` | NeutralForeground3 | 三级文字：元信息、说明、空态、三级目录项 |
| `--color-text-disabled` | `#999999` | `#777777` | NeutralForegroundDisabled | **仅**用于真正禁用的控件 |
| `--color-brand-fg` | `#0F6CBD` | `#479EF5` | BrandForeground1 | **品牌前景** |
| `--color-accent` | `#0078D4` | `#0078D4` | BrandBackground | **品牌背景** |
| `--color-accent-soft` | `rgba(15,108,189,.10)` | `rgba(71,158,245,.14)` | BrandBackground2 | 轻提示底（不承载品牌色文字） |
| `--color-selected-bg` | `#EBEBEB` | `#383838` | NeutralBackground1Selected | **导航激活 / 列表选中**（配品牌指示条） |
| `--color-selected-strong` | `#CFCFCF` | `#4A4A4A` | NeutralBackground1Pressed 近似 | **开关按钮的「已开启」**，必须比 hover 深一档 |
| `--color-danger-fg` | `#A80000` | `#FF99A4` | StatusDangerForeground | 危险色文字 |
| `--color-danger` | `#D13438` | `#D13438` | StatusDangerBackground3 | 危险色底（极少用） |
| `--color-success-fg` | `#0E700E` | `#54B054` | StatusSuccessForeground | 成功态**前景**（图标/文字），不做实心底 |
| `--color-border` | `#e1e1e1` | `#3a3a3a` | NeutralStroke2 | 容器边界 |
| `--color-stroke-control` | `#d1d1d1` | `#666666` | NeutralStroke1 | **控件描边**：带边框的标准按钮 |
| `--color-divider` | `rgba(0,0,0,.07)` | `rgba(255,255,255,.08)` | NeutralStroke3 | 发丝线 |
| `--color-chip-bg` | `rgba(0,0,0,.05)` | `rgba(255,255,255,.06)` | NeutralBackground4 | 胶囊底 |
| `--elevation-hover` | `rgba(0,0,0,.08)` | `rgba(255,255,255,.05)` | SubtleBackgroundHover | 悬浮态 |
| `--elevation-pressed` | `rgba(0,0,0,.12)` | `rgba(255,255,255,.09)` | SubtleBackgroundPressed | 按下态，必须比 hover 深/亮一档 |
| `--color-selection-bg` | `#e5f1fb` | `rgba(0,120,212,.20)` | BrandBackground2 近似 | **数据网格选区**（Excel 单元格范围），见 §2.4 |
| `--color-brand-fg-hover` | `#115ea3` | `#62abf5` | BrandForeground1Hover | 品牌前景的 hover（链接、品牌图标按钮） |
| `--color-danger-hover` | `#b52b2f` | `#e8646c` | StatusDangerBackground3Hover | 危险按钮 hover（其上仍为白字） |
| `--color-close-hover` | `#e81123` | `#e81123` | Windows 标题栏惯例 | **仅**窗口关闭钮 hover 底（其上为白字） |

`--color-accent-hi` 已删除，其全部引用改为 `--color-brand-fg`。
`--color-hover-bg` 已更名为 `--color-selection-bg`——它承载的是「选区」不是「悬浮」，旧名会把人引向 §2.5 的 hover 配方。

按钮五态独立成组（主按钮取 Fluent 2 `BrandBackground1Hover/Pressed`：`#106ebe` / `#005a9e`）：

| 变量组 | 用途 |
| --- | --- |
| `--btn-primary-bg` / `--btn-primary-text` | 实心主按钮：底 `= --color-accent`，文字白（4.53:1） |
| `--btn-primary-hover-bg` / `--btn-primary-active-bg` | 主按钮 hover / 按下 |
| `--btn-primary-disabled-bg` / `--btn-primary-disabled-text` | 主按钮禁用（豁免 WCAG，但深色下仍须可见） |
| `--btn-secondary-bg` / `--btn-secondary-border` / `--btn-secondary-text` | 描边次按钮：透明底 + `--color-brand-fg` 描边与文字 |
| `--btn-secondary-hover-bg` / `--btn-secondary-active-bg` | 次按钮 hover / 按下（品牌浅底，其上只放品牌前景） |
| `--btn-secondary-disabled-bg` / `--btn-secondary-disabled-text` / `--btn-secondary-disabled-border` | 次按钮禁用 |

### 2.3 对比度验收矩阵

WCAG 2.1 AA：正文 ≥ 4.5:1，图形/大字 ≥ 3:1。下列为**必须全部满足**的组合。

| 组合 | 浅色 | 深色 | 要求 |
| --- | --- | --- | --- |
| primary on surface | 17.40:1 ✅ | 14.16:1 ✅ | 4.5 |
| secondary on surface | 7.46:1 ✅ | 8.82:1 ✅ | 4.5 |
| **tertiary on surface** | **6.19:1** ✅ | **6.31:1** ✅ | 4.5 |
| **tertiary on bg** | **5.58:1** ✅ | **7.59:1** ✅ | 4.5 |
| **brand-fg on surface** | **5.38:1** ✅ | **5.04:1** ✅ | 4.5 |
| **danger-fg on surface** | **7.88:1** ✅ | **6.97:1** ✅ | 4.5 |
| **success-fg on surface** | **6.28:1** ✅ | **5.20:1** ✅ | 4.5 |
| white on accent（按钮底） | 4.53:1 ✅ | 4.53:1 ✅ | 4.5 |
| primary on accent-soft | 15.10:1 ✅ | 11.43:1 ✅ | 4.5 |
| secondary on accent-soft | 6.47:1 ✅ | 7.12:1 ✅ | 4.5 |
| brand-fg 作焦点环 on surface | 5.38:1 ✅ | 5.04:1 ✅ | 3.0 |
| brand-fg 作指示条 on selected-bg | 4.52:1 ✅ | 4.18:1 ✅ | 3.0 |
| disabled（豁免 WCAG） | 2.85:1 ➖ | 3.16:1 ➖ | — |

两点必须注意，否则会把合格配方用错：

1. **`accent-soft` 上只能放 `text-primary` / `text-secondary`，不能放 `brand-fg`**。品牌色文字在浅底上会跌到 4.07:1（深色）。InfoBar 因此用中性文字 + 品牌色图标。
2. **`selected-bg` 上同理不放品牌色文字**。品牌色只出现在 3px 指示条上，按图形 3:1 判定（实测 4.52 / 4.18，合格）。若在选中行用品牌色文字，深色会跌到 4.18:1，低于正文要求。

还有一条把合格配方用错的典型：**把 `text-disabled` 当「弱化文字」用**。`#999999` 在 `#f3f3f3` 上只有 **2.57:1**，连图形 3:1 都不满足。禁用色只用于真正不可用的控件；凡是需要「比次要文字再弱一档但仍可读」的场合一律用 `--color-text-tertiary`。判断标准只有一条：**这段文字在控件可用时是否仍会显示**——会，就是 tertiary；只在控件禁用时才出现，才是 disabled。

被否决的两条旧组合（不得再出现）：

| 组合 | 实测 | 问题 |
| --- | --- | --- |
| `accent` 文字 on `surface`（深色） | 3.13:1 | 品牌蓝作文字色 |
| `accent` 文字 on `accent-soft`（深色） | 2.60:1 | 叠色后进一步恶化 |

即使把文字换成 `#479EF5`，在 20% 品牌底上仍只有 3.67:1 —— **说明「品牌浅底 + 品牌文字」这个组合本身不可用**，与色值无关。故选中态改用中性底 `--color-selected-bg`。

### 2.4 选中态配方

```
背景    --color-selected-bg
文字    --color-text-primary（品牌感由左侧指示条承担，不用文字色）
指示条  3px × 18px，--color-brand-fg
hover    --elevation-hover + 文字转 --color-text-primary
```

浅色 `#EBEBEB` + `#1a1a1a` = 14.60:1；深色 `#383838` + `#ffffff` = 11.73:1。

**数据网格是另一种选中，不是同一种语言的两个色。** Excel 选区表达「正在框选的数据范围」，列表/导航选中表达「已选定的条目」——前者要在密集单元格里一眼可辨，中性 `#EBEBEB` 压在白底单元格上几乎看不见；后者要安静，品牌调浅底会把每一行都吵成高亮。故网格选区用 `--color-selection-bg`（品牌调浅底）+ `--color-brand-fg` 选区描边（Handsontable `--ht-selection-*` 两个变量就是这对组合），并且：

- 网格选区**不要**换成 `--color-selected-bg`，列表/导航**不要**借用 `--color-selection-bg`；
- 网格选区内文字仍用 `--color-text-primary`（浅色 `#e5f1fb` + `#1a1a1a` ≈ 15.2:1，深色选区叠加后 + `#ffffff` ≈ 11.7:1），品牌色只出现在选区描边上；
- 判断口诀：**「框出来的范围」用 selection-bg，「选中的条目」用 selected-bg + 指示条。**

### 2.5 开关按钮配方（命令栏的目录 / 预览 / 源码 / 自动换行等）

Fluent 2 的 ToggleButton 开启态就是中性底，不用品牌色；但 `--color-selected-bg` 有两处撞色，不能直接给开关按钮用：

| 主题 | 工具栏底 | hover | selected-bg | 结果 |
| --- | --- | --- | --- | --- |
| 浅色 | `#f3f3f3` | `#e0e0e0` | `#EBEBEB` | 选中比 hover **更浅**，移入反而像未开启 |
| 深色 | `#2b2b2b` | `#363636` | `#383838` | 只差 2 级，**基本同色** |

因此开关按钮单独取 `--color-selected-strong`（浅 `#CFCFCF` / 深 `#4A4A4A`），保证「已开启」恒比 hover 显著一档：

```
背景    --color-selected-strong
文字    --color-text-primary（#1a1a1a / #ffffff，实测 11.97:1 / 10.37:1）
hover   --elevation-hover（不受开启态影响，视觉上仍比开启态浅/深一档）
```

**为什么开关态不染品牌色**：12px 文字下，品牌色放不进任何候选底 —— `#479EF5` 在 `#383838` 上 4.18:1、在 14% 品牌浅底上 4.07:1、`#0F6CBD` 在 `#CFCFCF` 上 3.46:1，全部低于 4.5:1。深色主题里品牌色只能以图形身份出现（3:1），所以品牌感由目录抽屉、章节树指示条和焦点环承担。

## 2.6 按下态（pressed）

Fluent 2 的五态（rest / hover / pressed / focus / disabled）里，pressed 必须明显区别于 hover，否则点击没有反馈。v1.0.0 的命令栏按钮 `:active` 与 `:hover` 同为 `--elevation-hover`，按下时画面完全不变。

| 变量 | 浅色 | 深色 | 用法 |
| --- | --- | --- | --- |
| `--elevation-pressed` | `rgba(0,0,0,.12)` | `rgba(255,255,255,.09)` | 透明底按钮、导航项、图标按钮的按下态 |

叠加顺序：rest（透明）→ hover（8% / 5%）→ pressed（12% / 9%）。深色主题下两档都是「变亮」，保持同向层级。

**例外**：已经处于选中态的元素（导航激活项、`aria-pressed="true"` 的开关按钮）不再叠 pressed —— 它们已有 `selected-bg` / `selected-strong` 承担状态，再叠一层会糊成一片。

## 3. 字体

### 3.1 字体族

```css
font-family:
    'Segoe UI Variable',
    'Segoe UI',
    system-ui,
    'Microsoft YaHei UI',
    'Microsoft YaHei',
    '微软雅黑',
    sans-serif;
```

- 首位必须是 `Segoe UI Variable`（Windows 11 可变字体）。缺失时回落 `Segoe UI`。
- **不得前置 `Inter`**：它不是 Windows 系统字体，也不在 Fluent 2 规范内；一旦用户安装就会整体偏离 Fluent 字形。
- 中文用 `Microsoft YaHei UI`（系统 UI 变体），其后才是 `Microsoft YaHei`。

### 3.2 字号阶梯

Fluent 2 只有 10 / 12 / 14 / 16 / 20 / 24 / 28 / 32 / 40 / 68 十档。项目用其中七档（10–24 与 32）：

| 变量 | 字号 | 行高 | Fluent 2 token | 用途 |
| --- | --- | --- | --- | --- |
| `--font-2xs` | 10px | 14px | Caption2（Fluent 1 对应 Base100） | 全大写分组标签、序号 |
| `--font-xs` | 12px | 16px | Caption1（Fluent 1 对应 Base200） | 描述、元信息、空态 |
| `--font-sm` | 14px | 20px | Body1 | **UI 主力**：按钮、导航、设置行标题 |
| `--font-md` | 14px | 20px | Body1 | 与 `sm` 同值，保留兼容既有引用 |
| `--font-lg` | 16px | 22px | Body2 | 文件名标题、命令栏标题 |
| `--font-xl` | 16px | 22px | Body2 | 与 `lg` 同值，保留兼容既有引用 |
| `--font-2xl` | 20px | 28px | Subtitle1 | 身份卡名称、抽屉标题 |
| `--font-3xl` | 24px | 32px | Title3 | 页面标题 |
| `--font-4xl` | 32px | 40px | Hero800 | **仅**字体样张家族名（设置页「字体」样张），界面其余部分不得使用 |

**被删除的档位**：13px、15px、18px、以及全部半像素值（12.5 / 11.5 / 10.5）。新代码不得写入上表以外的字号。

**例外一：阅读正文。** Markdown / EPUB / DOCX 的正文、标题与代码字号属于「内容排版」而非 UI 文本，由用户可调的阅读字号控制（EPUB 14–24 六档、Markdown 15px 基准），不受本表约束。命令栏、目录、状态文字等 UI 部件仍须遵守上表。

**例外二：音频沉浸视图。** 该形态按窗口尺寸连续缩放（`--u` 系数），字号由 `clamp()` 在 10–20px 之间插值（`clamp(var(--font-sm), calc(15 * var(--u)), var(--font-lg))` 这类写法），插值过程中必然出现 13 / 15 这类非阶梯值。上下界本身必须写成变量，让边界始终落在阶梯上——这样改阶梯时缩放区不会掉队。这是「连续缩放区」的固有行为，不算违规；但插值的上下界必须落在本表内，且该区之外的任何 UI 文本不得借用这个例外。

### 3.3 行高

UI 行高与字号**成对绑定**——Fluent 2 type ramp 的做法是每个档位一个绝对行高，不是一条统一系数。样式里不直接写字面量：

| 变量 | 值 | 绑定字号 | 适用范围 |
| --- | --- | --- | --- |
| `--line-2xs` | 14px | 10px | 全大写分组标签、序号 |
| `--line-xs` | 16px | 12px | 描述、元信息、表头、徽章 |
| `--line-sm` | 20px | 14px | **UI 默认**：按钮、导航、说明文字、目录项 |
| `--line-md` | 20px | 14px | 与 `sm` 同值，对应 `--font-md` |
| `--line-lg` | 22px | 16px | 文件名标题、命令栏标题 |
| `--line-xl` | 22px | 16px | 与 `lg` 同值，对应 `--font-xl` |
| `--line-2xl` | 28px | 20px | 身份卡名称、抽屉标题、对话框标题 |
| `--line-3xl` | 32px | 24px | 页面标题 |
| `--line-4xl` | 40px | 32px | 字体样张家族名 |

**为什么废除统一系数**：v1.3.0 前的 `--line-tight: 1.33` / `--line-base: 1.43` 想用一个系数同时产出 10/14、12/16、14/20、16/22 四组配对，数学上不成立（1.33 × 10 = 13.3 ≠ 14，1.43 × 16 = 22.9 ≠ 22）。实现被逼出 `line-height: 14px` 这类字面量——值是对的，形式违规，根子在基准自身。现在每个档位一个绝对值，与字号变量同后缀一一对应：`font-size: var(--font-xs)` 必配 `line-height: var(--line-xs)`。

内容区因字号可调（EPUB 14–24 六档、Markdown 15px 基准），无法绑定绝对值，仍用系数，且**只允许在内容区与多尺度展示区使用**，UI 部件不得借用：

| 变量 | 值 | 适用范围 |
| --- | --- | --- |
| `--line-reading` | 1.75 | 阅读正文（Markdown / EPUB / 纯文本） |
| `--line-code` | 1.5 | 代码块与源码视图 |
| `--line-loose` | 1.7 | 阅读区的宽松行：源码视图、长段落说明 |
| `--line-sample` | 1.35 | **多尺度展示区**：字体样张的阶梯行（12–48px 同屏）、音频沉浸视图的插值文本、Markdown 阅读区的 em 标题组（随 `--md-fs` 缩放）——字号跨档/连续变化，绑单一绝对值会随档位漂移 |

v1.2.0 起 EPUB 正文由 1.8 收到 `--line-reading`：同一个工具里的 Markdown 与 EPUB 正文不该有两套呼吸感，2.8% 的差值肉眼不可辨，但把「阅读密度」收敛成了一个值。

`line-height: 1` 只允许用于单行控件的垂直居中（图标按钮、输入框内文本），语义是「不叠加行距」，不算字面量违规。

### 3.4 字重与字距

- 只用 400 / 500 / 600 三档（分别对应 `fontWeightRegular` / `fontWeightMedium` / `fontWeightSemibold`）；不使用 650、700 等非 token 值。
- 强调一律 600。500 仅用于 12px 以下的小号标签（徽章、表头），因为该档位在中文小字号下 600 会糊。
- 负字距只允许用于 ≥ 20px 的标题（`-0.01em`）。
- 全大写标签的字距上限 `0.04em`（原 `0.08em` 过宽）。

### 3.5 控件尺寸

Fluent 2 的控件高度只有三档，项目不得出现中间值：

| 变量 | 值 | Fluent 2 | 用途 |
| --- | --- | --- | --- |
| `--control-sm` | 24px | small | 只读数值胶囊、分段控件内按钮 |
| `--control-md` | 32px | medium | **默认**：命令栏按钮、标题行按钮、输入框、下拉、图标按钮、导航项 |
| `--control-lg` | 40px | large | 对话框底部主按钮（当前未使用） |

v1.0.0 的实现里同时存在 22 / 26 / 28 / 30 / 34 / 36 六种高度，同一排控件高度互不一致。v1.1.0 收敛为：命令栏按钮 28 → 32，标题行按钮 34×30 → 32×32，命令栏输入框 26 → 32，只读数值胶囊 22 → 24，设置页导航项 36 → 32。命令栏容器高度随之 40 → 44，与标题行等高。v1.4.0 清掉残余的 26 / 28 / 30 中间值（见 §6.6）。

**徽章 / chip 不走控件阶梯**，单独一档：

| 变量 | 值 | 用途 |
| --- | --- | --- |
| `--badge-h` | 22px | 纯标签：格式徽章、大小、语言、类型、筛选键 |

判断标准：**数值胶囊**（页码、字号值、缩放比，可编辑或只读）走 `--control-sm`（24，即 §3.5 表中的「只读数值胶囊」档）；**纯标签**（格式、大小、语言、类型、筛选键）是徽章 → `--badge-h`（22）。v1.3.0 前同类元素有 18 / 20 / 22 / 24 四种高度并存，现收敛为两档且各归其位。

**例外：音频沉浸视图的规格胶囊**（MP3 / 320k）。该形态整体按 §3.2 例外二缩放，胶囊高度随 10/14 行对走（14px），不套 `--badge-h`——560×360 的顶部行按 56px 封面高度排版，22px 徽章会撑破该行节奏。其余窗口的标题行胶囊一律 `--badge-h`。

### 3.6 图标体系与尺寸

**来源唯一**：界面图标一律取自 `@vicons/fluent`（Fluent System Icons，填充式、端点统一）。禁止三件事：

1. **手绘描边 `svg`**。1.3 / 1.4px 的笔画与 Fluent 图标的视觉重量差一档，同一排里并排时最明显（v1.0.0 的设置页导航就是这种状态：手绘地球、月亮、文件夹与旁边的 Fluent 图标粗细不一）。
2. **`@element-plus/icons-vue`**。Element 的图标是 24 网格线性风，与 Fluent 不同族；按钮文字、进度条等 Element 组件内部的图标不在此列，那是组件自带资产。
3. **同一排控件混用不同绘制网格**。20 网格缩到 16 可以接受（笔画换算后 1.25 × 16/20 ≈ 1.0，与 16 网格几乎一致）；24 放大到 48（2×）只允许用于插画。

只有两类例外，且不得与界面图标并排出现：

- **应用标识**：设置页「关于」里的 QuickLook 图标是品牌图形，不算图标，允许自绘；
- **空态 / 错态插画**：成幅的整页图形（≥ 40px），笔画按 48 网格取 2px，保证放大后不糊。

尺寸只有这几档：

| 变量 | 值 | 用途 |
| --- | --- | --- |
| `--icon-xs` | 12px | 下拉箭头、输入框内联提示、代码块复制按钮；**不得作按钮主图标** |
| `--icon-sm` | 16px | **默认**：命令栏、标题行、设置页导航 |
| `--icon-md` | 20px | 卡片标题、列表主图标 |
| `--icon-lg` | 24px | 空态、区块标题 |
| `--icon-xl` | 32px | 空态 / 错态的状态图形（如升级页的成败图标） |

15px 及任何中间值一律不允许：Fluent 的网格只有在整数尺寸上笔画才清晰。图标必须同时给宽高（`width` + `height`），只给一侧会被内联 `svg` 的 `max-width: 100%` 拉伸。

**状态图标要成对**。升级页的「已是最新」与「检查失败」是并列关系，图标不该一大一小：原先错态用 56px 的 Element 三角警告、成功态用 32px 的对勾，两页之间还各自为政。现在统一为 `DismissCircle32Regular` + `Checkmark32Regular`，同为 32 网格。Warning 在 Fluent 里最大只有 24，放大到 32 会发虚，这也是错态改用圆形 DismissCircle 的原因。

## 4. 焦点指示

Fluent 2 的焦点环是**双层**：外层 2px 主色、内侧 1px 背景色隔离，保证在任意底色上可见。

```css
:focus-visible {
    outline: 2px solid var(--color-brand-fg);
    outline-offset: 1px;
    box-shadow: 0 0 0 1px var(--color-bg);
}
```

`outline-offset: 1px` 提供外层缝隙，`box-shadow` 提供内层隔离环。

## 5. 圆角（明确偏离，不改）

| 变量 | 项目 | Fluent 2 | 说明 |
| --- | --- | --- | --- |
| `--radius-xs` | 2px | 2px | **同档**：进度条、指示条、内联代码、输入框内的小按钮 |
| `--radius-sm` | 4px | 2px | 大一档：32px 及以下的按钮、导航项、搜索框内控件 |
| `--radius-md` | 8px | 4px | 大一档：卡片内的分组、信息条、目录 |
| `--radius-lg` | 10px | 6px | 介于 Fluent 2 的 large(8) 与 xlarge(12) 之间，**无对应档**，属自创值：设置页卡片 |
| `--radius-xl` | 12px | 8px | 大一档：窗口、悬浮面板 |
| `--radius-pill` | 999px | — | 胶囊：开关、徽章、chip |

项目圆角整体比 Fluent 2 大一档（`sm` / `md` / `xl` 成立，`lg` 是自创的 10px），且 12px 窗口圆角在 Fluent 2 中无对应。这是刻意的品牌化偏离：桌面原生窗口需要更柔和的边界，缩小圆角会让悬浮控件显得生硬。**本次不调整**。

硬性要求：

- **样式里不得出现字面量圆角**，除 `50%`（圆点、圆形按钮）与 `0`（明确要直角）之外一律走变量；
- 胶囊不要用「高度的一半」去近似。v1.0.0 里同时存在 `11px`（22px 高的 chip）、`16px`（音量浮层）、`9px`（18px 高的 tag）—— 三个值都在表达「圆角 = 一半高度」，现在统一为 `--radius-pill`；
- 小型控件（32px 及以下的按钮、导航项）用 `--radius-sm`，不要再出现 6px 这类中间值。

## 6. 迁移映射（实施依据）

### 6.1 颜色

| 原写法 | 新写法 | 涉及 |
| --- | --- | --- |
| `color: var(--color-accent)` | `color: var(--color-brand-fg)` | 链接、激活导航、内联强调 |
| `color: var(--color-accent-hi)` | `color: var(--color-brand-fg)` | 状态文字、设置页多处 |
| `--el-color-primary: var(--color-accent-hi)` | `var(--color-brand-fg)` | Element 重皮 |
| `background: var(--color-accent-soft)` + 品牌色文字 | `background: var(--color-selected-bg)` + `var(--color-text-primary)` | 导航激活、选中项 |
| `background: var(--color-selected-bg)`（开关按钮） | `background: var(--color-selected-strong)` | 命令栏开关按钮的按下/开启态 |
| `color: var(--color-danger)`（作文字） | `color: var(--color-danger-fg)` | 内联校验、清理提示 |
| `background: var(--color-accent)` | 保持 | 按钮底、开关轨道、指示条 |

### 6.2 字号

| 原值 | 新值 | 典型位置 |
| --- | --- | --- |
| 13px / `--font-sm`(13) | 14px | 导航项、SubtleButton、设置行标题 |
| 15px / `--font-lg`(15) | 16px | 文件名标题、页面标题副行 |
| 18px / `--font-2xl`(18) | 20px | 身份卡名称 |
| 26px | 24px | 设置页页面标题 |
| 11px | 12px | hint、导航底部版本号 |
| 11.5px / 12.5px | 12px | 元信息、表头、数值 |
| 10.5px | 10px | 导航序号 |
| 7px | 10px | 装饰性上标 |

### 6.3 字体族

`base.css` 与三个原型统一为 §3.1 的栈。原型中 `'Segoe UI Variable Display'` 改为 `'Segoe UI Variable'`（Display 光学尺寸仅适用于 ≥24px 标题，正文用 Text，共用 `Segoe UI Variable` 是最稳定的写法）。

### 6.4 三级文字、尺寸与状态（v1.1.0 增补）

| 原写法 | 新写法 | 涉及 |
| --- | --- | --- |
| `color: var(--color-text-disabled)` 承载常显信息 | `color: var(--color-text-tertiary)` | 命令栏信息与占位、设置页说明/统计/空态/导航版本号、目录三级项、代码行号、状态图标 |
| `height: 28px`（命令栏按钮） | `height: var(--control-md)` | `preview-chrome.css` |
| `height: 30px` / `width: 34px`（标题行按钮） | `var(--control-md)` 双向 32 | `header.vue` |
| `height: 26px`（命令栏输入） | `var(--control-md)` | `preview-chrome.css` |
| `height: 22px`（只读数值胶囊） | `var(--control-sm)` | `preview-chrome.css` |
| `height: 36px`（设置页导航项） | `var(--control-md)` | `settings.vue` |
| `width: 15px`（图标） | `var(--icon-sm)` | 命令栏、标题行 |
| `:active { background: var(--elevation-hover) }` | `var(--elevation-pressed)` | 所有透明底可点击元素 |
| `<div @click>`（标题行操作） | `<button type="button" aria-label>` | `header.vue` |

### 6.5 图标、圆角与行高（v1.2.0 增补）

| 原写法 | 新写法 | 涉及 |
| --- | --- | --- |
| 手绘 `<svg stroke-width="1.3">`（导航、搜索、加号、提示） | `@vicons/fluent` 对应图标 | `settings.vue` |
| 手绘 `<svg>`（搜索、清空、排序箭头、文件夹、文件、树展开） | `Search16Regular` / `Dismiss16Regular` / `ArrowUp16Regular` / `Folder16Regular` / `Document16Regular` / `ChevronRight16Regular` | `archive.vue` |
| 手绘 `<svg>`（换行、PDF 旋转/缩放/翻页、反色） | `ArrowWrap20Regular` / `ArrowRotateClockwise16Regular` / `ZoomIn16Regular` / `ZoomOut16Regular` / `ChevronLeft16Regular` / `ChevronRight16Regular` / `CircleHalfFill16Regular` | `code.vue`、`document.vue`、`font.vue` |
| 手绘环形箭头（后退/前进 10 秒） | `Rewind16Regular` / `FastForward16Regular` | `audio.vue` |
| `@element-plus/icons-vue`（`Warning` / `Check` / `Refresh` / `CircleClose` / `Download`） | `DismissCircle32Regular` / `Checkmark32Regular` / `ArrowClockwise16Regular` / `DismissCircle16Regular` / `ArrowDownload16Regular` | `upgrade.vue` |
| `<el-icon :size="15">` | 去掉 `size`，由 `.pv-tb-btn .el-icon { font-size: var(--icon-sm) }` 统一 | 全部预览视图命令栏 |
| `width: 14px`（设置页图标） | `var(--icon-sm)` | `settings.vue`、`archive.vue`、`preview-chrome.css` |
| `border-radius: 2px / 3px` | `var(--radius-xs)` | 进度条、指示条、内联代码、徽章 |
| `border-radius: 6px`（导航项、分段控件） | `var(--radius-sm)` | `settings.vue` |
| `border-radius: 9px / 11px / 16px`（胶囊） | `var(--radius-pill)` / `var(--radius-xl)` | `font.vue`、`settings.vue`、`audio.vue` |
| `line-height: 1.2 / 1.3 / 1.35 / 1.4` | `var(--line-tight)` | 标题、导航、元信息 |
| `line-height: 1.45 / 1.5` | `var(--line-base)` | 目录项、卡片说明 |
| `line-height: 1.75 / 1.8`（阅读正文） | `var(--line-reading)` | `md-doc.scss`、`book.vue` |
| `line-height: 1.5`（代码块） | `var(--line-code)` | `book.vue` |

### 6.6 控件高度、徽章、行高与 token 收口（v1.4.0 增补）

| 原写法 | 新写法 | 涉及 |
| --- | --- | --- |
| `height: 26px` / `28px`（播放按钮、归档行） | `var(--control-sm)` | `audio.vue`、`archive.vue` |
| `height: 30px`（状态按钮、下拉项） | `var(--control-md)` | `book-doc.scss`、`model.vue`、`pdf-doc.scss`、`md-doc.scss`、`font.vue`、`settings.vue` |
| `height: 20px`（PDF 页码输入） | `var(--control-sm)` | `pdf-doc.scss` |
| `width/height: 14px`（chip 关闭图标） | `var(--icon-xs)` | `settings.vue` |
| `.pv-tb-btn svg.is-sm` 错映 `--icon-xs` | `--icon-sm` | `preview-chrome.css` |
| `height: 18px / 20px`（pill、chip 徽章） | `var(--badge-h)` | `preview-chrome.css` |
| `line-height: 22px / 30px / 14px`（字面量） | `var(--line-lg)` / `var(--line-sm)` / `var(--line-2xs)` | `settings.vue`、`audio.vue` |
| `--line-tight` / `--line-base` | 按所在字号档位取 `--line-2xs … --line-4xl`（内容区 em 相对字号改取 §3.3 系数） | 全局 |
| `--color-hover-bg` | `--color-selection-bg` | `var.css`、Excel 选区 |
| `background: #e81123`（关闭钮 hover） | `var(--color-close-hover)` | `audio.vue` |
| `color: #fff`（实心按钮文字） | `var(--btn-primary-text)` | `audio.vue`、`header.vue` |
| `font-weight: 700` | `600` | `md-viewer` 内容层 |
| 标题行高字面量 `40 / 32 / 28 / 24px` | `var(--line-3xl/2xl/…)` | `vp-doc.css` |
| 标题字距 `-0.02em` / 18px 标题的负字距 | `-0.01em` / 去掉 | `vp-doc.css`、`md-doc.scss` |

## 7. 窗口形态：页面 vs 对话框

同一个应用里有两种窗口，Fluent 2 对它们的组织方式不同。判断标准只有一条：**这个窗口是一个会被反复访问的场所，还是一次性的任务？**

| | 页面型（设置、预览） | 对话框型（更新） |
| --- | --- | --- |
| 客户区底 | `--color-bg` | `--color-surface` |
| 内容组织 | 卡片：`--color-surface` + 1px `--color-border` + `--radius-lg` | 三段式：标题区 / 内容区 / 页脚，直接铺在客户区 |
| 分区手段 | 卡片之间的留白 | `--color-divider` 发丝线 |
| 嵌套 | 卡片内再用 divider 分设置行 | **不再嵌套**：不叠第二层底色与描边 |
| 标题 | 页面标题 24/32 · 600，左上 | 对话框标题并入 44px 标题行（16/22 · 600），不另起标题区 |
| 页脚 | 无，内容自己到底 | 固定底部，主操作最右 |

判断错的两个方向都要避免：

- **对话框套页面**：灰底 + 白卡片 + 四周 16px 边距。600px 宽的窗口里，16px 页边加 24px 卡内边距 = 40px 双重留白，内容被压扁，看起来像「网页里放了一张卡片」。
- **页面套对话框**：整窗 surface、无卡片。设置页有几十行、8 个分组，失去卡片边界就没有分组感。

更新窗口按对话框处理（见 `design/upgrade-ui-design.md`），设置窗口与预览窗口按页面处理。

### 7.1 状态态居中，内容态起左上

同一个窗口里的两种内容归两类，排布规则不同：

| | 状态态（检查中、空、已是最新、成功、失败） | 内容态（清单、键值表、进度块、更新说明） |
| --- | --- | --- |
| 排布 | 整块居中：图标 / 标题 / 说明逐段水平居中，垂直居中于内容区 | 从左上起排，按行序读 |
| 说明文字 | 限宽 `min(100%, 420px)`，超出居中折行 | 占满内容宽 |

状态态只有一到两行字，而内容区常常有 300–400px 高。按左上起排会在右下各留一大片空白，画面重心整个塌向一角，第一眼看到的是空白而不是结论。内容态反过来——居中会破坏阅读起点，标题与列表的左边线必须对齐。

**居中也要限宽**。420px 是居中文本的单行上限，再宽，换行后视线要从行尾跨过半个窗口才回到行首。所以长错误详情仍然居中，只是收窄到这个宽度折行，而不是退回左对齐——左对齐解决的是「行首对不齐，看不出层级」，不是「一行太长」。

描边也分两档：容器边界用 `--color-border`（NeutralStroke2），**带边框的标准按钮**用 `--color-stroke-control`（NeutralStroke1）。差别在浅色下尤其明显——`#e1e1e1` 画在 surface 上几乎只剩一道影子。

### 7.2 标题栏由前端自绘

三个窗口（预览 / 设置 / 更新）统一 `decorations(false)`，标题栏共用 `window-titlebar.vue`：44px 高、透明底、右侧 32×32 控件（最小化 / 最大化 / 关闭，关闭钮 hover 实心红）。固定尺寸的对话框只给关闭钮。

标题栏有两种变体，差别只在左侧那段字：

| 变体 | 用在哪 | 左侧文字 |
| --- | --- | --- |
| `window`（默认） | 页面型窗口 | 窗口名，12/16 · 400 次要色——只标明「这是哪个窗口」，不参与内容层级 |
| `dialog` | 对话框 | 唯一标题，16/22 · 600 主色、左缩进 24——它就是内容标题，下方不再另起标题区 |

**对话框不要「空标题栏 + 大标题」两层。** 无边框窗口的标题栏也由前端画：若它只放一颗关闭钮，顶部会先出现一条 44px 空带、紧接着再出现一行 20px 大标题，两层抢同一个位置，空带既无信息又白占 40px。判断标准很简单——标题栏里有没有字：有字就是页面型窗口（窗口名与内容标题各司其职），没字就该把内容标题收进这一行。

| 部位 | 绘制方 | 约束 |
| --- | --- | --- |
| 标题栏 | webview | 随 `html.dark` 自动明暗，**不需要** `setTheme` 去对齐系统主题 |
| 拖动 | webview | `data-tauri-drag-region`（自带双击最大化） |
| 边缘缩放 | webview | 8 条透明热区 → `startResizeDragging`；最大化时热区失效 |
| 系统投影 / 圆角 | OS | 无边框窗口没有。`shadow(true)` 在 Windows 会给窗口加 1px 白边，深色下更刺眼，故不启用 |

自绘的收益就是一句话：**标题栏在 webview 里，明暗跟着 `html.dark` 走**，「应用内切深色、系统是浅色」不会再出现深页面配浅标题栏。

代价要认：拖动与缩放都得自己补，弹窗也不再带系统投影。所以只在三个常驻窗口里统一，临时对话框（原生 `MessageDialog`）仍交给 OS。

主题初始化仍须**每窗口各一次**：Tauri 每个窗口是独立 webview、各有独立 `document`，初始化收敛在 `src/hooks/theme.ts` 的模块作用域（由 `main.ts` 副作用导入，早于 `app.mount`，避免首帧白闪）。不要指望「页面自己调 `useTheme()` 来完成初始化」——没有调用它的窗口（如更新页）会直接停在浅色，这正是曾经的故障形态。

## 8. 验收清单

- [ ] 全局不存在 `color: var(--color-accent)`（仅允许 `background` / `border` 使用）
- [ ] 全局不存在 `--color-accent-hi`
- [ ] 深色模式下任一品牌色文字在其实心底上实测 ≥ 4.5:1
- [ ] `--color-text-disabled` 只出现在 `:disabled` / `.is-disabled` / `--el-disabled-*` 上，不承载常显信息
- [ ] UI 区 CSS 中不存在 13 / 15 / 18px 及任何 `.5px` 字号（内容区例外见 §3.2 例外一）
- [ ] `font-family` 首位为 `Segoe UI Variable`，无 `Inter`
- [ ] `font-weight` 只出现 400 / 500 / 600，其中 500 仅限 ≤12px 徽章与表头（§3.4）
- [ ] 浅色 / 深色切换后，导航激活项、危险文字、焦点环三处对比度与上表一致
- [ ] 所有可点击元素是 `<button>` / `<a>`，或带 `role` + `tabindex`，Tab 键可遍历且焦点环可见
- [ ] 每个可点击元素都有 `hover` 与 `:active`，且两者视觉可区分
- [ ] 控件高度只取 `--control-sm/md/lg`（24 / 32 / 40）三档；只读标签高度一律 `var(--badge-h)`（22）
- [ ] UI 行高按 §3.3 档位绑定绝对值（`--line-2xs … --line-4xl`），内容区系数（`--line-reading/code/loose`）不出现在 UI 部件上
- [ ] 图标只取 `--icon-xs/sm/md/lg/xl`（12 / 16 / 20 / 24 / 32）五档，且同时给了宽与高
- [ ] 界面图标全部来自 `@vicons/fluent`：全局无手绘的界面 `<svg>`，无 `@element-plus/icons-vue` 引入
- [ ] 空态 / 错态的状态图形成对同尺寸，不出现一大一小
- [ ] 状态态（加载 / 空 / 成功 / 失败）整块居中，说明文字限宽 420px；内容态从左上起排（见 §7.1）
- [ ] 样式里无字面量 `border-radius`（`50%` 与 `0` 除外），胶囊一律 `--radius-pill`
- [ ] 样式里无字面量 `line-height`（单行控件的 `1` 除外）
- [ ] 页面背景不含硬编码渐变，深色模式下无浅色残留
- [ ] 窗口形态与用途相符：页面型窗口用 `--color-bg` + 卡片，对话框型窗口整窗 `--color-surface`（见 §7）
- [ ] 对话框内无嵌套卡片：分区用 `--color-divider` 发丝线，不叠第二层底色与描边
- [ ] 带边框的标准按钮用 `--color-stroke-control`，容器边界才用 `--color-border`
