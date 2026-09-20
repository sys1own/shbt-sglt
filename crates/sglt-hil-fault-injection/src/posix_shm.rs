//! POSIX shared-memory frame buffer /dev/shm/sglt_frame_buffer at 100 kHz
//! (up1.txt §6). GATE-24: SHM latency ≤ 10 µs at 100 kHz.

use std::io::{Read, Seek, SeekFrom, Write};

pub const SHM_PATH: &str = "/dev/shm/sglt_frame_buffer";
pub const FRAME_RATE_HZ: f64 = 100e3;
pub const FRAME_BYTES: usize = 2112;

/// RAII wrapper over the POSIX shm object backing `.stinespring_frame`.
pub struct ShmFrameBuffer {
    file: std::fs::File,
}

impl ShmFrameBuffer {
    /// Create or open the shared frame buffer and size it to 2112 B.
    pub fn open() -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(SHM_PATH)?;
        file.set_len(FRAME_BYTES as u64)?;
        Ok(Self { file })
    }

    /// Write a full 2112-byte frame; returns wall-clock latency in µs.
    pub fn write_frame(&mut self, frame: &[u8; FRAME_BYTES]) -> std::io::Result<f64> {
        let t0 = std::time::Instant::now();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(frame)?;
        self.file.sync_data()?;
        Ok(t0.elapsed().as_secs_f64() * 1e6)
    }

    /// Read the current frame.
    pub fn read_frame(&mut self) -> std::io::Result<[u8; FRAME_BYTES]> {
        let mut buf = [0u8; FRAME_BYTES];
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }
}
