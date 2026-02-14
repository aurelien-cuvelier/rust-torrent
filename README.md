# Rust Torrent Client (Prototype)

A prototype BitTorrent client written in Rust. Currently in early development.

## ⚠️ Disclaimer

This is a **personal learning project** created to:
- Learn more about the Rust programming language
- Understand how BitTorrent protocol works
- Build a functional torrent client from scratch

**This project should NOT be used in production.** It is for educational purposes only and may contain bugs, incomplete error handling, and is not optimized for real-world use.

## Current Status

**Phase 1: Bencode Parsing** ✅ (In Progress)
- Bencode format parser (the encoding used by BitTorrent)
- Torrent file structure parsing (`.torrent` files)
- Parsing torrent metadata (announce URLs, file info, pieces, etc.)

**Bencode parser design**
- **Trait-based and reusable:** Any type can be parsed from bencode as long as it implements `BencodeParsable`. Each type has an associated key type implementing `BencodeKey` (key names and field shapes: string, integer, binary, list, nested list, dictionary).
- **Multiple parsable types:** The same decoder is used for:
  - **Torrent files** (`.torrent`) — `TorrentFile` and nested `MetaInfo`, read from `File`
  - **Tracker responses** — `TrackerData` (e.g. interval, peers), read from `Cursor<Vec<u8>>`
- **Unknown keys:** Keys that are not handled (e.g. `private`, `publisher`) are mapped to an “unsupported” key variant so their data is consumed and parsing continues instead of failing.

**Future Phases:**
- Tracker communication
- Peer discovery and connection
- Piece downloading and verification
- File assembly

## Project Structure

- `src/bencode.rs` - Bencode format parser and `BencodeParsable` / `BencodeKey` traits
- `src/torrent_file.rs` - Torrent file data structures (`TorrentFile`, `MetaInfo`) and key enums
- `src/tracker_data.rs` - Tracker response parsing (`TrackerData`)
- `src/torrent_net.rs` - Tracker/network helpers
- `src/client.rs` - Client identifier and shared state
- `src/main.rs` - Entry point

## Usage

```bash
cargo run
```

## Learning Goals

- Understanding BitTorrent protocol specification
- Bencode format parsing
- Working with binary file parsing in Rust
- Network programming (tracker communication, peer connections)
- Implementing recursive parsers
- Working with `BufReader` and file I/O
- Rust ownership and borrowing concepts
- Error handling patterns in Rust
- Concurrent programming (for handling multiple peers)

## License

This is a personal learning project. Use at your own risk.
