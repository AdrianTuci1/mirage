#!/usr/bin/env python3
"""Export `bigscience/mt0-small` to split encoder/decoder ONNX files for Mirage.

This script uses Hugging Face Optimum to produce the layout expected by
`src/daemon_next/src/slm/onnx.rs`:

    <output_dir>/tokenizer.json
    <output_dir>/encoder.onnx
    <output_dir>/decoder.onnx

The encoder ONNX accepts `input_ids` and `attention_mask` and emits the
encoder hidden states. The decoder ONNX accepts `input_ids`,
`attention_mask`, `encoder_hidden_states` and `encoder_attention_mask`
and emits `logits`. The decoder is exported **without** a KV cache so
Mirage can run greedy token-by-token decoding with a plain session per step.

Install dependencies before running:

    pip install optimum[onnxruntime] transformers torch

Example:

    python scripts/export_mt0_small_onnx.py /tmp/mt0_onnx
"""

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


MODEL_ID = "bigscience/mt0-small"


def run_optimum_cli(output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        sys.executable,
        "-m",
        "optimum.exporters.onnx",
        "--model",
        MODEL_ID,
        "--task",
        "seq2seq-lm",
        str(output_dir),
    ]
    print(f"Running: {' '.join(cmd)}")
    subprocess.run(cmd, check=True)


def rename_optimum_outputs(output_dir: Path) -> None:
    """Rename Optimum's default ONNX names to the names Mirage expects."""
    source_encoder = output_dir / "encoder_model.onnx"
    source_decoder = output_dir / "decoder_model.onnx"
    target_encoder = output_dir / "encoder.onnx"
    target_decoder = output_dir / "decoder.onnx"

    if not source_encoder.exists():
        raise FileNotFoundError(
            f"Expected Optimum encoder output at {source_encoder}; "
            "check that the export succeeded and adjust this script."
        )
    if not source_decoder.exists():
        raise FileNotFoundError(
            f"Expected Optimum decoder output at {source_decoder}; "
            "check that the export succeeded and adjust this script."
        )

    shutil.move(str(source_encoder), str(target_encoder))
    shutil.move(str(source_decoder), str(target_decoder))

    # The no-cache decoder is enough for Mirage; remove the cached variant if present.
    decoder_with_past = output_dir / "decoder_with_past_model.onnx"
    if decoder_with_past.exists():
        decoder_with_past.unlink()
        print(f"Removed {decoder_with_past} (not needed by Mirage).")


def verify_inputs_outputs(output_dir: Path) -> None:
    try:
        import onnx
    except ImportError:
        print("Note: install `onnx` to verify model input/output names.")
        return

    for name in ("encoder.onnx", "decoder.onnx"):
        path = output_dir / name
        model = onnx.load(str(path))
        inputs = [inp.name for inp in model.graph.input]
        outputs = [out.name for out in model.graph.output]
        print(f"{name}: inputs={inputs}, outputs={outputs}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Export bigscience/mt0-small to ONNX for Mirage SLM routing."
    )
    parser.add_argument(
        "output_dir",
        type=Path,
        help="Directory where tokenizer.json, encoder.onnx and decoder.onnx will be written.",
    )
    args = parser.parse_args()

    run_optimum_cli(args.output_dir)
    rename_optimum_outputs(args.output_dir)
    verify_inputs_outputs(args.output_dir)
    print(f"Export complete: {args.output_dir}")


if __name__ == "__main__":
    main()
