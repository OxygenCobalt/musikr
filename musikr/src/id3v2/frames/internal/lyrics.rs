use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::time::{Timestamp, TimestampFormat};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct UnsyncLyricsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    lyrics: String,
}

impl UnsyncLyricsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("USLT", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        UnsyncLyricsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            desc: String::new(),
            lyrics: String::new(),
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn lyrics(&self) -> &String {
        &self.lyrics
    }
}

impl Frame for UnsyncLyricsFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 5 {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = string::get_string(Encoding::Latin1, &data[1..4]);

        let desc = string::get_terminated_string(self.encoding, &data[4..]);
        self.desc = desc.string;

        let text_pos = 4 + desc.size;
        self.lyrics = string::get_string(self.encoding, &data[text_pos..]);

        Ok(())
    }
}

impl Display for UnsyncLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.desc.is_empty() {
            write![f, "\n{}:", self.desc]?;
        }

        write![f, "\n{}", self.lyrics]
    }
}

impl Default for UnsyncLyricsFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

pub struct SyncedLyricsFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    time_format: TimestampFormat,
    content_type: SyncedContentType,
    desc: String,
    lyrics: Vec<SyncedText>,
}

impl SyncedLyricsFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("SYLT", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        SyncedLyricsFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            time_format: TimestampFormat::default(),
            content_type: SyncedContentType::default(),
            desc: String::new(),
            lyrics: Vec::new(),
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn time_format(&self) -> TimestampFormat {
        self.time_format
    }

    pub fn content_type(&self) -> SyncedContentType {
        self.content_type
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn lyrics(&self) -> &Vec<SyncedText> {
        &self.lyrics
    }
}

impl Frame for SyncedLyricsFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}:{}", self.id(), self.desc, self.lang]
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 6 {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = String::from_utf8_lossy(&data[1..4]).to_string();
        self.time_format = TimestampFormat::new(data[5]);
        self.content_type = SyncedContentType::new(data[6]);
        let desc = string::get_terminated_string(self.encoding, &data[7..]);
        self.desc = desc.string;

        // For UTF-16 Synced Lyrics frames, a tagger might only write the BOM to the description
        // and nowhere else. If thats the case, we will subsitute the generic Utf16 encoding for
        // the implicit encoding if there is no bom in each lyric.

        let implicit_encoding = match self.encoding {
            Encoding::Utf16 => {
                let bom = raw::to_u16(&data[7..9]);

                match bom {
                    0xFFFE => Encoding::Utf16Le,
                    0xFEFF => Encoding::Utf16Be,
                    _ => self.encoding,
                }
            }

            _ => self.encoding,
        };

        let mut pos = desc.size + 7;

        while pos < data.len() {
            let bom = raw::to_u16(&data[pos..pos + 2]);

            // If the lyric does not have a BOM, use the implicit encoding we got earlier.
            let enc = if bom != 0xFEFF && bom != 0xFFFE {
                implicit_encoding
            } else {
                self.encoding
            };

            let text = string::get_terminated_string(enc, &data[pos..]);
            pos += text.size;

            let timestamp = Timestamp::new(self.time_format, raw::to_u32(&data[pos..pos + 4]));
            pos += 4;

            self.lyrics.push(SyncedText {
                text: text.string,
                timestamp,
            })
        }

        Ok(())
    }
}

impl Display for SyncedLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Append a brief header if we have a description, otherwise we omit the content type
        // altogether since it only really works in conjunction with a description
        if !self.desc.is_empty() {
            write![f, "\n\"{}\" [{:?}]:", self.desc, self.content_type]?;
        }

        for lyric in &self.lyrics {
            write![f, "\n{}", lyric]?;
        }

        Ok(())
    }
}

impl Default for SyncedLyricsFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

byte_enum! {
    pub enum SyncedContentType {
        Other = 0x00,
        Lyrics = 0x01,
        TextTranscription = 0x02,
        Movement = 0x03,
        Events = 0x04,
        Chord = 0x05,
        Trivia = 0x06,
        WebpageUrls = 0x07,
        ImageUrls = 0x08,
    }
}

impl Default for SyncedContentType {
    fn default() -> Self {
        SyncedContentType::Other
    }
}

pub struct SyncedText {
    pub text: String,
    pub timestamp: Timestamp,
}

impl Display for SyncedText {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Since we are formatting this already, strip any trailing newlines from the lyrics
        // using a somewhat clunky if block.
        let text = if self.text.starts_with('\n') {
            self.text
                .strip_prefix("\r\n")
                .or_else(|| self.text.strip_prefix("\n"))
                .unwrap_or(&self.text)
        } else if self.text.ends_with('\n') {
            self.text
                .strip_suffix("\r\n")
                .or_else(|| self.text.strip_suffix("\n"))
                .unwrap_or(&self.text)
        } else {
            &self.text
        };

        // Don't include the timestamp, as formatting time is beyond the scope of libmusikr
        write![f, "{}", text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unsync_lyrics() {
        let data = b"\x00\
                     eng\
                     Description\0\
                     Jumped in the river, what did I see?\n\
                     Black eyed angels swam with me\n";

        let mut frame = UnsyncLyricsFrame::new();
        frame.parse(&TagHeader::new_test(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.desc(), "Description");
        assert_eq!(
            frame.lyrics(),
            "Jumped in the river, what did I see?\n\
                                    Black eyed angels swam with me\n"
        )
    }

    #[test]
    fn parse_synced_lyrics() {
        let data = b"\x03\
                     eng\0\
                     \x02\x01\
                     Description\0\
                     You don't remember, you don't remember\n\0\
                     \x00\x02\x78\xD0\
                     Why don't you remember my name?\n\0\
                     \x00\x02\x88\x70";

        let mut frame = SyncedLyricsFrame::new();
        frame.parse(&TagHeader::new_test(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.time_format(), TimestampFormat::Millis);
        assert_eq!(frame.content_type(), SyncedContentType::Lyrics);
        assert_eq!(frame.desc(), "Description");

        let lyrics = frame.lyrics();

        assert_eq!(lyrics[0].timestamp, Timestamp::Millis(162_000));
        assert_eq!(lyrics[0].text, "You don't remember, you don't remember\n");
        assert_eq!(lyrics[1].timestamp, Timestamp::Millis(166_000));
        assert_eq!(lyrics[1].text, "Why don't you remember my name?\n");
    }

    #[test]
    fn parse_bomless_synced_lyrics() {
        let data = b"\x01\
                     eng\0\
                     \x02\x01\
                     \xFF\xFE\x44\x00\x65\x00\x73\x00\x63\x00\x72\x00\x69\x00\x70\x00\
                     \x74\x00\x69\x00\x6f\x00\x6e\x00\0\0\
                     \x59\x00\x6f\x00\x75\x00\x20\x00\x64\x00\x6f\x00\x6e\x00\
                     \x27\x00\x74\x00\x20\x00\x72\x00\x65\x00\x6d\x00\x65\x00\x6d\x00\
                     \x62\x00\x65\x00\x72\x00\x2c\x00\x20\x00\x79\x00\x6f\x00\x75\x00\
                     \x20\x00\x64\x00\x6f\x00\x6e\x00\x27\x00\x74\x00\x20\x00\x72\x00\
                     \x65\x00\x6d\x00\x65\x00\x6d\x00\x62\x00\x65\x00\x72\x00\x0a\x00\0\0\
                     \x00\x02\x78\xD0\
                     \x57\x00\x68\x00\x79\x00\x20\x00\x64\x00\x6f\x00\x6e\x00\
                     \x27\x00\x74\x00\x20\x00\x79\x00\x6f\x00\x75\x00\x20\x00\x72\x00\
                     \x65\x00\x6d\x00\x65\x00\x6d\x00\x62\x00\x65\x00\x72\x00\x20\x00\
                     \x6d\x00\x79\x00\x20\x00\x6e\x00\x61\x00\x6d\x00\x65\x00\x3f\x00\
                     \x0a\x00\0\0\
                     \x00\x02\x88\x70";

        let mut frame = SyncedLyricsFrame::new();
        frame.parse(&TagHeader::new_test(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.time_format(), TimestampFormat::Millis);
        assert_eq!(frame.content_type(), SyncedContentType::Lyrics);
        assert_eq!(frame.desc(), "Description");

        let lyrics = frame.lyrics();

        assert_eq!(lyrics[0].timestamp, Timestamp::Millis(162_000));
        assert_eq!(lyrics[0].text, "You don't remember, you don't remember\n");
        assert_eq!(lyrics[1].timestamp, Timestamp::Millis(166_000));
        assert_eq!(lyrics[1].text, "Why don't you remember my name?\n");
    }
}
