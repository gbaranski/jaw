pub const SERVER_LISTEN_PORT: u16 = 8080;

use bytes::{Buf, BufMut};
use std::convert::TryFrom;
use std::io::BufRead;

#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("invalid opcode {0}")]
    InvalidOpcode(u8),

    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum Opcode {
    Connect = 0x01,
    Set = 0x02,
}

impl TryFrom<u8> for Opcode {
    type Error = DeserializeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Connect),
            0x02 => Ok(Self::Set),
            value => Err(DeserializeError::InvalidOpcode(value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Frame {
    Connect {},
    Set { bytes: bytes::Bytes },
}

impl Frame {
    pub fn deserialize(mut bytes: impl Buf) -> Result<Self, DeserializeError> {
        let opcode = bytes.get_u8();
        let opcode = Opcode::try_from(opcode)?;
        let frame = match opcode {
            Opcode::Connect => Self::Connect {},
            Opcode::Set => {
                let mut buf = Vec::with_capacity(512);
                bytes.reader().read_until(0x00, &mut buf)?;
                buf.pop();
                Self::Set { bytes: buf.into() }
            }
        };

        Ok(frame)
    }

    pub fn serialize(&self, mut buf: impl BufMut) -> Result<(), SerializeError> {
        buf.put_u8(self.opcode() as u8);
        match self {
            Frame::Connect {} => {}
            Frame::Set { bytes } => {
                buf.put_slice(&bytes);
                buf.put_u8(0x00);
            }
        };
        Ok(())
    }

    pub fn opcode(&self) -> Opcode {
        match self {
            Frame::Connect { .. } => Opcode::Connect,
            Frame::Set { .. } => Opcode::Set,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod connect {
        use super::*;

        #[test]
        fn de() {
            const BYTES: &[u8] = &[0x01];
            assert_eq!(Frame::deserialize(BYTES).unwrap(), Frame::Connect {});
        }

        #[test]
        fn ser() {
            let mut buffer = Vec::new();
            Frame::Connect {}.serialize(&mut buffer).unwrap();
            assert_eq!(buffer, [0x01]);
        }

        #[test]
        fn serde() {
            let mut buffer = Vec::new();
            let frame = Frame::Connect {};
            frame.serialize(&mut buffer).unwrap();
            assert_eq!(Frame::deserialize(buffer.as_slice()).unwrap(), frame);
        }
    }

    mod set {
        use super::*;

        #[test]
        fn de() {
            const BYTES: &[u8] = &[
                0x02, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x00, 0x10, 0x11,
            ];
            assert_eq!(
                Frame::deserialize(BYTES).unwrap(),
                Frame::Set {
                    bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]
                        .as_ref()
                        .into()
                }
            );
        }

        #[test]
        fn ser() {
            let mut buffer = Vec::new();
            Frame::Set {
                bytes: vec![1, 2, 3, 4, 5].into(),
            }
            .serialize(&mut buffer)
            .unwrap();
            assert_eq!(buffer, [0x02, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00]);
        }

        #[test]
        fn serde() {
            let mut buffer = Vec::new();
            let frame = Frame::Set {
                bytes: vec![1, 2, 3, 4, 5].into(),
            };
            frame.serialize(&mut buffer).unwrap();
            assert_eq!(Frame::deserialize(buffer.as_slice()).unwrap(), frame);
        }
    }
}
