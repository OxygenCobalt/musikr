#![no_main]

use std::fs::File;
use std::path::Path;
use std::io::Write;
use musikr::id3v2::Tag;
use libfuzzer_sys::fuzz_target;

const PATH: &str = "/tmp/fuzz.mp3";

fuzz_target!(|data: &[u8]| {
    // Musikr's only input surface is with files, so we write our data to a file in /tmp/
    // memory and then write our random bytes to it.
    // Fuzzing only works on *nix right now, so this is okay.

    let path = Path::new(&PATH);

    let mut file = File::create(path).unwrap();
    file.write_all(data).unwrap();

    let _ = Tag::open(PATH);
});