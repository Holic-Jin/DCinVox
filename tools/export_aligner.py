"""wav2vec2 음소 인식 모델(Hugging Face) → phoneme-aligner-int8.onnx.

사용:
  python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft /path/to/models
  python3 export_aligner.py /local/wav2vec2-xlsr-53-espeak-cv-ft /path/to/models

출력 폴더는 앱(또는 .app) 옆의 models/ 이어야 한다.
결과: phoneme-aligner-int8.onnx, phoneme-aligner-vocab.json
"""
import shutil
import sys
from pathlib import Path

USAGE = """사용: python3 export_aligner.py <HF모델ID 또는 폴더> <출력 models 폴더> [확인용 16kHz mono wav]

예 (배포 zip을 푼 폴더에서, 앱 옆 models 로 출력):
  python3 -m pip install -r requirements.txt
  python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models

phoneme-aligner-int8.onnx 는 zip에 없다. Hugging Face에서 .onnx 를 받는 것이 아니라
facebook/wav2vec2-xlsr-53-espeak-cv-ft 를 받아 여기서 변환한다.
"""

PIP_PACKAGES = "numpy torch transformers onnx onnxruntime huggingface_hub"


def die_missing_python_packages(err: ModuleNotFoundError) -> None:
    py = sys.executable
    here = Path(__file__).resolve().parent
    req = next((p for p in (here / "requirements.txt", here / "aligner-requirements.txt") if p.is_file()), None)
    install = f'"{py}" -m pip install -r "{req}"' if req else f'"{py}" -m pip install {PIP_PACKAGES}'
    sys.stderr.write(
        f"{err.name} 패키지가 없습니다. (실행 중인 Python: {py})\n"
        f"export_aligner.py 는 앱 zip에 들어 있지만, numpy/torch 는 들어 있지 않습니다.\n"
        f"아래를 **방금 그 python3와 같은 것**으로 실행한 뒤 다시 시도하세요:\n"
        f"  {install}\n"
        f"  또는: \"{py}\" -m pip install {PIP_PACKAGES}\n"
        f"pip 자체가 없으면: \"{py}\" -m ensurepip --upgrade\n"
    )
    sys.exit(1)


if len(sys.argv) < 3:
    sys.stderr.write(USAGE)
    sys.exit(2)

import json
import wave

try:
    import numpy as np
    import onnxruntime as ort
    import torch
    from onnxruntime.quantization import QuantType, quantize_dynamic
    from transformers import Wav2Vec2ForCTC
except ModuleNotFoundError as e:
    die_missing_python_packages(e)


def resolve_src(arg: str) -> Path:
    p = Path(arg)
    if p.is_dir() and (p / "config.json").is_file():
        return p
    try:
        from huggingface_hub import snapshot_download
    except ModuleNotFoundError as e:
        die_missing_python_packages(e)

    print("downloading", arg)
    return Path(snapshot_download(arg))


src = resolve_src(sys.argv[1])
out = Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
fp32 = out / "phoneme-aligner-fp32.onnx"
int8 = out / "phoneme-aligner-int8.onnx"

print("loading", src)
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
print("wrote", int8, "int8 MB", int8.stat().st_size // 2**20)
print("wrote", out / "phoneme-aligner-vocab.json")

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
