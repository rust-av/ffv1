use std::fs::File;
use std::io::Read;

use av_data::params::MediaKind;
use av_format::buffer::AccReader;
use av_format::demuxer::{Context, Event};

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

fn decode(input: &str) -> ffv1::decoder::Frame {
    let reader = File::open(input).unwrap();

    // Create a buffer of size 4096KiB to contain matroska data
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

    decode_single_frame(&mut demuxer, &mut ffv1_decoder).unwrap()
}

#[test]
fn test_yuv420() {
    let input = "data/ffv1_v3_yuv420p.mkv";
    let reference = "data/ffv1_v3_yuv420p.ref";
    let f = File::open(reference).unwrap();
    let frame = decode(input);

    let yplane = frame.buf[0].iter();
    let uplane = frame.buf[1].iter();
    let vplane = frame.buf[2].iter();

    let pixels = yplane.chain(uplane).chain(vplane);

    for (i, (&p, r)) in pixels.zip(f.bytes()).enumerate() {
        assert_eq!(p, r.unwrap(), "pixel {}", i);
    }
}

#[test]
fn test_bgr0() {
    let input = "data/ffv1_v3_bgr0.mkv";
    let reference = "data/ffv1_v3_bgr0.ref";

    let mut f = File::open(reference).unwrap();
    let frame = decode(input);

    let gplane = frame.buf[0].iter();
    let bplane = frame.buf[1].iter();
    let rplane = frame.buf[2].iter();

    let pixels = gplane.zip(bplane).zip(rplane);

    for (i, p) in pixels.enumerate() {
        let mut reference_pixel = [0u8; 4];
        f.read_exact(&mut reference_pixel).unwrap();

        let [r_b, r_g, r_r, _] = reference_pixel;
        let ((&p_g, &p_b), &p_r) = p;

        assert_eq!((p_r, p_g, p_b), (r_r, r_g, r_b), "pixel {}", i);
    }
}

#[test]
fn test_gbrp16le() {
    use byteorder::{LittleEndian, ReadBytesExt};
    let input = "data/ffv1_v3_gbrp16le.mkv";
    let reference = "data/ffv1_v3_gbrp16le.ref";

    let mut f = File::open(reference).unwrap();
    let frame = decode(input);

    let gplane = frame.buf16[0].iter();
    let bplane = frame.buf16[1].iter();
    let rplane = frame.buf16[2].iter();

    let pixels = gplane.chain(bplane).chain(rplane);

    for (i, &p) in pixels.enumerate() {
        let r = f.read_u16::<LittleEndian>().unwrap();
        assert_eq!(p, r, "pixel {}", i);
    }
}
