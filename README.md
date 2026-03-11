# Anamnesis 古籍文献校对编辑器

## 介绍

这是一个古籍文献校对编辑器，通过集成 Kraken OCR 引擎与 CATMuS 模型，本工具允许用户在本地完成从图片到文本的自动化识别，并提供了一个的可视化编辑器。不同于普通的文本编辑器，它支持像素级的多边形（Polygon）与基线（Baseline）调整，并引入了独有的**“单词级（Word-level）”逻辑**，能够处理中世纪文本中常见的单词粘连、跨行断词（Hyphenation/SUBS）以及特殊的组合字符。所有数据均严格遵循 ALTO XML 标准，确保学术研究的数据合规性。

## 调试 \& 构建

本项目使用 `Tauri 2.0` 开发，若要调试或构建，需要 `rust` 与 `node` 环境.

并且用到了 `python` 打包的 `exe` 程序作为 `sidecar`，需要先打包

```bash
cd ./sidecar
```

```bash
pyinstaller ocr.spec
```

之后将 `./sidecar/dist` 中的exe程序复制到 `./src-tauri/binaries` 中，才能正常调试或构建


```bash
pnpm i
# 调试
pnpm tauri dev
# 构建
pnpm tauri build
```

## 交流

欢迎加入 QQ 群 1077917619 交流讨论，遇到 bug 也欢迎进群反馈.

## 支持作者

这个项目是完全开源免费的，如果喜欢，可以投喂作者，非常感谢!

BTC：bc1q7n500ksqg5005adyjzyzs0eamxzvzys0jgde84
ETH：0xbDdBDc37631d40525cD86934Fa30D913a26CD056
Solana：2GJ5H6PBYWXsYTd4YX2J27D6GzBawUiBwbaDkys5odVU