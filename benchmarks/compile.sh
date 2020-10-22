#!/bin/sh

mkdir -p builds

# Compile Go binary stripping debug symbols (release mode)
pushd go-ffv1
go get github.com/dwbuiten/matroska github.com/dwbuiten/go-ffv1/ffv1 &&
go build -ldflags "-s -w" -o ../builds/go-ffv1 .
popd

# Compile Rust binary in release mode
pushd rust-ffv1
cargo build --release
cp target/release/rust-ffv1 ../builds
popd

# Compile C binary in optimized mode
pushd c-ffv1
cc -O3 main.c -o ../builds/c-ffv1 `pkg-config --cflags --libs libavutil libavcodec libavformat`
popd
