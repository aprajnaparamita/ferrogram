// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Chunk records and pointers of the `VisioDocument` stream (ADR 0069).
//!
//! The stream starts with a fixed header: the Visio magic
//! `"Visio (TM) Drawing\r\n\0"` and, at offset `0x1A`, the one-byte file
//! version. The rest of the stream is a sequence of chunk records; each
//! record header names its record kind, its own id, the list it indexes
//! into, its payload length and its nesting level. Some records are
//! pointers: they redirect to another offset of the stream, and list
//! records group the items they index. The exact header layout differs
//! between the version 6 and the version 11 format.
//!
//! [`read_chunks`] flattens the stream into [`Chunk`] values in stream
//! order with their nesting level preserved; the domain modules
//! ([`crate::shape`], [`crate::text`], [`crate::style`],
//! [`crate::pages`]) interpret the payloads.
//!
//! Task P8-01 defines the entry points as interface stubs; task P8-03
//! implements them.

use crate::error::Error;
use crate::limits::Limits;

/// The byte offset of the version inside the `VisioDocument` stream,
/// after the Visio magic.
pub const VERSION_OFFSET: usize = 0x1A;

/// The file format version (ADR 0069).
///
/// The alpha reads the two chunk-format versions. Versions 1 to 5 use
/// an older record layout and are out of scope;
/// [`FormatVersion::from_byte`] rejects them with
/// [`Error::UnsupportedVersion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatVersion {
    /// File version 6 (Visio 2000 and 2002).
    V6,
    /// File version 11 (Visio 2003); every later Visio writes this
    /// version when it saves `.vsd`, `.vss` or `.vst`.
    V11,
}

impl FormatVersion {
    /// The format named by the version byte of the stream header.
    ///
    /// Versions 7 to 10 do not exist in real files; they are rejected
    /// the same way as versions 1 to 5.
    pub fn from_byte(byte: u8) -> Result<FormatVersion, Error> {
        match byte {
            6 => Ok(FormatVersion::V6),
            11 => Ok(FormatVersion::V11),
            other => Err(Error::UnsupportedVersion(other)),
        }
    }
}

/// The kind of one chunk record.
///
/// The value is the record-type id of the binary format. Each domain
/// module compares against the ids it consumes; unknown ids pass
/// through unchanged and are skipped with a warning. The id table is
/// derived from the libvisio reference (AGENTS.md section 5) and lives
/// in this module with its reference note (ADR 0069).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Record(pub u16);

/// One chunk record of the `VisioDocument` stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    /// The record kind.
    pub record: Record,
    /// The nesting level of the record.
    pub level: u16,
    /// The id of the list this record indexes into, or `0`.
    pub list: u32,
    /// The record's own id within its list.
    pub id: u32,
    /// The record payload, after the record header. Byte order and
    /// field widths follow the file version (ADR 0069).
    pub data: &'a [u8],
}

/// The format version of a `VisioDocument` stream.
///
/// The stream must carry at least [`VERSION_OFFSET`] + 1 bytes; a
/// shorter stream is [`Error::Invalid`].
///
/// Task P8-01 leaves this as the stub contract's one real helper;
/// task P8-03 keeps it and adds the magic check.
#[allow(dead_code)] // no caller until P8-07 wires the pipeline
pub fn version(visio_document: &[u8]) -> Result<FormatVersion, Error> {
    let byte = visio_document.get(VERSION_OFFSET).copied().ok_or_else(|| {
        Error::Invalid("VisioDocument stream is shorter than its header".to_owned())
    })?;
    FormatVersion::from_byte(byte)
}

/// Reads every chunk record of the `VisioDocument` stream.
///
/// `stream` is the whole stream, header included; `file_version` is the
/// version [`version`] read from it. Pointers and lists are resolved to
/// the stream offsets they name, each offset at most once, and the
/// result is the records in stream order with their nesting level.
/// `limits` caps the record count and the nesting level.
///
/// Task P8-01 leaves this an interface stub; task P8-03 implements it.
#[allow(dead_code)] // no caller until P8-07 wires the pipeline
pub fn read_chunks<'a>(
    stream: &'a [u8],
    file_version: FormatVersion,
    limits: &Limits,
) -> Result<Vec<Chunk<'a>>, Error> {
    let _ = (stream, file_version, limits);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_byte_accepts_the_two_chunk_versions() {
        assert_eq!(FormatVersion::from_byte(6), Ok(FormatVersion::V6));
        assert_eq!(FormatVersion::from_byte(11), Ok(FormatVersion::V11));
    }

    #[test]
    fn from_byte_rejects_versions_out_of_scope() {
        for byte in [0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 12, 255] {
            assert_eq!(
                FormatVersion::from_byte(byte),
                Err(Error::UnsupportedVersion(byte)),
                "version {byte}"
            );
        }
    }

    #[test]
    fn version_reads_the_byte_after_the_magic() {
        let mut stream = vec![0u8; VERSION_OFFSET + 1];
        stream[VERSION_OFFSET] = 11;
        assert_eq!(version(&stream), Ok(FormatVersion::V11));
        stream[VERSION_OFFSET] = 3;
        assert_eq!(version(&stream), Err(Error::UnsupportedVersion(3)));
    }

    #[test]
    fn version_rejects_a_short_stream() {
        let short = vec![0u8; VERSION_OFFSET];
        assert_eq!(
            version(&short),
            Err(Error::Invalid(
                "VisioDocument stream is shorter than its header".to_owned()
            ))
        );
    }

    #[test]
    fn a_chunk_keeps_its_fields() {
        let chunk = Chunk {
            record: Record(0x8a),
            level: 2,
            list: 7,
            id: 1,
            data: &[1, 2],
        };
        assert_eq!(chunk.record, Record(0x8a));
        assert_eq!(chunk.level, 2);
        assert_eq!(chunk.list, 7);
        assert_eq!(chunk.id, 1);
        assert_eq!(chunk.data, &[1, 2]);
    }

    #[test]
    fn read_chunks_is_a_stub_returning_not_implemented() {
        let err = read_chunks(&[], FormatVersion::V11, &Limits::DEFAULT).unwrap_err();
        assert_eq!(err, Error::NotImplemented);
    }
}
