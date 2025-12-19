# Audio Merge 🎛️

A high-performance Windows audio routing tool built with **Rust** (Tauri) and **React**. 
Seamlessly mirror your system audio to multiple output devices simultaneously (e.g., Speakers + Headphones + TV) with zero-config loopback capture.

## Features

- 🎧 **Auto-Source**: Automatically captures "What You Hear" (System Default Loopback).
- 🔊 **Multi-Output**: Add unlimited output devices to the mix.
- 🎚️ **Volume Control**: Independent volume sliders for each output.
- 🚀 **Low Latency**: Uses `cpal` and lock-free RingBuffers (`rtrb`) for real-time audio.
- 🛡️ **Thread Safe**: Actor model architecture prevents UI freezes and audio glitches.
- 💅 **Premium UI**: Glassmorphism design with Dark Mode by default.

## Tech Stack

- **Frontend**: React, TypeScript, Vite
- **Backend**: Rust (Tauri v2)
- **Audio**: `cpal` (WASAPI), `rtrb` (RingBuffer)

## Prerequisites

- **Windows 10/11** (Required for WASAPI Loopback)
- [Node.js](https://nodejs.org/) (v16+)
- [Rust](https://rustup.rs/) (stable)

## Getting Started

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Run in Development Mode**:
   ```bash
   npm run tauri dev
   ```
   This will start the React dev server and compile the Rust backend.

3. **Build for Production**:
   ```bash
   npm run tauri build
   ```
   The executable will be in `src-tauri/target/release/bundle/nsis/`.

## Architecture

The app uses an **Actor Model** pattern:
1.  **Main Thread (UI)**: Sends commands (`StartLoopback`, `AddOutput`, `SetVolume`) via a `crossbeam-channel`.
2.  **Audio Thread**: A dedicated thread that owns the Audio Streams. It handles the `cpal` events and manages the `RingBuffers`.
    - **Capture**: Reads samples from the system loopback.
    - **Broadcast**: Pushes samples to a list of active `Producers`.
    - **Playback**: Each output device has a `Consumer` that pulls audio from its buffer and plays it.

## License

MIT
