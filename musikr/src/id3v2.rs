//! ID3v2 tag manipulation.
//!
//! ID3v2 is the most common tag format, being the primary tag format in MP3 files and
//! having a presence in other formats as well. However, its also the most complex
//! tag format, making this module one of the more complicated APIs to use in musikr.
//!
//! The ID3v2 module assumes that you have working knowledge of the ID3v2 tag format, so
//! it's recommended to read the [ID3v2.3](https://id3.org/id3v2.3.0) and
//! [ID3v2.4](https://id3.org/id3v2.4.0-structure) documents to get a better idea of the
//! tag structure.
//!
//! # Usage

pub mod collections;
mod compat;
#[macro_use]
mod macros;
pub mod frames;
mod syncdata;
pub mod tag;

use crate::core::io::{write_replaced, BufStream};
use collections::{FrameMap, UnknownFrames};
use frames::FrameResult;
use tag::{ExtendedHeader, SaveVersion, TagHeader, Version};

use log::{error, info, warn};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// TODO: The current roadmap:
// - Try to complete most if not all of the frame specs
// - Add further documentation
// - Improve testing
// - Make deep and shallow find methods that can search anything that implements Read.
// The former will search for all tags and concat them, while the latter will be like
// Tag::open. There might be a deep_clean method as well that is like deep_find, but
// also removes the nested tags. This would likely be for a file.

/// An ID3v2 tag.
///
/// A tag can be created programmatically, or it can be opened from a file.
#[derive(Debug, Clone)]
pub struct Tag {
    header: TagHeader,
    /// The tag's extended header. This is optional.
    pub extended_header: Option<ExtendedHeader>,
    /// A collection of known frames.
    pub frames: FrameMap,
    /// A collection of unknown frames encountered during parsing.
    pub unknown_frames: UnknownFrames,
}

impl Tag {
    /// Creates an empty tag.
    /// 
    /// The version of the new tag will always be ID3v2.4.If another version is 
    /// desired, [`with_version`](Tag::with_version) can be used instead.
    pub fn new() -> Self {
        Self::with_version(SaveVersion::V24)
    }

    /// Creates an empty tag with the specified `version`.
    pub fn with_version(version: SaveVersion) -> Self {
        Tag {
            header: TagHeader::with_version(Version::from(version)),
            extended_header: None,
            frames: FrameMap::new(),
            unknown_frames: UnknownFrames::new(Version::from(version), Vec::new()),
        }
    }

    /// Attempts to open and parse a tag in `path`.
    ///
    /// All ID3v2.2 tags will be upgraded to ID3v2.3 if they are read. If the file cannot 
    /// be opened, does not contain a tag, or if the tag is malformed, an error will be 
    /// returned with a general reason for why. Specific information about parsing errors 
    /// will be logged.
    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let mut file = File::open(path)?;

        // Read and parse the possible ID3v2 header
        let mut header_raw = [0; 10];
        file.read_exact(&mut header_raw)?;

        let mut header = TagHeader::parse(header_raw).map_err(|err| match err {
            ParseError::MalformedData => ParseError::NotFound,
            err => err,
        })?;

        // Then get the full tag data. If the size is invalid, then we will just truncate it.
        let mut tag_data = vec![0; header.size() as usize];
        let read = file.read(&mut tag_data)?;
        tag_data.truncate(read);

        let mut stream = BufStream::new(&tag_data);

        // ID3v2.3 tag-specific synchronization, decode the stream here.
        if header.version() < Version::V24 && header.flags().unsync {
            tag_data = syncdata::decode(&mut stream);
            stream = BufStream::new(&tag_data);
        }

        let mut extended_header = None;

        if header.flags().extended {
            // Certain taggers will flip the extended header flag without writing one,
            // so if parsing fails then we correct the flag.
            match ExtendedHeader::parse(&mut stream, header.version()) {
                Ok(header) => extended_header = Some(header),
                Err(_) => {
                    info!("resetting incorrectly-set extended header flag");
                    header.flags_mut().extended = false
                }
            }
        }

        // Now try parsing our frames.
        let mut frames = FrameMap::new();
        let mut unknowns = Vec::new();

        while let Ok(result) = frames::parse(&header, &mut stream) {
            match result {
                FrameResult::Frame(frame) => frames.add_boxed(frame),
                FrameResult::Unknown(unknown) => {
                    info!("found unknown frame {}", unknown.id_str());
                    unknowns.push(unknown)
                }
                FrameResult::Dropped => {
                    // Dropped frames have already moved the stream to the next
                    // frame, so we can skip them.
                }
            }
        }

        // Unknown frames are kept in a separate collection for two reasons:
        // 1. To make sure downcasting behavior is consistent
        // 2. To make sure tags of one version don't end up polluted with frames of another
        // version.
        let unknown_frames = UnknownFrames::new(header.version(), unknowns);

        Ok(Self {
            header,
            extended_header,
            frames,
            unknown_frames,
        })
    }

    /// Returns the version of this tag.
    ///
    /// While ID3v2.2 tags are converted to ID3v2.3, the version will still be
    /// [`Version::V22`](crate::id3v2::tag::Version::V22) until the tag is saved 
    /// or upgraded.
    pub fn version(&self) -> Version {
        self.header.version()
    }

    /// Returns the total size of this tag, in bytes.
    ///
    /// The size includes the extended header, the tag body [e.g all frames], and
    /// the footer. This value is only updated when the tag is read or saved, so it
    /// may not be accurate to the current contents of a tag. In a freshly created
    /// tag, this value will be `0`.
    pub fn size(&self) -> u32 {
        self.header.size()
    }

    /// Update the tag to the specified version.
    ///
    /// **Update operations are inherently destructive.** Frames will be renamed, merged,
    /// parsed, or removed depending on the target version. The versions that tags are
    /// restricted to are limited to those declared by [`SaveVersion`](crate::id3v2::tag::SaveVersion)
    ///
    /// # ID3v2.3 Conversions
    ///
    /// ```text
    /// EQU2 -> Dropped (no sane conversion)
    /// RVA2 -> Dropped (no sane conversion)
    /// ASPI -> Dropped (no analogue)
    /// SEEK -> Dropped (no analogue)
    /// SIGN -> Dropped (no analogue)
    /// TDEN -> Dropped (no analogue)
    /// TDRL -> Dropped (no analogue)
    /// TDTG -> Dropped (no analogue)
    /// TMOO -> Dropped (no analogue)
    /// TPRO -> Dropped (no analogue)
    /// TSST -> Dropped (no analogue)
    ///
    /// Note: iTunes writes these frames to ID3v2.3 tags, but musikr will still drop these.
    /// TSOA -> Dropped (no analogue)
    /// TSOP -> Dropped (no analogue)
    /// TSOT -> Dropped (no analogue)
    ///
    /// TDOR -> TORY
    /// TIPL -> IPLS
    /// TMCL -> IPLS
    /// TRDC -> (yyyy)(-MM-dd)(THH:mm):ss
    ///          TYER   TDAT    TIME
    /// ```
    ///
    /// # ID3v2.4 Conversions
    ///
    /// ```
    /// EQUA -> Dropped (no sane conversion)
    /// RVAD -> Dropped (no sane conversion)
    /// TRDA -> Dropped (no sane conversion)
    /// TSIZ -> Dropped (no analogue)
    /// IPLS -> TIPL
    /// TYER -> TRDC: (yyyy)- MM-dd  THH:mm :ss
    /// TDAT -> TDRC:  yyyy -(MM-dd) THH:mm :ss
    /// TIME -> TDRC:  yyyy - MM-dd (THH:mm):ss
    /// TORY -> TDOR: (yyyy)- MM-dd  THH:mm :ss
    /// ```
    pub fn update(&mut self, to: SaveVersion) {
        match to {
            SaveVersion::V23 => compat::to_v3(&mut self.frames),
            SaveVersion::V24 => compat::to_v4(&mut self.frames),
        }

        if let Some(ext) = &mut self.extended_header {
            ext.update(to)
        }

        *self.header.version_mut() = Version::from(to);
    }

    /// Clears the tag.
    ///
    /// This will remove all known and unknown frames, alongside the extended header.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.unknown_frames = UnknownFrames::new(self.version(), Vec::new());
        self.extended_header = None;
    }

    /// Saves the tag to `path`.
    ///
    /// [`Tag::update`](Tag::update) will be called with either the tag's current version in 
    /// the case of ID3v2.3/ID3v2.4, or to ID3v2.3 in the case of ID3v2.2.
    /// 
    /// All known frames will be written, while unknown frames will be written only if [`Tag::version`](Tag::version)
    /// is equal to [`UnknownFrames::version`](crate::id3v2::collections::UnknownFrames::version).
    /// No unsynchronization, compression, or similar manipulation is done on the tag body.
    ///
    /// The tag will be written to the file regardless of if a previous tag is present. If the 
    /// is written to a file that may not support ID3v2, this may render the file inoperable.
    /// If the written tag is smaller than a pre-existing tag, at most 1% of the file size will be
    /// used for padding. If the tag is larger, then 1 KiB of padding will be applied. 
    ///
    /// If the tag creation or writing process fails, then an error with a general reason will
    /// be returned.  Specific information about saving errors will be logged.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> SaveResult<()> {
        // Before saving, ensure that our tag has been fully upgraded. ID3v2.2 tags always
        // become ID3v2.3 tags, as it has been obsoleted.
        match self.header.version() {
            Version::V22 | Version::V23 => self.update(SaveVersion::V23),
            Version::V24 => self.update(SaveVersion::V24),
        };

        // Reset all the flags that we don't really have a way to expose or support.
        let flags = self.header.flags_mut();
        flags.unsync = false; // Modern software is aware of ID3v2, making this obsolete
        flags.extended = self.extended_header.is_some(); // Supported
        flags.experimental = false; // This has no use defined by the spec
        flags.footer = false; // May be exposed in the future

        // Render the extended header first, if it's present.
        let mut tag_data = match &self.extended_header {
            Some(ext) => ext.render(self.header.version()),
            None => Vec::new(),
        };

        // Keep track of the body length here so we can tell if we actually wrote frames.
        let start_len = tag_data.len();

        tag_data.extend(self.frames.render(&self.header));
        
        // While we could theoretically upgrade unknown frames, its better that we don't
        // since they could be metaframes and since the flags would also have to be changed.
        if self.unknown_frames.version() == self.version() {
            for frame in self.unknown_frames.frames() {
                tag_data.extend(frames::render_unknown(&self.header, frame))
            }
        } else {
            warn!("dropping {} unknown frames", self.unknown_frames.version())
        }

        // Check if theres an existing header in this file or not.
        // If there is, keep track of its size so that we can replace it with this tag.
        let mut len = 0;
        let mut old_size = 0;

        if let Ok(mut file) = File::open(&path) {
            len = file.metadata()?.len();

            let mut header_raw = [0; 10];

            if file.read(&mut header_raw).is_ok() {
                if let Ok(header) = TagHeader::parse(header_raw) {
                    info!("found previously written tag, will be overwritten");

                    old_size = header.size() as u64
                }
            };
        }

        // Make sure our tag isn't empty. If it is, then we will just delete the tag.
        if tag_data.len() > start_len {
            // Find a sensible padding length. We make all tag sizes here u64 so that we don't accidentally
            // overflow while doing this.
            let tag_size = tag_data.len() as u64;

            let padding_size = match u64::checked_sub(old_size, tag_size) {
                Some(delta) => u64::min(delta, len / 100), // Tag is smaller, use the remaining space or 1% of the file size
                None => 1024,                              // Tag is larger, use 1KiB.
            };

            let tag_size = tag_size + padding_size;

            // Tag sizes are syncsafe, so tags can never be more than 256mb. This also ensures that we won't overflow the
            // u32 when we cast it.
            if tag_size > 256_000_000 {
                error!("tag was larger than 256mb");
                return Err(SaveError::TooLarge);
            }

            *self.header.size_mut() = tag_size as u32;

            // Finalize our tag, adding the padding and prepending the header.
            tag_data.resize(tag_size as usize, 0);
            tag_data.splice(0..0, self.header.render());

            write_replaced(path, &tag_data, old_size + 10)?;
        } else {
            info!("tag is empty, deleting tag instead");

            *self.header.size_mut() = 0;

            write_replaced(&path, &[], old_size + 10)?;
        }

        Ok(())
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new()
    }
}

/// The result given after a parsing operation.
pub type ParseResult<T> = Result<T, ParseError>;

/// The error type returned when parsing ID3v2 tags.
#[derive(Debug)]
pub enum ParseError {
    /// Generic IO errors. This either means that a problem occurred while opening the file
    /// for a tag, or an unexpected EOF was encountered while parsing.
    IoError(io::Error),
    /// A part of the tag was not valid.
    MalformedData,
    /// The tag or a element of the tag is unsupported.
    Unsupported,
    /// The tag was not found in the given file.
    NotFound,
}

impl From<io::Error> for ParseError {
    fn from(other: io::Error) -> Self {
        ParseError::IoError(other)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => err.fmt(f),
            Self::MalformedData => write![f, "malformed data"],
            Self::Unsupported => write![f, "unsupported"],
            Self::NotFound => write![f, "not found"],
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        let error = match self {
            Self::IoError(err) => err,
            _ => return None,
        };

        Some(error)
    }
}

/// The result given after a save operation.
pub type SaveResult<T> = Result<T, SaveError>;

/// The error type returned when saving ID3v2 tags.
#[derive(Debug)]
pub enum SaveError {
    /// Generic IO errors. This means that a problem occurred while writing the tag to a file.
    IoError(io::Error),
    /// The tag [or an element in the tag] was too large to be written.
    TooLarge,
}

impl From<io::Error> for SaveError {
    fn from(other: io::Error) -> Self {
        SaveError::IoError(other)
    }
}

impl Display for SaveError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => err.fmt(f),
            Self::TooLarge => write![f, "tag is too large to be saved"],
        }
    }
}

impl error::Error for SaveError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        let error = match self {
            Self::IoError(err) => err,
            _ => return None,
        };

        Some(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3v2::frames::CommentsFrame;
    use crate::core::string::Encoding;
    use std::env;

    #[test]
    fn read_id3v22() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/v22.mp3";
        let tag = Tag::open(&path).unwrap();
        id3v22_ensure(&tag, Version::V22);
    }

    #[test]
    fn write_id3v22() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/v22.mp3";
        let mut tag = Tag::open(&path).unwrap();

        let out = env::temp_dir().join("musikr_test.mp3");
        tag.save(&out).unwrap();

        let tag = Tag::open(out).unwrap();
        id3v22_ensure(&tag, Version::V23);
    }

    fn id3v22_ensure(tag: &Tag, version: Version) {
        assert_eq!(tag.version(), version);
        assert_eq!(tag.frames["TIT2"].to_string(), "cosmic american");
        assert_eq!(tag.frames["TPE1"].to_string(), "Anais Mitchell");
        assert_eq!(tag.frames["TALB"].to_string(), "Hymns for the Exiled");
        assert_eq!(tag.frames["TRCK"].to_string(), "3/11");
        assert_eq!(tag.frames["TYER"].to_string(), "2004");
        assert_eq!(tag.frames["TENC"].to_string(), "iTunes v4.6");

        let comm = tag.frames["COMM::eng"].downcast::<CommentsFrame>().unwrap();
        assert_eq!(comm.encoding, Encoding::Latin1);
        assert_eq!(comm.text, "Waterbug Records, www.anaismitchell.com");

        let norm = tag.frames["COMM:iTunNORM:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(norm.encoding, Encoding::Latin1);
        assert_eq!(norm.text, " 0000044E 00000061 00009B67 000044C3 00022478 00022182 00007FCC 00007E5C 0002245E 0002214E");

        let cddb = tag.frames["COMM:iTunes_CDDB_1:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(cddb.encoding, Encoding::Latin1);
        assert_eq!(cddb.text, "9D09130B+174405+11+150+14097+27391+43983+65786+84877+99399+113226+132452+146426+163829");

        let dbtk = tag.frames["COMM:iTunes_CDDB_TrackNumber:eng"]
            .downcast::<CommentsFrame>()
            .unwrap();
        assert_eq!(dbtk.encoding, Encoding::Latin1);
        assert_eq!(dbtk.text, "3");
    }
}
