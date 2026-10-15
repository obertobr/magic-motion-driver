# LG Magic Motion — Receiver Protocol

Reverse-engineered from an **AN-MR200** remote + LG RF USB receiver, validated on
Linux and Windows with real captures (see `tests/fixtures/captured_frames.txt`).

## Transport

| Item | Value |
|---|---|
| USB bridge | Silicon Labs **CP2102** — VID `0x10C4`, PID `0xEA60` |
| Windows | `Silicon Labs CP210x USB to UART Bridge (COMx)` (driver `silabser.sys`) |
| Serial | `115200` baud, `8-N-1`, no flow control |
| Rate | ~111 frames/s, continuous while the remote is awake |

The receiver only streams; the host never needs to send anything.

## Frame (17 bytes, fixed length)

| Byte | Field | Notes |
|---:|---|---|
| 0 | `STX` | always `0x02` |
| 1 | `aux` | slowly drifting analog reading (`0x36`–`0x39` observed). **Not** a button/event flag. Probably battery or temperature. |
| 2–3 | gyro X (BE `u16`) | yaw angular rate, rest ≈ 537 |
| 4–5 | gyro Y (BE `u16`) | pitch angular rate, rest ≈ 537 |
| 6 | **button code** | `0x00` when nothing is pressed (see below) |
| 7–8 | accel X (BE `u16`) | |
| 9–10 | accel Y (BE `u16`) | |
| 11–12 | accel Z (BE `u16`) | |
| 13 | link | `0x01` while paired and transmitting |
| 14 | sequence | `+1` per frame, wraps at `0xFF` (occasional gaps = RF loss) |
| 15 | checksum | `!(sum(bytes[1..=14])) & 0xFF` |
| 16 | `ETX` | always `0x03` |

## Button codes (byte 6)

The code is held for as long as the button is reported as pressed and returns
to `0x00` on release.

**Navigation codes** — exclusive values with the high nibble `0xF`:

| Code | Button |
|---|---|
| `0xF1` | Up |
| `0xF2` | Down |
| `0xF3` | Left |
| `0xF4` | Right |
| `0xF5` | Home |

**Flag bits** — any other value is a bitmask (the high nibble can never be
`0xF` because bit 7 is never used):

| Bit | Button |
|---|---|
| `0x01` | Mute |
| `0x02` | OK |
| `0x04` | Volume + |
| `0x08` | Volume − |
| `0x10` | Channel + |
| `0x40` | Channel − |

`0x20` and `0x80` were never observed.

**Power** is not reported over RF: pressing it puts the remote to sleep and
the stream stops, so it cannot be remapped.

The AN-MR200 has **no scroll wheel**.
