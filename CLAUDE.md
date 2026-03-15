# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust library for parsing and encoding APRS (Automatic Packet Reporting System) packets. Supports both textual (APRS-IS) and binary (AX.25/KISS) formats. Minimum supported Rust version: 1.60.0.

## Build Commands

```bash
cargo build                                                    # Build
cargo test                                                     # Run all tests
cargo test <test_name>                                         # Run a single test
cargo fmt                                                      # Format code
cargo fmt -- --check                                           # Check formatting
cargo clippy --all-targets --all-features -- -D warnings       # Lint (warnings are errors)
```

## Architecture

The crate centers on `AprsPacket` (defined in `packet.rs`), which contains a source `Callsign`, a `Vec<Via>` routing path, and an `AprsData` enum dispatching to packet types:

- **Position** (`position.rs`) — uncompressed and compressed position reports with optional timestamp, extensions (course/speed, PHG, DFS), and altitude
- **MicE** (`mic_e.rs`) — Mic-E encoded position/status, destination-encoded latitude
- **Message** (`message.rs`) — point-to-point messages with addressee and optional message ID
- **Object** (`object.rs`) / **Item** (`item.rs`) — named map objects and items
- **Status** (`status.rs`) — free-text status reports

Key supporting modules:
- `components/lonlat.rs` — `Latitude`, `Longitude`, `Precision` types with encode/decode for both compressed and uncompressed formats
- `components/extensions.rs` — `Extension` enum (DirectionSpeed, PHG, DFS, RadioRange, AreaDescription)
- `base91.rs` — Base-91 encoding used by compressed positions
- `compressed_cs.rs` — compressed course/speed/altitude handling
- `callsign.rs` — `Callsign` with SSID support, AX.25 byte-level encoding
- `error.rs` — `DecodeError` / `EncodeError` via `thiserror`

Entry points: `AprsPacket::decode_textual()` / `encode_textual()` for APRS-IS format, `AprsPacket::decode_ax25()` / `encode_ax25()` for binary AX.25.

## Dependencies

- `thiserror` — error type derives (only production dependency)
- `approx` — floating-point comparison in tests (dev only)
