//! Musikr is an audio metadata reading/writing library primarily built for the `musikr-cli`
//! program. The library aims to provide a powerful, low-level interface for tagging audio
//! metadata while also remaining consistent, tested, and fuzz-resistant.

#![forbid(unsafe_code)]

#[macro_use]
mod core;

pub mod id3v2;
pub mod string;
