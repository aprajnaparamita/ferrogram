// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The OLE compound file container and its streams (ADR 0068).
//!
//! A binary Visio file is an OLE compound file (MS-CFB) with a small
//! number of streams. [`read_streams`] reads the container into memory
//! with [`crate::Limits`] caps. [`decompress`] restores one compressed
//! block of the Visio stream format.
//!
//! Task P8-01 defines the entry points as interface stubs; task P8-02
//! implements them.

use crate::error::Error;
use crate::limits::Limits;

/// The stream that holds the drawing: its header (magic and version)
/// followed by the chunk records.
pub const VISIO_DOCUMENT: &str = "VisioDocument";

/// One stream of the compound file, fully read and, when the format
/// compresses it, decompressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// The stream name, e.g. `"VisioDocument"`.
    pub name: String,
    /// The stream's bytes.
    pub data: Vec<u8>,
}

/// The streams of one compound file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Streams {
    /// The streams in directory order.
    pub streams: Vec<Stream>,
}

impl Streams {
    /// The bytes of the named stream, or `None` when the file has no
    /// such stream.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.streams
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.data.as_slice())
    }

    /// The bytes of the [`VISIO_DOCUMENT`] stream, or `None` when the
    /// file has none.
    pub fn visio_document(&self) -> Option<&[u8]> {
        self.get(VISIO_DOCUMENT)
    }
}

/// Reads every stream of the compound file `bytes`.
///
/// The file must be an OLE compound file (MS-CFB); anything else is
/// [`Error::NotAVisioFile`]. Caps in `limits` bound the stream count
/// and the total bytes read.
///
/// Task P8-01 leaves this an interface stub; task P8-02 implements it.
#[allow(dead_code)] // no caller until P8-07 wires the pipeline
pub fn read_streams(bytes: &[u8], limits: &Limits) -> Result<Streams, Error> {
    let _ = (bytes, limits);
    Err(Error::NotImplemented)
}

/// Decompresses one compressed block of the Visio stream format
/// (ADR 0068): an LZ-style scheme with a small sliding window.
///
/// `limits` caps the output size and the expansion ratio. Uncompressed
/// data is not passed here; the caller checks the format's compression
/// marker first.
///
/// Task P8-01 leaves this an interface stub; task P8-02 implements it.
#[allow(dead_code)] // no caller until P8-07 wires the pipeline
pub fn decompress(data: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let _ = (data, limits);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_get_finds_a_stream_by_name() {
        let streams = Streams {
            streams: vec![
                Stream {
                    name: VISIO_DOCUMENT.to_owned(),
                    data: vec![1, 2, 3],
                },
                Stream {
                    name: "\x05SummaryInformation".to_owned(),
                    data: vec![],
                },
            ],
        };
        assert_eq!(streams.get(VISIO_DOCUMENT), Some([1u8, 2, 3].as_slice()));
        assert_eq!(streams.get("\x05SummaryInformation"), Some([].as_slice()));
        assert_eq!(streams.get("Missing"), None);
    }

    #[test]
    fn visio_document_is_the_named_stream() {
        let empty = Streams::default();
        assert_eq!(empty.visio_document(), None);
        let streams = Streams {
            streams: vec![Stream {
                name: VISIO_DOCUMENT.to_owned(),
                data: vec![9],
            }],
        };
        assert_eq!(streams.visio_document(), Some([9u8].as_slice()));
    }

    #[test]
    fn read_streams_is_a_stub_returning_not_implemented() {
        let err = read_streams(&[], &Limits::DEFAULT).unwrap_err();
        assert_eq!(err, Error::NotImplemented);
    }

    #[test]
    fn decompress_is_a_stub_returning_not_implemented() {
        let err = decompress(&[], &Limits::DEFAULT).unwrap_err();
        assert_eq!(err, Error::NotImplemented);
    }
}
