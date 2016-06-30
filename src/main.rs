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
        if let Ok(c_src_uri) = ffi::CString::new(src_uri.to_string()) {
            // Open video file
            if ffsys::avformat_open_input(&mut src_avfc.0, c_src_uri.as_ptr(), ptr::null(), ptr::null_mut()) != 0 {
                println!("Could not open media file {}", src_uri);
                process::exit(-1);
            }
        } else {
            println!("Could not convert the source URI to a C String.");
            process::exit(-1);
        }
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