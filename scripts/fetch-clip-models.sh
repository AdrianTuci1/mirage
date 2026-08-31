#!/bin/bash
# Fetch the verified CLIP artifacts into the layout the daemon discovers.
set -euo pipefail
MODELS="${1:-$PWD/test-models}"
BASE="https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main"

mkdir -p "$MODELS/clip_text_encoder/1.0.0" "$MODELS/clip_vision_encoder/1.0.0" "$MODELS/clip_tokenizer/1.0.0"

fetch() {
  local url="$1" dest="$2" want="$3"
  if [[ -f "$dest" ]]; then
    echo "cached: $dest"
  else
    curl -sSL --fail --retry 3 -o "$dest.part" "$url"
    mv "$dest.part" "$dest"
  fi
  local got
  got=$(shasum -a 256 "$dest" | cut -d' ' -f1)
  if [[ "$got" != "$want" ]]; then
    echo "CHECKSUM MISMATCH $dest"
    echo "  expected $want"
    echo "  got      $got"
    exit 1
  fi
  echo "ok: $dest ($(stat -f%z "$dest") bytes)"
}

fetch "$BASE/onnx/text_model_int8.onnx" "$MODELS/clip_text_encoder/1.0.0/text_model_int8.onnx" \
  18845f2ccc35223bb7fec403383a131154b11ac0918df25cf51986df5efd3a21
fetch "$BASE/onnx/vision_model_int8.onnx" "$MODELS/clip_vision_encoder/1.0.0/vision_model_int8.onnx" \
  0ab0c1b3ace708e539633af1744d5a95247fe4e14d3e08ff197ef82a6cb9bd93
fetch "$BASE/tokenizer.json" "$MODELS/clip_tokenizer/1.0.0/tokenizer.json" \
  f7f3b7af117d467b58374797691a6438d3e6b9e9cef800dfd5dced7f697a90cd
