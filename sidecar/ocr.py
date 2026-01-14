import sys
import argparse
import dataclasses
from pathlib import Path
from PIL import Image

from kraken import blla
from kraken import rpred
from kraken import serialization
from kraken.lib import models
from kraken.lib import vgsl


def run_ocr(image_path: str, model_path: str, seg_model_path: str = None) -> str:
    """
    使用 kraken 进行 OCR，返回 ALTO XML 格式结果
    """
    # 加载识别模型
    rec_model = models.load_any(model_path)
    
    # 打开图片
    im = Image.open(image_path)
    
    # 确保图片是 RGB 模式
    if im.mode not in ('RGB', 'L'):
        im = im.convert('RGB')
    
    # 加载分割模型
    if seg_model_path:
        seg_model = vgsl.TorchVGSLModel.load_model(seg_model_path)
    else:
        import kraken
        kraken_dir = Path(kraken.__file__).parent
        default_seg_model = kraken_dir / 'blla.mlmodel'
        if default_seg_model.exists():
            seg_model = vgsl.TorchVGSLModel.load_model(str(default_seg_model))
        else:
            raise FileNotFoundError(
                f"Default segmentation model not found at {default_seg_model}."
            )
    
    # 使用 baseline segmentation
    baseline_seg = blla.segment(im, model=seg_model)
    
    # 进行 OCR 识别 - 参数名是 bounds 不是 segmentation
    pred_it = rpred.rpred(
        network=rec_model,
        im=im,
        bounds=baseline_seg
    )
    
    # 收集所有识别记录
    records = list(pred_it)
    
    # 用识别结果替换分割结果中的 lines
    results = dataclasses.replace(pred_it.bounds, lines=records)
    
    # 序列化为 ALTO XML
    alto_xml = serialization.serialize(
        results,
        image_size=im.size,
        template='alto'
    )
    
    return alto_xml


if __name__ == "__main__":
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8')

    parser = argparse.ArgumentParser(description="Kraken OCR Sidecar Wrapper")
    parser.add_argument("image_path", help="Input image path")
    parser.add_argument("--model", "-m", required=True, help="Path to the recognition model file (.mlmodel)")
    parser.add_argument("--seg-model", "-s", help="Path to the segmentation model file (.mlmodel)")
    parser.add_argument("--output", "-o", help="Output file path (optional)")
    
    args = parser.parse_args()

    try:
        result_xml = run_ocr(args.image_path, args.model, args.seg_model)
        
        if args.output:
            with open(args.output, 'w', encoding='utf-8') as f:
                f.write(result_xml)
            print(f"Output saved to {args.output}", file=sys.stderr)
        else:
            print(result_xml)
        
    except Exception as e:
        import traceback
        print(f"Error: {str(e)}", file=sys.stderr)
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)