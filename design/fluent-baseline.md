# Fluent 2 视觉基准（颜色 · 字体 · 焦点）

> 适用范围：`design/preview-ui-design.md`、`design/settings-ui-design.md`、`design/audio-preview-design.md` 及其原型
> 目标代码：`src/assets/var.css`、`src/assets/var/layout.css`、`src/assets/base.css`
> 版本基线：v1.1.0（本文件引入的 token 变更）
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
| `--color-text-disabled` | `#999999` | `#777777` | NeutralForegroundDisabled | 禁用文字 |
| `--color-brand-fg` | `#0F6CBD` | `#479EF5` | BrandForeground1 | **品牌前景** |
| `--color-accent` | `#0078D4` | `#0078D4` | BrandBackground | **品牌背景** |
| `--color-accent-soft` | `rgba(15,108,189,.10)` | `rgba(71,158,245,.14)` | BrandBackground2 | 轻提示底（不承载品牌色文字） |
| `--color-selected-bg` | `#EBEBEB` | `#383838` | NeutralBackground1Selected | **导航激活 / 列表选中**（配品牌指示条） |
| `--color-selected-strong` | `#CFCFCF` | `#4A4A4A` | NeutralBackground1Pressed 近似 | **开关按钮的「已开启」**，必须比 hover 深一档 |
| `--color-danger-fg` | `#A80000` | `#FF99A4` | StatusDangerForeground | 危险色文字 |
| `--color-danger` | `#D13438` | `#D13438` | StatusDangerBackground3 | 危险色底（极少用） |
| `--color-border` | `#e1e1e1` | `#3a3a3a` | NeutralStroke2 | 容器边界 |
| `--color-divider` | `rgba(0,0,0,.07)` | `rgba(255,255,255,.08)` | NeutralStroke3 | 发丝线 |
| `--color-chip-bg` | `rgba(0,0,0,.05)` | `rgba(255,255,255,.06)` | NeutralBackground4 | 胶囊底 |
| `--elevation-hover` | `rgba(0,0,0,.08)` | `rgba(255,255,255,.05)` | SubtleBackgroundHover | 悬浮态 |

`--color-accent-hi` 已删除，其全部引用改为 `--color-brand-fg`。

### 2.3 对比度验收矩阵

WCAG 2.1 AA：正文 ≥ 4.5:1，图形/大字 ≥ 3:1。下列为**必须全部满足**的组合。

| 组合 | 浅色 | 深色 | 要求 |
| --- | --- | --- | --- |
| primary on surface | 17.40:1 ✅ | 14.16:1 ✅ | 4.5 |
| secondary on surface | 7.46:1 ✅ | 8.82:1 ✅ | 4.5 |
| **brand-fg on surface** | **5.38:1** ✅ | **5.04:1** ✅ | 4.5 |
| **danger-fg on surface** | **7.88:1** ✅ | **6.97:1** ✅ | 4.5 |
| white on accent（按钮底） | 4.53:1 ✅ | 4.53:1 ✅ | 4.5 |
| primary on accent-soft | 15.10:1 ✅ | 11.43:1 ✅ | 4.5 |
| secondary on accent-soft | 6.47:1 ✅ | 7.12:1 ✅ | 4.5 |
| brand-fg 作焦点环 on surface | 5.38:1 ✅ | 5.04:1 ✅ | 3.0 |
| brand-fg 作指示条 on selected-bg | 4.52:1 ✅ | 4.18:1 ✅ | 3.0 |
| disabled（豁免 WCAG） | 2.85:1 ➖ | 3.16:1 ➖ | — |

两点必须注意，否则会把合格配方用错：

1. **`accent-soft` 上只能放 `text-primary` / `text-secondary`，不能放 `brand-fg`**。品牌色文字在浅底上会跌到 4.07:1（深色）。InfoBar 因此用中性文字 + 品牌色图标。
2. **`selected-bg` 上同理不放品牌色文字**。品牌色只出现在 3px 指示条上，按图形 3:1 判定（实测 4.52 / 4.18，合格）。若在选中行用品牌色文字，深色会跌到 4.18:1，低于正文要求。

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

Fluent 2 只有 10 / 12 / 14 / 16 / 20 / 24 / 28 / 32 / 40 / 68 十档。项目用前六档：

| 变量 | 字号 | 行高 | Fluent 2 token | 用途 |
| --- | --- | --- | --- | --- |
| `--font-2xs` | 10px | 14px | Base100 · Caption2 | 全大写分组标签、序号 |
| `--font-xs` | 12px | 16px | Base200 · Caption1 | 描述、元信息、空态 |
| `--font-sm` | 14px | 20px | Base300 · Body1 | **UI 主力**：按钮、导航、设置行标题 |
| `--font-md` | 14px | 20px | Base300 · Body1 | 与 `sm` 同值，保留兼容既有引用 |
| `--font-lg` | 16px | 22px | Base400 · Body2 | 文件名标题、命令栏标题 |
| `--font-xl` | 16px | 22px | Base400 · Body2 | 与 `lg` 同值，保留兼容既有引用 |
| `--font-2xl` | 20px | 28px | Base500 · Subtitle1 | 身份卡名称、抽屉标题 |
| `--font-3xl` | 24px | 32px | Base600 · Title3 | 页面标题 |

**被删除的档位**：13px、15px、18px、以及全部半像素值（12.5 / 11.5 / 10.5）。新代码不得写入上表以外的字号。

**唯一例外：阅读正文。** Markdown / EPUB / DOCX 的正文与代码字号属于「内容排版」而非 UI 文本，由用户可调的阅读字号控制（EPUB 14–24 六档、Markdown 15px 基准），不受本表约束。命令栏、目录、状态文字等 UI 部件仍须遵守上表。

### 3.3 行高

改为按档位绑定固定像素，不再用单一系数：

| 变量 | 值 | 说明 |
| --- | --- | --- |
| `--line-tight` | 1.33 | 对应 10/14、12/16、24/32 |
| `--line-base` | 1.43 | 对应 14/20，UI 默认 |
| `--line-loose` | 1.7 | 阅读类正文（Markdown / EPUB），不属 UI 阶梯 |

原 1.6 的全局行高偏松（Fluent 2 的 UI 文本在 1.33–1.43），改为 1.43。阅读区各自保留 1.7–1.8。

### 3.4 字重与字距

- 只用 400 / 500 / 600 三档（分别对应 `fontWeightRegular` / `fontWeightMedium` / `fontWeightSemibold`）；不使用 650、700 等非 token 值。
- 强调一律 600。500 仅用于 12px 以下的小号标签（徽章、表头），因为该档位在中文小字号下 600 会糊。
- 负字距只允许用于 ≥ 20px 的标题（`-0.01em`）。
- 全大写标签的字距上限 `0.04em`（原 `0.08em` 过宽）。

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

| 变量 | 项目 | Fluent 2 |
| --- | --- | --- |
| `--radius-sm` | 4px | 2px |
| `--radius-md` | 8px | 4px |
| `--radius-lg` | 10px | 6px |
| `--radius-xl` | 12px | 8px |

项目圆角整体比 Fluent 2 大一档，且 12px 窗口圆角在 Fluent 2 中无对应。这是刻意的品牌化偏离：桌面原生窗口需要更柔和的边界，缩小圆角会让悬浮控件显得生硬。**本次不调整**，记录在此以免被当作遗漏。

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

## 7. 验收清单

- [ ] 全局不存在 `color: var(--color-accent)`（仅允许 `background` / `border` 使用）
- [ ] 全局不存在 `--color-accent-hi`
- [ ] 深色模式下任一品牌色文字在其实心底上实测 ≥ 4.5:1
- [ ] 全局 CSS 中不存在 13 / 15 / 18px 及任何 `.5px` 字号
- [ ] `font-family` 首位为 `Segoe UI Variable`，无 `Inter`
- [ ] `font-weight` 只出现 400 / 600
- [ ] 浅色 / 深色切换后，导航激活项、危险文字、焦点环三处对比度与上表一致
