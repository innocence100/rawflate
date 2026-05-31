# rawflate

Lossless image preprocessor for archiving — unwraps JPEG/PNG internal compression,
outputs a raw intermediate format that compresses dramatically better under
general-purpose archivers (zpaq, 7z, etc.), then restores the original file
**byte-for-byte**.

## Why

JPEG and PNG are already compressed — feeding them directly to zpaq/7z yields almost
no savings. But their internal compression (Huffman/DEFLATE) is far from optimal for
long-term cold storage.

rawflate **unwraps** that internal compression layer:

| Format | Internals | rawflate does |
|--------|-----------|--------------|
| JPEG | Huffman-coded DCT coefficients | [Lepton](https://github.com/microsoft/lepton_jpeg_rust) re-encoding (~22% smaller, byte-identical round-trip) |
| PNG | DEFLATE over filtered pixels | [preflate-rs](https://github.com/microsoft/preflate-rs) DEFLATE reconstruction → raw pixels + corrections (byte-identical round-trip) |

### Output format

The intermediate file has a 5-byte header followed by format-specific payload:

- **JPEG** (type `0x01`): raw Lepton bytes — no further wrapping.
- **PNG** (type `0x02`): processed through preflate-rs container format with Zstd level 1.
  The Zstd overhead is negligible (~1%) but required by the preflate-rs decoder for
  reconstruction. The heavy lifting — DEFLATE decompression and CABAC correction encoding —
  is already done; Zstd is a thin compatibility wrapper.

## Usage

```bash
# Encode: JPEG/PNG → raw intermediate
rawflate -m encode -i photo.jpg -o photo.jpg.raw

# Decode: raw intermediate → original file (byte-identical)
rawflate -m decode -i photo.jpg.raw -o restored.jpg
```

The `.raw` files are designed to be fed to any general-purpose archiver for the final
compression pass.

## Building

Requires Rust ≥ 1.89.

```bash
cargo build --release
```
