use std::io;

use bytes::BytesMut;
use proto::{Message, format, parse};
use tokio_util::codec::{Decoder, Encoder};

pub type ParseResult<T = Message, E = parse::Error> = std::result::Result<T, E>;

pub struct Codec;

/// Maximum bytes buffered for a single IRC line before it is rejected. Generous headroom over the
/// IRCv3 message-tags limit (8191) plus the 512-byte message.
const MAX_LINE_LENGTH: usize = 16 * 1024;

impl Decoder for Codec {
    type Item = ParseResult;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let Some(pos) = src.windows(2).position(|b| b == [b'\r', b'\n']) else {
            // Guard against a peer that never sends CRLF: without a cap the framed stream would
            // buffer the "line" without bound, exhausting memory from a single connection.
            if src.len() > MAX_LINE_LENGTH {
                return Err(Error::LineTooLong);
            }
            return Ok(None);
        };

        Ok(Some(parse::message_bytes(&src.split_to(pos + 2))))
    }
}

impl Encoder<Message> for Codec {
    type Error = Error;

    fn encode(
        &mut self,
        message: Message,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let encoded = format::message(message);

        dst.extend(encoded.into_bytes());

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("IRC line exceeded the maximum buffered length")]
    LineTooLong,
}
