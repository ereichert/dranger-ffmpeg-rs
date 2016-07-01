#![feature(alloc_system)]
extern crate alloc_system;

extern crate ffmpeg_sys;

use std::{ptr, process, env, ffi, mem, fs, slice};
use ffmpeg_sys as ffsys;
use std::io::Write;

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
        if let Some(idx) = (0..num_streams as i32).find(|x| {
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
        let avcc = AVCC::wrap(stream.codec);

        println!("Retrieved AVCodecContext for {}.", src_uri);

        // Find the decoder for the video stream
        let codec_id = (*avcc.avcc).codec_id;
        let avc = ffsys::avcodec_find_decoder(codec_id);
        if avc.is_null() {
            println!("Could not find a codec for codec id {:?}.", codec_id);
            process::exit(-1);
        }

        println!("Found codec {:?}.", ffi::CStr::from_ptr((*avc).name));

        // Open codec
        if ffsys::avcodec_open2(avcc.avcc, avc, ptr::null_mut()) < 0 {
            println!("Could not open the codec context for {}.", src_uri);
            process::exit(-1);
        } 
        
        println!("Opened codec context for {}.", src_uri);

        let mut src_frame = AVF::new();
        let mut packet = AVP::new();
        let mut frame_finished = 0;
        let mut frame_num = 0;
        while ffsys::av_read_frame(src_avfc.0, packet.as_mut_ptr()) >=0 && frame_num < 5 {
            // Is this a packet from the video stream?
            if packet.stream_index() == stream_idx {
                // Decode video frame
                ffsys::avcodec_decode_video2(avcc.avcc, src_frame.as_mut_ptr(), &mut frame_finished, packet.as_ptr());
                // Did we get a video frame?
                if frame_finished != 0 {
                    println!("Saving frame {} to disk.", frame_num + 1);
                    // Save the frame to disk
                    save_avframe_to_jpeg(&src_frame, frame_num + 1);
                    frame_num += 1;
                }
            }

            // Free the packet that was allocated by av_read_frame
            packet.free();
        }
    }

    process::exit(0);
}

unsafe fn save_avframe_to_jpeg(src_frame: &AVF, frame_num: i32) -> i32 {
    let mjpeg_codec = ffsys::avcodec_find_encoder(ffsys::AV_CODEC_ID_MJPEG);
    if mjpeg_codec.is_null() {
        println!("could not find mjpeg codec");
        return -1;
    }

    println!("Found MJPEG codec.");

    let jpeg_avcc = AVCC::new(mjpeg_codec);
    if jpeg_avcc.avcc.is_null() {
        println!("could not allocate jpeg codec context");
        return -1;
    }

    println!("Allocated the MJPEG codec context.");

    let ref mut jpeg_avcc_deref = *jpeg_avcc.avcc;
    let ref src_frame_deref = *src_frame.0;
    jpeg_avcc_deref.pix_fmt = ffsys::AV_PIX_FMT_YUVJ420P;
    jpeg_avcc_deref.height = src_frame_deref.height;
    jpeg_avcc_deref.width = src_frame_deref.width;
    jpeg_avcc_deref.time_base.num = 1;
    jpeg_avcc_deref.time_base.den = 1000000;

    println!("height = {}", src_frame_deref.height);
    println!("width = {}", src_frame_deref.width);

    println!("before opening the jpeg context");
    if ffsys::avcodec_open2(jpeg_avcc.avcc, mjpeg_codec, ptr::null_mut()) < 0 {
        println!("could not open jpeg codec context");
        return -1;
    }

    let mut packet = AVP::new_with_size(src_frame_deref.height * src_frame_deref.width);

    let mut frame_complete = 0;
    println!("before encoding the jpeg");
    if ffsys::avcodec_encode_video2(jpeg_avcc.avcc, packet.as_mut_ptr(), src_frame.as_ptr(), &mut frame_complete) < 0 {
        println!("could not encode jpeg");
        return -1;
    }

    let file_open_result = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("sample-{}.jpg", frame_num));

    if file_open_result.is_err() {
        println!("failed to open file for writing the JPEG");
        return -1;
    }

    let mut file = file_open_result.unwrap();
    println!("Opened file for writing.");
        
    let data = slice::from_raw_parts(packet.0.data as *const u8, packet.0.size as usize);
    let write_result = file.write(data);
    
    if write_result.is_err() {
        println!("failed to write the JPEG");
        return -1;
    }
    
    0
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

struct AVCC{
    avcc: *mut ffsys::AVCodecContext,
    wrapped: bool,
}

impl AVCC {

    unsafe fn new(avc: *mut ffsys::AVCodec) -> AVCC {
        AVCC {
            avcc: ffsys::avcodec_alloc_context3(avc),
            wrapped: false,
        }
    }

    fn wrap(avcc: *mut ffsys::AVCodecContext) -> AVCC {
        AVCC {
            avcc: avcc,
            wrapped: true,
        }
    }
}

impl Drop for AVCC {

    fn drop(&mut self) -> () {
        if !self.avcc.is_null() {
            unsafe {
                println!("Dropping the AVCC.");
                if self.wrapped {
                    ffsys::avcodec_close(self.avcc);
                } else {
                    ffsys::avcodec_free_context(&mut self.avcc);
                }
            }
        }
    }
}

pub struct AVF(*mut ffsys::AVFrame);

impl AVF {

    pub unsafe fn new() -> AVF {
        let av_frame = ffsys::av_frame_alloc();
        AVF(av_frame)
    }

    pub fn as_ptr(&self) -> *const ffsys::AVFrame {
        self.0
    }

    pub fn as_mut_ptr(&mut self) -> *mut ffsys::AVFrame {
        self.0
    }
}

impl Drop for AVF {
    
    fn drop(&mut self) -> () {
        unsafe {
            println!("Dropping the AVF.");
            ffsys::av_frame_free(&mut self.as_mut_ptr());
        }
    }
}

pub struct AVP(ffsys::AVPacket);

impl AVP {

    pub unsafe fn new() -> AVP {
        let mut pkt: ffsys::AVPacket = mem::zeroed();
        ffsys::av_init_packet(&mut pkt);
        AVP(pkt)
    }

    pub unsafe fn new_with_size(size: i32) -> AVP {
        let mut pkt: ffsys::AVPacket = mem::zeroed();
        ffsys::av_new_packet(&mut pkt, size);
        AVP(pkt)
    }

    pub fn as_ptr(&self) -> *const ffsys::AVPacket {
        &self.0 as *const ffsys::AVPacket
    }

    pub fn as_mut_ptr(&mut self) -> *mut ffsys::AVPacket {
        &mut self.0 as *mut ffsys::AVPacket
    }

    pub fn stream_index(&self) -> i32 {
        self.0.stream_index
    }

    pub unsafe fn free(&mut self) -> () {
        println!("Dropping the AVP.");
        ffsys::av_packet_unref(self.as_mut_ptr());
    }
}

impl Drop for AVP {
    
    fn drop(&mut self) -> () {
        unsafe {
            self.free()
        }
    }
}