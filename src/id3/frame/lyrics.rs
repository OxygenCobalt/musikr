use std::fmt::{self, Display, Formatter};
use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct UnsyncLyricsFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    lyrics: String,
}

impl UnsyncLyricsFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> UnsyncLyricsFrame {
        let encoding = Encoding::from_raw(data[0]);

        let lang = string::get_string(Encoding::Utf8, &data[1..3]);
        let (desc, desc_size) = string::get_terminated_string(encoding, &data[4..]);

        let text_pos = 4 + desc_size;
        let lyrics = string::get_string(encoding, &data[text_pos..]);

        return UnsyncLyricsFrame {
            header,
            encoding,
            lang,
            desc,
            lyrics,
        };
    }
}

impl Id3Frame for UnsyncLyricsFrame {
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for UnsyncLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "\n{}", self.lyrics]
    }
}
