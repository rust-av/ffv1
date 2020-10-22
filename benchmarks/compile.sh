#!/bin/sh

# Compile Go binary stripping debug symbols (release mode)
cd go-ffv1 &&
go get github.com/dwbuiten/matroska github.com/dwbuiten/go-ffv1/ffv1 &&
go build -ldflags "-s -w" -o go-ffv1 .

# Compile Rust binary in release mode
cd rust-ffv1 && cargo build --release

# Compile C binary in optimized mode
cd c-ffv1 && gcc -03 main.c -o c-ffv1 \
                 `pkg-config --cflags --libs libavutil libavcodec libavformat`
