//! Wire protocol of the LG Magic Motion RF receiver. See `docs/PROTOCOL.md`.

use std::fmt;

/// Serial speed used by the receiver's CP2102 bridge.
pub const BAUD_RATE: u32 = 115_200;
/// USB vendor id of the Silicon Labs CP2102 bridge inside the receiver.
pub const RECEIVER_USB_VID: u16 = 0x10C4;
/// USB product id of the Silicon Labs CP2102 bridge inside the receiver.
pub const RECEIVER_USB_PID: u16 = 0xEA60;
/// Every frame has exactly this many bytes.
pub const FRAME_LEN: usize = 17;

const STX: u8 = 0x02;
const ETX: u8 = 0x03;
const CHECKSUM_INDEX: usize = 15;
const PAYLOAD: std::ops::Range<usize> = 1..CHECKSUM_INDEX;
/// Upper bound of buffered bytes, so garbage input can never grow memory.
const MAX_BUFFERED: usize = FRAME_LEN * 8;

/// One decoded sensor/button report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Slowly drifting analog reading of unknown meaning (byte 1).
    pub aux: u8,
    pub gyro_x: u16,
    pub gyro_y: u16,
    /// Raw button code (byte 6); decode with [`crate::domain::button::decode_button_code`].
    pub button_code: u8,
    pub accel: [u16; 3],
    pub link: u8,
    pub sequence: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    BadDelimiters,
    BadChecksum { expected: u8, actual: u8 },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadDelimiters => write!(f, "frame is not delimited by STX/ETX"),
            Self::BadChecksum { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected:#04x}, got {actual:#04x}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// One's complement of the byte sum.
pub fn checksum(payload: &[u8]) -> u8 {
    !payload.iter().fold(0u8, |acc, b| acc.wrapping_add(*b))
}

fn be_u16(bytes: &[u8; FRAME_LEN], index: usize) -> u16 {
    u16::from_be_bytes([bytes[index], bytes[index + 1]])
}

impl Frame {
    pub fn parse(bytes: &[u8; FRAME_LEN]) -> Result<Self, FrameError> {
        if bytes[0] != STX || bytes[FRAME_LEN - 1] != ETX {
            return Err(FrameError::BadDelimiters);
        }
        let expected = checksum(&bytes[PAYLOAD]);
        let actual = bytes[CHECKSUM_INDEX];
        if expected != actual {
            return Err(FrameError::BadChecksum { expected, actual });
        }
        Ok(Self {
            aux: bytes[1],
            gyro_x: be_u16(bytes, 2),
            gyro_y: be_u16(bytes, 4),
            button_code: bytes[6],
            accel: [be_u16(bytes, 7), be_u16(bytes, 9), be_u16(bytes, 11)],
            link: bytes[13],
            sequence: bytes[14],
        })
    }
}

/// Counters useful for diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecoderStats {
    pub frames: u64,
    pub discarded_bytes: u64,
}

/// Incremental, allocation-free (after construction) stream decoder that
/// resynchronises on corrupted or partial data.
#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    stats: DecoderStats,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(MAX_BUFFERED + FRAME_LEN),
            stats: DecoderStats::default(),
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_BUFFERED {
            let excess = self.buffer.len() - MAX_BUFFERED;
            self.discard(excess);
        }
    }

    /// Returns the next valid frame, or `None` when more bytes are needed.
    pub fn next_frame(&mut self) -> Option<Frame> {
        loop {
            match self.buffer.iter().position(|&b| b == STX) {
                Some(0) => {}
                Some(offset) => self.discard(offset),
                None => {
                    self.discard(self.buffer.len());
                    return None;
                }
            }
            let candidate: &[u8; FRAME_LEN] = self.buffer.get(..FRAME_LEN)?.try_into().ok()?;
            match Frame::parse(candidate) {
                Ok(frame) => {
                    self.buffer.drain(..FRAME_LEN);
                    self.stats.frames += 1;
                    return Some(frame);
                }
                // The STX was a payload byte: skip it and look for the next one.
                Err(_) => self.discard(1),
            }
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    pub fn stats(&self) -> DecoderStats {
        self.stats
    }

    fn discard(&mut self, count: usize) {
        self.buffer.drain(..count);
        self.stats.discarded_bytes += count as u64;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a valid frame with the given gyro/button values.
    pub(crate) fn make_frame(gyro_x: u16, gyro_y: u16, button_code: u8, seq: u8) -> [u8; FRAME_LEN] {
        let mut f = [0u8; FRAME_LEN];
        f[0] = STX;
        f[1] = 0x37;
        f[2..4].copy_from_slice(&gyro_x.to_be_bytes());
        f[4..6].copy_from_slice(&gyro_y.to_be_bytes());
        f[6] = button_code;
        f[7..9].copy_from_slice(&1043u16.to_be_bytes());
        f[9..11].copy_from_slice(&2055u16.to_be_bytes());
        f[11..13].copy_from_slice(&2091u16.to_be_bytes());
        f[13] = 0x01;
        f[14] = seq;
        f[CHECKSUM_INDEX] = checksum(&f[PAYLOAD]);
        f[16] = ETX;
        f
    }

    fn hex(s: &str) -> [u8; FRAME_LEN] {
        let bytes: Vec<u8> = s
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();
        bytes.try_into().unwrap()
    }

    #[test]
    fn parses_real_frame() {
        let frame = Frame::parse(&hex("02 37 02 19 02 1a 00 04 12 08 02 08 27 01 d2 6f 03")).unwrap();
        assert_eq!(frame.aux, 0x37);
        assert_eq!(frame.gyro_x, 537);
        assert_eq!(frame.gyro_y, 538);
        assert_eq!(frame.button_code, 0);
        assert_eq!(frame.accel, [0x0412, 0x0802, 0x0827]);
        assert_eq!(frame.link, 1);
        assert_eq!(frame.sequence, 0xD2);
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut raw = make_frame(537, 537, 0, 1);
        raw[3] ^= 0x01;
        assert!(matches!(Frame::parse(&raw), Err(FrameError::BadChecksum { .. })));
    }

    #[test]
    fn rejects_bad_delimiters() {
        let mut raw = make_frame(537, 537, 0, 1);
        raw[16] = 0x00;
        assert_eq!(Frame::parse(&raw), Err(FrameError::BadDelimiters));
    }

    #[test]
    fn decodes_byte_by_byte() {
        let mut decoder = FrameDecoder::new();
        let raw = make_frame(600, 500, 0x02, 7);
        let mut frames = Vec::new();
        for b in raw.iter().chain(raw.iter()) {
            decoder.push(std::slice::from_ref(b));
            frames.extend(decoder.next_frame());
        }
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].gyro_x, 600);
        assert_eq!(frames[0].button_code, 0x02);
    }

    #[test]
    fn resyncs_after_garbage_and_corruption() {
        let mut decoder = FrameDecoder::new();
        let mut corrupted = make_frame(537, 537, 0, 1);
        corrupted[5] ^= 0xFF;
        decoder.push(&[0xAA, 0x02, 0x55]);
        decoder.push(&corrupted);
        decoder.push(&make_frame(540, 530, 0, 2));

        let frame = decoder.next_frame().expect("valid frame after garbage");
        assert_eq!(frame.sequence, 2);
        assert!(decoder.next_frame().is_none());
        assert_eq!(decoder.stats().frames, 1);
        assert!(decoder.stats().discarded_bytes >= 3 + FRAME_LEN as u64);
    }

    #[test]
    fn stx_inside_payload_does_not_break_sync() {
        // gyro_x = 0x0202 puts STX bytes inside the payload.
        let mut decoder = FrameDecoder::new();
        decoder.push(&make_frame(0x0202, 0x0203, 0, 1));
        decoder.push(&make_frame(0x0202, 0x0203, 0, 2));
        assert_eq!(decoder.next_frame().unwrap().sequence, 1);
        assert_eq!(decoder.next_frame().unwrap().sequence, 2);
    }

    #[test]
    fn buffer_is_bounded() {
        let mut decoder = FrameDecoder::new();
        for _ in 0..1000 {
            decoder.push(&[0x02; 64]);
            let _ = decoder.next_frame();
        }
        assert!(decoder.buffer.len() <= MAX_BUFFERED);
    }
}
