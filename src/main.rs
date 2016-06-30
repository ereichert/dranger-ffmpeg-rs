extern crate ffmpeg_sys;

use std::{ptr, process, env, ffi};
use ffmpeg_sys as ffsys;

fn main() {
    println!("Starting");
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("Please specify a media file.");
        process::exit(-1);
    }
    let ref src_uri = args[1];
    unsafe {
        // Register all formats, codecs, and network features.
        ffsys::av_register_all();
        ffmpeg_sys::avformat_network_init();

        let mut src_avfc = AVFC::new();
        let possible_c_src_uri = ffi::CString::new(src_uri.to_string());
        if let Ok(ref c_src_uri) = possible_c_src_uri {
            // Open video file
            if ffsys::avformat_open_input(&mut src_avfc.0, c_src_uri.as_ptr(), ptr::null(), ptr::null_mut()) != 0 {
                println!("Could not open media file {}", src_uri);
                process::exit(-1);
            }
        } else {
            println!("Could not convert the source URI to a C String.");
            process::exit(-1);
        }

        println!("Opened media file {}.", src_uri);

        // Retrieve stream information
        if ffsys::avformat_find_stream_info(src_avfc.0, ptr::null_mut()) < 0 {
            println!("Could not find the stream info.");
            process::exit(-1);
        }
        println!("Found stream info for {}.", src_uri);

        // Dump information about file
        ffsys::av_dump_format(src_avfc.0, 0, possible_c_src_uri.unwrap().as_ptr(), 0);

        // Find the first video stream
        let mut stream_idx = 0;
        let avfc_deref = &*src_avfc.0;
        let num_streams = avfc_deref.nb_streams;
        let streams = avfc_deref.streams;
        if let Some(idx) = (0..num_streams as u32).find(|x| {
            let av_stream_deref = &**streams.offset(*x as isize);
            let av_codec_ctx_deref = &*av_stream_deref.codec;  
            av_codec_ctx_deref.codec_type == ffsys::AVMEDIA_TYPE_VIDEO
        }) {
            println!("Found video stream at stream index {}.", idx);
            stream_idx = idx;
        } else {
            println!("Could not a video stream.");
            process::exit(-1);
        }

        // Get a pointer to the codec context for the video stream
        let stream = &**streams.offset(stream_idx as isize);
        let avcc = AVCC::new(stream.codec);

        println!("Retrieved AVCodecContext for {}.", src_uri);
    }

    process::exit(0);
}

struct AVFC(*mut ffsys::AVFormatContext);

impl AVFC {

    fn new() -> AVFC {
        AVFC(ptr::null_mut())
    }
}

impl Drop for AVFC {
    
    fn drop(&mut self) -> () {
        if !self.0.is_null() {
            unsafe {
                println!("Dropping the AVFC.");
                ffsys::avformat_close_input(&mut self.0);
            }
        }
    }
}

struct AVCC(*mut ffsys::AVCodecContext);

impl AVCC {

    fn new(avcc: *mut ffsys::AVCodecContext) -> AVCC {
        AVCC(avcc)
    }
}

impl Drop for AVCC {

    fn drop(&mut self) -> () {
        if !self.0.is_null() {
            unsafe {
                println!("Dropping the AVCC.");
                ffsys::avcodec_close(self.0);
            }
        }
    }
}