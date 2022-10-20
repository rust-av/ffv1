//! This example decodes a ffv1 codec contained in a matroska file.

// ffv1 crate
extern crate ffv1;

// rust-av crates
extern crate av_data as data;
extern crate av_format as format;

// Matroska demuxer
extern crate matroska;

// CLI crates
extern crate clap;

// Byteorder crate
extern crate byteorder;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use data::params::MediaKind;
use format::buffer::{AccReader, Buffered};
use format::demuxer::{Context, Event, Demuxer};

use matroska::demuxer::MkvDemuxer;

use ffv1::decoder::{Decoder, Frame};

use byteorder::{LittleEndian, WriteBytesExt};
use clap::{App, Arg};

// ffv1 decoder parameters
#[derive(Default)]
struct DecParams {
    width: u32,
    height: u32,
    extradata: Vec<u8>,
}

// Writes a u16 buffer as little endian on a file.
#[inline(always)]
fn write_u16_le<W: Write>(
    file: &mut BufWriter<W>,
    buf16: &[u16],
) -> std::io::Result<()> {
    for &v in buf16 {
        file.write_u16::<LittleEndian>(v)?
    }
    Ok(())
}

// Decodes a single ffv1 frame
fn decode_single_frame(
    demuxer: &mut Context<impl Demuxer, impl Buffered>,
    decoder: &mut Decoder,
    extradata: &[u8],
) -> Result<Frame, String> {
    // The demuxer reads which event has occurred
    match demuxer.read_event() {
        // If a new packet has been found, decode it
        Ok(event) => match event {
            Event::NewPacket(pkt) => {
                println!(
                    "extradata = {} packet = {} track = {}\n",
                    extradata.len(),
                    pkt.data.len(),
                    pkt.pos.unwrap_or(0)
                );
                // Reads a ffv1 frame
                let frame = decoder.decode_frame(&pkt.data).unwrap();
                println!(
                    "Frame decoded at {}x{}\n",
                    frame.width, frame.height
                );
                Ok(frame)
            }
            // When the EOF is reached, the decoding process is stopped
            Event::Eof => {
                println!("EOF reached.");
                Err("EOF reached".to_owned())
            }
            _ => {
                // If an unsupported event occurs,
                // the decoding process is stopped
                println!("Unsupported event {:?}", event);
                Err("Unsupported event".to_owned())
            }
        },
        Err(err) => {
            // If there are no more events, the decoding process is stopped
            println!("No more events {:?}", err);
            Err("No more events".to_owned())
        }
    }
}

fn main() -> std::io::Result<()> {
    // Set up CLI configuration and input parameters
    let matches = App::new("ffv1-decode")
        .about("Decodes a ffv1 codec contained in a matroska file")
        .arg(
            Arg::new("input-path")
                .help("Matroska file to analyze")
                .short('i')
                .long("input")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("output-path")
                .help("Output file")
                .short('o')
                .long("output")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    // Get the path to the matroska file
    let input_path = matches.value_of("input-path").map(Path::new).unwrap();

    // Get the path to the output file
    let output_path = matches.value_of("output-path").map(Path::new).unwrap();

    // Open the matroska file
    let reader = File::open(input_path).unwrap();

    // Create a buffer of size 4096MB to contain matroska data
    let ar = AccReader::with_capacity(4 * 1024, reader);

    // Set the type of demuxer, in this case, a matroska demuxer
    let mut demuxer = Context::new(MkvDemuxer::new(), ar);

    // Read matroska headers
    demuxer
        .read_headers()
        .expect("Cannot parse the format headers");

    // Save decoder params for ffv1 decoder
    let mut decoder_params: DecParams = Default::default();

    // Iterate over the streams contained in a matroska file
    for stream in &demuxer.info.streams {
        // Considers only video streams and analyze the type of codec inside.
        if let Some(MediaKind::Video(info)) = &stream.params.kind {
            let extradata =
                stream.params.extradata.as_ref().unwrap_or_else(|| {
                    eprintln!("No extradata detected. Aborting");
                    std::process::exit(1);
                });
            if String::from_utf8_lossy(extradata).contains("FFV1") {
                decoder_params.width = info.width as u32;
                decoder_params.height = info.height as u32;
                // As per Matroska spec for VFW CodecPrivate
                decoder_params.extradata = extradata[40..].to_owned();
            }
        }
    }

    // Create a new ffv1 decoder
    let mut ffv1_decoder = Decoder::new(
        &decoder_params.extradata,
        decoder_params.width,
        decoder_params.height,
    )
    .unwrap();

    // Open raw file
    let mut output_file = BufWriter::new(File::create(output_path).unwrap());

    // Iterate over the decoded frames
    while let Ok(frame) = decode_single_frame(
        &mut demuxer,
        &mut ffv1_decoder,
        &decoder_params.extradata,
    ) {
        if frame.bit_depth == 8 {
            output_file.write_all(&frame.buf[0])?;
            output_file.write_all(&frame.buf[1])?;
            output_file.write_all(&frame.buf[2])?;
        } else {
            write_u16_le(&mut output_file, &frame.buf16[0])?;
            write_u16_le(&mut output_file, &frame.buf16[1])?;
            write_u16_le(&mut output_file, &frame.buf16[2])?;
        }
    }
    println!("Done.");
    Ok(())
}
