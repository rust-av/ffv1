# Benchmarks

This directory contains the code to benchmark three different `ffv1` versions
written in the following programming languages:

- Rust
- Go
- C

## Environment

Benchmarks are thought to run on `Linux` systems.

You need to install `Go`, `Rust` and `C` toolchains in order to
build the executables.

In case of the `C` executable, you also have to install
`libavformat`, `libavcodec`, `libavutil` from `FFmpeg`.

The code is benchmarked using [hyperfine](https://github.com/sharkdp/hyperfine).
Take a look at the [Releases](https://github.com/sharkdp/hyperfine/releases)
page to install the version best suited for your architecture.

## Building

To build executables, please run the `./compile.sh` script.

## Run benchmark

To run benchmarks, please run the `/.benchmarks.sh` script and have a coffee :)
