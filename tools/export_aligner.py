"""개발용: wav2vec2 음소 인식 모델(HF) → ONNX int8 변환 + 정확도 확인.

사용: python export_aligner.py <HF 모델 폴더> <출력 폴더> [확인용 16kHz mono wav]
결과: phoneme-aligner-int8.onnx, phoneme-aligner-vocab.json  (앱의 models/ 폴더에 넣음)
"""
import json
import shutil
import sys
import wave
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch
from onnxruntime.quantization import QuantType, quantize_dynamic
from transformers import Wav2Vec2ForCTC

src, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
fp32 = out / "phoneme-aligner-fp32.onnx"
int8 = out / "phoneme-aligner-int8.onnx"

model = Wav2Vec2ForCTC.from_pretrained(src).eval()


class Logits(torch.nn.Module):
    def __init__(self, m):
        super().__init__()
        self.m = m

    def forward(self, x):
        return self.m(x).logits


with torch.no_grad():
    torch.onnx.export(
        Logits(model),
        torch.randn(1, 16000 * 3),
        str(fp32),
        input_names=["input_values"],
        output_names=["logits"],
        dynamic_axes={"input_values": {0: "batch", 1: "samples"}, "logits": {0: "batch", 1: "frames"}},
        opset_version=17,
        dynamo=False,
    )
quantize_dynamic(str(fp32), str(int8), weight_type=QuantType.QInt8)
shutil.copy(src / "vocab.json", out / "phoneme-aligner-vocab.json")
print("fp32 MB", fp32.stat().st_size // 2**20, "int8 MB", int8.stat().st_size // 2**20)

# 정확도 확인: 원본(PyTorch)과 int8 ONNX의 프레임별 최고 음소 일치율
if len(sys.argv) > 3:
    with wave.open(sys.argv[3]) as w:
        pcm = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768
    pcm = pcm[: 16000 * 20]
    x = ((pcm - pcm.mean()) / np.sqrt(pcm.var() + 1e-7))[None, :].astype(np.float32)
    with torch.no_grad():
        ref = model(torch.from_numpy(x)).logits[0].numpy()
    got = ort.InferenceSession(str(int8)).run(None, {"input_values": x})[0][0]
    agree = (ref.argmax(-1) == got.argmax(-1)).mean()
    vocab = {v: k for k, v in json.load(open(src / "vocab.json")).items()}
    seq = [vocab[i] for i in got.argmax(-1) if i != 0]
    dedup = [p for i, p in enumerate(seq) if i == 0 or p != seq[i - 1]]
    print(f"frames {len(got)}  argmax agreement {agree:.3f}")
    print("int8 phones:", " ".join(dedup[:60]))
fp32.unlink()
