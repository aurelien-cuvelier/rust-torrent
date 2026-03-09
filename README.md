# Rust Torrent Client (Prototype)

A prototype BitTorrent client written in Rust. Currently in development as a learning project.

## ⚠️ Disclaimer

This is a **personal learning project** created to:
- Learn more about the Rust programming language
- Understand how the BitTorrent protocol works
- Build a functional torrent client from scratch

**This project should NOT be used in production.** It is for educational purposes only and may contain bugs, incomplete error handling, and is not optimized for real-world use.

## Current Status

**Implemented**

- **Bencode parsing** — Trait-based decoder for torrent files and tracker responses; supports `TorrentFile`, `MetaInfo`, `TrackerData`; unknown keys are consumed via an “unsupported” variant so parsing continues.
- **Torrent file parsing** — Parsing of `.torrent` metadata (announce, info hash, piece length, piece hashes, etc.).
- **Tracker communication** — HTTP GET announce with compact peer list; parses interval and peer list from the tracker response.
- **Peer connections** — TCP connect, 68-byte handshake (info hash + peer ID), info-hash validation.
- **Peer wire protocol** — Choke/Unchoke, Interested/NotInterested, Bitfield, Have, Request, Piece; keep-alive; synchronous message loop per connection.
- **Multi-peer download** — One thread per peer (configurable max, e.g. 50); shared `FileHandler` via `Arc<Mutex<>>`; each connection runs handshake, sends bitfield, then message loop.
- **Piece and block handling** — Request blocks (16 KiB default); receive Piece messages; reassemble per-piece buffer; SHA1 verification against torrent piece hashes; write verified pieces to file.
- **File handling** — Single-file torrents: create/open file under `./downloads/`, compute local bitfield from existing data, maintain `needed_pieces` queue, write verified pieces at the correct offset.

**Limitations / not implemented**

- Multi-file torrents.
- DHT, PEX, or other peer discovery beyond the initial tracker response.
- Async I/O; everything is blocking and thread-per-peer.
- Robust error recovery (e.g. re-request failed pieces, peer disconnect handling).
- Progress reporting beyond debug logs.

## Project Structure

```
src/
├── main.rs              # Entry point: .torrent path → tracker → spawn peer threads
├── lib.rs               # Library root
├── client.rs            # Client peer ID and shared constants
├── tracker.rs           # Tracker HTTP announce, handshake bytes, spawn connection threads
├── tracker_data.rs      # Tracker response parsing (TrackerData)
├── file_handler.rs      # File I/O, bitfield, needed_pieces, write_piece_to_file
├── bencode/
│   ├── mod.rs           # Bencode types and trait wiring
│   └── decode.rs        # Bencode decoding (BencodeParsable, BencodeKey)
├── torrent_file/
│   ├── mod.rs           # TorrentFile
│   └── meta_info.rs     # MetaInfo and related structures
└── connection_handler/
    ├── mod.rs           # ConnectionHandler: handshake, message loop, request/plan logic
    ├── message.rs       # MessageType, Piece struct, REQUEST_PIECE_SIZE
    └── handlers.rs      # handle_new_piece, handle_request_piece, handle_bitfield, handle_have
```

## Usage

Pass a single `.torrent` file path. Downloaded content is written under `./downloads/` using the torrent’s `name` as the filename.

```bash
# Optional: set log level (default is info; use debug for protocol/piece detail)
export RUST_LOG=debug

cargo run -- /path/to/file.torrent
```

Example:

```bash
cargo run -- ./path/to/ubuntu-24.04.torrent
```

## Learning Goals

- BitTorrent protocol (handshake, peer wire messages, piece/block semantics)
- Bencode format and reusable parsing with traits
- Binary and file I/O in Rust
- Network programming (TCP, HTTP tracker, blocking I/O)
- Concurrency (thread-per-peer, shared state with `Arc<Mutex<>>`)
- Ownership, borrowing, and reborrowing in Rust
- Error handling and logging

## License

This is a personal learning project. Use at your own risk.
