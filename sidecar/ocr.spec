# -*- mode: python ; coding: utf-8 -*-
import kraken
from pathlib import Path

kraken_dir = Path(kraken.__file__).parent

a = Analysis(
    ['ocr.py'],
    pathex=[],
    binaries=[],
    datas=[
        # 包含 kraken 的模板文件（序列化需要）
        (str(kraken_dir / 'templates'), 'kraken/templates'),
    ],
    hiddenimports=[
        'kraken',
        'kraken.blla',
        'kraken.rpred',
        'kraken.serialization',
        'kraken.lib.models',
        'kraken.lib.vgsl',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='kraken_sidecar',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=True,
)