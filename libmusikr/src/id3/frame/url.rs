use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use std::fmt::{self, Display, Formatter};

pub struct UrlFrame {
    header: Id3FrameHeader,
    url: String,
}

impl UrlFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> UrlFrame {
        let url = string::get_string(Encoding::Utf8, &data[0..]);

        UrlFrame { header, url }
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Id3Frame for UrlFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for UrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}

pub struct UserUrlFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    desc: String,
    url: String,
}

impl UserUrlFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> UserUrlFrame {
        let encoding = Encoding::from_raw(data[0]);

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[1..]);

        let text_pos = 1 + desc_size;
        let url = string::get_string(Encoding::Utf8, &data[text_pos..]);

        UserUrlFrame {
            header,
            encoding,
            desc,
            url,
        }
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

impl Id3Frame for UserUrlFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for UserUrlFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.url]
    }
}