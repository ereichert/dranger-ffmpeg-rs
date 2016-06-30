An experimentation project, loosley based on the Dranger FFMPEG tutorials http://dranger.com/ffmpeg/ ported to Rust.

It's an easy way to test ideas for working with FFMPEG in Rust without a bunch of stuff in the way. 

It includes some light abstraction but a production application would likely have more to make use of rust-ffmpeg-sys safer.

For an example of a safe wrapper around rust-ffmpeg-sys see https://github.com/meh/rust-ffmpeg.