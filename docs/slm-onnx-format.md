# SLM ONNX Model Format

Mirage's local SLM router uses a split encoder/decoder ONNX export of
`bigscience/mt0-small`. This keeps the daemon small: the model is downloaded on
demand as a regular Mirage module and removed on uninstall.

## Expected module layout

The module is named `slm_nl_router`. After download, Mirage expects:

```text
<models_dir>/slm_nl_router/<version>/
  tokenizer.json
  encoder.onnx
  decoder.onnx
```

If the module ships a single `model.onnx` instead, the daemon logs an error
explaining that a split encoder/decoder export is required.

## Input/output contract

### `encoder.onnx`

Inputs:

| Name            | Shape       | Type | Description                 |
|-----------------|-------------|------|-----------------------------|
| `input_ids`     | `[1, S]`    | i64  | Token IDs from `tokenizer` |
| `attention_mask`| `[1, S]`    | i64  | 1 for real tokens, 0 for pad |

Outputs:

| Name                | Shape         | Type | Description            |
|---------------------|---------------|------|------------------------|
| `encoder_hidden_states` or `encoder_outputs` or `hidden_states` | `[1, S, H]` | f32 | Encoder hidden states |

### `decoder.onnx`

Inputs:

| Name                  | Shape       | Type | Description                         |
|-----------------------|-------------|------|-------------------------------------|
| `input_ids` or `decoder_input_ids` | `[1, T]` | i64 | Decoder input tokens (start with pad=0) |
| `attention_mask` or `decoder_attention_mask` | `[1, T]` | i64 | Causal attention mask (all 1s for greedy decoding) |
| `encoder_hidden_states` / `encoder_outputs` / `hidden_states` | `[1, S, H]` | f32 | Encoder hidden states |
| `encoder_attention_mask` / `encoder_mask` | `[1, S]` | i64 | Encoder attention mask |

Outputs:

| Name    | Shape         | Type | Description                 |
|---------|---------------|------|-----------------------------|
| `logits`| `[1, T, V]`   | f32  | Logits for every decoder position |

`S` = encoder sequence length, `T` = decoder sequence length, `H` = hidden size,
`V` = vocabulary size.

The decoder is used **without** a KV cache. Mirage runs one decoder forward per
generated token, feeding the full generated prefix each time. This is slower than a
cached loop but avoids shipping a separate cached-decoder ONNX and keeps the
Rust inference code simple.

## Exporting the model

Run the helper script from the repository root:

```bash
python scripts/export_mt0_small_onnx.py /tmp/mt0_onnx
```

Then copy the resulting files into the module staging area or upload them to the
Mirage catalog storage.

### Manual export with Optimum

If you prefer to run Optimum directly:

```bash
optimum-cli export onnx --model bigscience/mt0-small --task seq2seq-lm ./mt0_onnx
```

Rename the produced files:

```bash
mv ./mt0_onnx/encoder_model.onnx ./mt0_onnx/encoder.onnx
mv ./mt0_onnx/decoder_model.onnx ./mt0_onnx/decoder.onnx
rm -f ./mt0_onnx/decoder_with_past_model.onnx
```

The directory already contains `tokenizer.json` and `config.json` from the
export.

## Tokenizer handling

Mirage loads `tokenizer.json` with the `tokenizers` crate and maps the special
tokens:

- `<pad>` -> 0 (decoder start token)
- `</s>` -> 1 (end-of-sequence, used to stop generation)

If these tokens are not present in the tokenizer file, the engine falls back to
T5/mt0 defaults.

## Module manifest example

A minimal manifest entry for the `slm_nl_router` module looks like:

```yaml
id: slm_nl_router
name: Mirage SLM Natural Language Router
description: Routes natural-language questions to semantic search or DuckDB SQL.
version: "1.0.0"
archive:
  format: tar.gz
  url: https://cdn.example.com/mirage/modules/slm_nl_router-1.0.0.tar.gz
  sha256: <hex>
installed_size: 750000000
```

The archive should extract the three files shown in the layout above.
