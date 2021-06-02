use std::fmt::{self, Display, Formatter};
use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct GeneralObjectFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: String,
    filename: String,
    desc: String,
    data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> GeneralObjectFrame {
        let encoding = Encoding::from_raw(data[0]);

        let (mime, mime_size) = string::get_terminated_string(encoding, &data[1..]);
        let mut pos = mime_size + 1;

        let (filename, fn_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += fn_size;

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc_size;

        let data = data[pos..].to_vec();

        return GeneralObjectFrame {
            header,
            encoding,
            mime,
            filename,
            desc,
            data,
        };
    }

    fn mime(&self) -> &String {
        return &self.mime;
    }

    fn filename(&self) -> &String {
        return &self.filename;
    }

    fn desc(&self) -> &String {
        return &self.desc;
    }

    fn data(&self) -> &Vec<u8> {
        return &self.data;
    }
}

impl Id3Frame for GeneralObjectFrame {
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for GeneralObjectFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.mime.is_empty() {
            write![f, "{} ", self.mime]?;
        }

        if !self.filename.is_empty() {
            write![f, "\"{}\"", self.filename]?;
        }

        if !self.desc.is_empty() {
            write![f, " [{}]", self.desc]?;
        }

        return Ok(());
    }
}
