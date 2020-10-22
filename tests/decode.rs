// ffv1 crate
extern crate ffv1;

// rust-av crates
extern crate av_data as data;
extern crate av_format as format;

// Matroska demuxer
extern crate matroska;

use std::fs::File;

use data::params::MediaKind;
use format::buffer::AccReader;
use format::demuxer::{Context, Event};

use matroska::demuxer::MkvDemuxer;

use ffv1::decoder::Decoder;

// ffv1 decoder parameters
#[derive(Default)]
struct DecParams {
    width: u32,
    height: u32,
    extradata: Vec<u8>,
}

// Decodes a single ffv1 frame
fn decode_single_frame(
    demuxer: &mut Context,
    decoder: &mut Decoder,
) -> Result<ffv1::decoder::Frame, String> {
    // The demuxer reads which event has occurred
    match demuxer.read_event() {
        // If a new packet has been found, decode it
        Ok(event) => match event {
            Event::NewPacket(pkt) => {
                // Reads a ffv1 frame
                decoder
                    .decode_frame(&pkt.data)
                    .map_err(|_e| "Decoding failure".to_owned())
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

#[test]
fn test_1() {
    // Open the matroska file
    let f = "data/ffv1_v3.mkv";
    let reader = File::open(f).unwrap();

    // Create a buffer of size 4096MB to contain matroska data
    let ar = AccReader::with_capacity(4 * 1024, reader);

    // Set the type of demuxer, in this case, a matroska demuxer
    let mut demuxer = Context::new(Box::new(MkvDemuxer::new()), Box::new(ar));

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
            if String::from_utf8_lossy(&extradata).contains("FFV1") {
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

    // Iterate over the decoded frames
    let frame = decode_single_frame(&mut demuxer, &mut ffv1_decoder).unwrap();

    use std::io::Read;
    let r1 = File::open("data/ref_1.raw").unwrap().bytes();

    let yplane = frame.buf[0].iter();
    let uplane = frame.buf[1].iter();
    let vplane = frame.buf[2].iter();

    let planes = yplane.chain(uplane).chain(vplane);

    for (&p, r) in planes.zip(r1) {
        assert_eq!(p, r.unwrap());
    }
}
