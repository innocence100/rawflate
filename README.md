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
| JPEG | Huffman-coded DCT coefficients | [Lepton](https://github.com/microsoft/lepton_jpeg_rust) re-encoding (~22% smaller, byte-identical) |
| PNG | DEFLATE over filtered pixels | [preflate-rs](https://github.com/microsoft/preflate-rs) DEFLATE reconstruction → raw pixels + corrections (byte-identical) |

## Architecture — no Zstd pass-through

Standard preflate-rs wraps its output in a Zstd compression layer. rawflate uses a
patched version with `no_zstd: true`, emitting **raw container blocks** directly.
This avoids a wasteful Zstd compress+decompress cycle and lets zpaq compress the
raw pixel data directly.

The patch to preflate-rs is minimal (3 files, ~20 lines):

| File | Change |
|------|--------|
| `container/src/container_read.rs` | Decoder accepts raw (non-Zstd) blocks for all types |
| `container/src/container_common.rs` | `no_zstd: bool` config field |
| `container/src/container_write.rs` | `BlockBuf` enum to bypass Zstd encoder |

No API changes, no wire format changes. The raw blocks use `BLOCK_COMPRESSION_NONE`
instead of `BLOCK_COMPRESSION_ZSTD`.

## Usage

```bash
# Encode: JPEG/PNG → raw intermediate
rawflate -m encode -i photo.jpg -o photo.jpg.raw

# Decode: raw intermediate → original file (byte-identical)
rawflate -m decode -i photo.jpg.raw -o restored.jpg
```

The `.raw` files are designed to be fed to any general-purpose archiver for the final
compression pass. JPEG `.raw` files contain raw Lepton bytes; PNG `.raw` files contain
the preflate-rs container with raw (uncompressed) blocks.

## Building

Requires Rust ≥ 1.89 and a patched copy of preflate-rs.

```bash
# Clone patched fork alongside rawflate
git clone --branch raw-format https://github.com/innocence100/preflate-rs.git ../preflate-rs
cargo build --release
```

The CI workflow does this automatically — see `.github/workflows/build.yml`.
Prebuilt binaries for Linux, Windows, and macOS are available on the
[Releases](https://github.com/innocence100/rawflate/releases) page.

### preflate-rs patches

The 3-file patch set must be applied to any preflate-rs version used with rawflate:

1. Fork [microsoft/preflate-rs](https://github.com/microsoft/preflate-rs) → [innocence100/preflate-rs](https://github.com/innocence100/preflate-rs)
2. Apply the changes from the files listed above (decoder raw-block acceptance,
   `no_zstd` config, `BlockBuf` enum)
3. Push as branch `raw-format`, tag as `vX.Y.Z-raw`
4. Put `sync-upstream.yml` in `.github/workflows/` for automatic syncing

## License

MIT — see [LICENSE](LICENSE).
