# Anamnesis 古籍文献校对编辑器

## 介绍

这是一个古籍文献校对编辑器，集成了 `Kraken`，并且以 `CATMuS` 和 `blla` 作为默认识别与分割模型，用户也可以加载自己的模型。之后可以用编辑器识别，得到 `ALTO XML`，用户可以用编辑器调整多边形、基线，还可以对每个TextLine内部的字母做精细操作，并支持导出 `ALTO XML`，与 `eScriptorium` 兼容。

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

BTC：bc1qmy0rxv2yglvpw2uelvfhv468us8fc5hr6y0wxw
ETH：0x9B033C903ff37EE65B42bD72f0248B1eC32d0a7a
Solana：2UbqTDCy97FFtXXXhZxq2imyrPyWaEDm1rNuBkyPzVDE