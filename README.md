# A Rust FFV1 Decoder

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Actions Status](https://github.com/Luni-4/ffv1/workflows/ffv1/badge.svg)](https://github.com/Luni-4/ffv1/actions)
[![Coverage Status](https://coveralls.io/repos/github/rust-av/ffv1/badge.svg?branch=integration_tests)](https://coveralls.io/github/rust-av/ffv1?branch=integration_tests)

A pure-Rust FFV1 decoder based on this [Go](https://github.com/dwbuiten/go-ffv1)
implementation. A great and personal thanks to
[@dwbuiten](https://github.com/dwbuiten) for developing it and all
[FFV1](https://github.com/FFmpeg/FFV1) people involved in writing the
relative specifications.

This project has been developed with the aim of testing parallelism.

## Building

Debug mode:

```bash
cargo build
```

Release mode:

```bash
cargo build --release
```

## Building examples

```bash
cargo build --release --examples
```

## Run example

```bash
cargo run --release --example EXAMPLE_NAME -- -i INPUT_FILEPATH -o OUTPUT_FILEPATH
```

For example, if you want to run the `ffv1_decode` example:

```bash
cargo run --release --example ffv1_decode -- -i input.mkv -o output.raw
```

You can reproduce your raw file with `ffplay` from `FFmpeg` specifying
the video parameters associated to the `raw` output file.

For example, if we consider the output produced using `ffv1_v3.mkv`, called
`ffv1_v3.raw`, you should run the following command:

```bash
ffplay -f rawvideo -pixel_format yuv420p -video_size 640x360 -framerate 25 output.raw
```

## Notes

The code is still in flux and pretty messed up. No parallelism has been
implemented for now, so the library is pretty slow.

## License

Released under the [MIT License](LICENSE).
