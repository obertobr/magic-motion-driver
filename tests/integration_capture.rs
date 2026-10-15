//! End-to-end integration test validating decoding against real frames
//! captured from the physical LG Magic Motion remote.

use magic_motion::domain::button::{decode_button_code, Button, ButtonSet};
use magic_motion::domain::motion::{MotionProcessor, MotionSettings};
use magic_motion::domain::protocol::{Frame, FrameDecoder, FRAME_LEN};

#[test]
fn test_real_capture_fixture_decodes_cleanly() {
    let fixture_str = include_str!("fixtures/captured_frames.txt");
    let mut current_section = "";
    let mut decoder = FrameDecoder::new();
    let mut proc = MotionProcessor::new(MotionSettings::default());

    let mut total_frames = 0;
    let mut section_press_detected = false;

    for line in fixture_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = &line[1..line.len() - 1];
            decoder.reset();
            section_press_detected = false;
            continue;
        }

        let bytes: Vec<u8> = line
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).expect("valid hex byte"))
            .collect();
        assert_eq!(bytes.len(), FRAME_LEN);

        decoder.push(&bytes);
        let frame = decoder.next_frame().expect("valid frame from real capture");
        total_frames += 1;

        // Verify checksum with direct parse
        let frame_direct =
            Frame::parse((&bytes[..]).try_into().unwrap()).expect("direct parse succeeds");
        assert_eq!(frame, frame_direct);

        // Motion sanity check: gyro readings should stay near 537 when resting
        if current_section == "idle" {
            let (dx, dy) = proc.process(frame.gyro_x, frame.gyro_y);
            assert_eq!((dx, dy), (0, 0), "resting remote should not move mouse");
        }

        // Button decoding validation
        let buttons = decode_button_code(frame.button_code);
        if frame.button_code != 0 {
            section_press_detected = true;
            match current_section {
                "home" => assert!(buttons.contains(Button::Home)),
                "up" => assert!(buttons.contains(Button::Up)),
                "down" => assert!(buttons.contains(Button::Down)),
                "left" => assert!(buttons.contains(Button::Left)),
                "right" => assert!(buttons.contains(Button::Right)),
                "ok" => assert!(buttons.contains(Button::Ok)),
                "vol_up" => assert!(buttons.contains(Button::VolumeUp)),
                "vol_down" => assert!(buttons.contains(Button::VolumeDown)),
                "ch_up" => assert!(buttons.contains(Button::ChannelUp)),
                "ch_down" => assert!(buttons.contains(Button::ChannelDown)),
                "mute" => assert!(buttons.contains(Button::Mute)),
                _ => {}
            }
        } else if current_section == "idle" {
            assert_eq!(buttons, ButtonSet::EMPTY);
        }
    }

    assert!(total_frames > 200, "expected at least 200 fixture frames, got {total_frames}");
    assert!(section_press_detected, "expected button presses in fixture");
}
