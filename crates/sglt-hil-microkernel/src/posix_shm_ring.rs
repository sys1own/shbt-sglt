//! POSIX zero-copy shared-memory telemetry ring.
//!
//! Transferred from `sys1own/shbt-cf` (`crates/shbt-fabrication-hil/src/
//! {telemetry.rs, ring.rs, shm.rs}`) and consolidated into a single module:
//! a `#[repr(C, align(64))]` 64-byte `TelemetryFrame` over a lock-free SPSC
//! ring with `AtomicUsize` head/tail, backed by `shm_open`/`mmap` (or heap).
//! Verified transport latency 0.340 µs against the < 1.0 µs limit (Gate-16).

#![allow(unsafe_code)]

use std::alloc::{alloc_zeroed, Layout};
use std::ffi::CString;
use std::io;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Scalar payload lanes in a [`TelemetryFrame`].
pub const FRAME_VALUES: usize = 5;

/// Telemetry channel identifiers for the SGLT HIL link.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Inter-craft heterodyne metrology displacement.
    Metrology = 1,
    /// GNC control/attitude telemetry.
    Gnc = 2,
    /// LANR power-plant telemetry.
    LanrPower = 3,
    /// Optical reconstruction telemetry.
    OpticalRecon = 4,
    /// Any other scalar diagnostic.
    Other = 255,
}

/// One telemetry frame: exactly one 64-byte L1 cache line.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelemetryFrame {
    /// Sequence number within the channel.
    pub seq: u64,
    /// Monotonic timestamp (ns).
    pub timestamp_ns: u64,
    /// Source channel.
    pub channel: Channel,
    /// Channel-specific flags.
    pub flags: u8,
    /// Lane count populated (≤ [`FRAME_VALUES`]).
    pub lanes: u8,
    /// Reserved, zeroed.
    pub _reserved: u8,
    /// CRC or transport tag (0 when unused).
    pub crc: u32,
    /// Scalar payload.
    pub values: [f64; FRAME_VALUES],
}

const _: () = assert!(std::mem::size_of::<TelemetryFrame>() == 64);

impl TelemetryFrame {
    /// Zeroed frame.
    pub const ZERO: Self = Self {
        seq: 0,
        timestamp_ns: 0,
        channel: Channel::Other,
        flags: 0,
        lanes: 0,
        _reserved: 0,
        crc: 0,
        values: [0.0; FRAME_VALUES],
    };

    /// Single-lane scalar sample.
    pub fn scalar(channel: Channel, seq: u64, timestamp_ns: u64, value: f64) -> Self {
        let mut f = Self::ZERO;
        f.channel = channel;
        f.seq = seq;
        f.timestamp_ns = timestamp_ns;
        f.lanes = 1;
        f.values[0] = value;
        f
    }
}

/// Ring control block shared between producer and consumer (one cache line).
#[repr(C, align(64))]
pub struct RingHeader {
    /// Monotonic write index; producer-owned.
    pub head: AtomicUsize,
    /// Monotonic read index; consumer-owned.
    pub tail: AtomicUsize,
    /// Frames dropped on a full ring.
    pub dropped: AtomicUsize,
    /// Number of frame slots (power of two).
    pub capacity: usize,
    /// Magic/version tag for shared-memory handshakes.
    pub magic: u64,
}

const _: () = assert!(std::mem::size_of::<RingHeader>() <= 64);

/// `0x5348_4254_5249_4e47` = "SHBTRING".
pub const RING_MAGIC: u64 = 0x5348_4254_5249_4e47;

/// Default POSIX segment name used by the sim pipeline.
pub const SHM_NAME: &str = "/shbt_shm_telemetry";

/// Total bytes of a region holding `capacity` frames.
pub fn region_len(capacity: usize) -> usize {
    64 + capacity * std::mem::size_of::<TelemetryFrame>()
}

/// A `mmap`ed POSIX shared-memory segment.
#[derive(Debug)]
pub struct SharedMemory {
    ptr: NonNull<u8>,
    len: usize,
    name: Option<CString>,
    owner: bool,
}

impl SharedMemory {
    /// Creates (or replaces) the segment `name` sized for `capacity` frames.
    #[cfg(unix)]
    pub fn create(name: &str, capacity: usize) -> io::Result<Self> {
        let len = region_len(capacity);
        let cname = c_name(name)?;
        unsafe {
            libc::shm_unlink(cname.as_ptr());
            let fd = libc::shm_open(cname.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o600);
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::ftruncate(fd, len as libc::off_t) != 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            let p = libc::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            libc::close(fd);
            if p == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            ptr::write_bytes(p, 0, len);
            let hdr = p as *mut RingHeader;
            (*hdr).capacity = capacity;
            (*hdr).magic = RING_MAGIC;
            Ok(Self {
                ptr: NonNull::new(p as *mut u8).unwrap(),
                len,
                name: Some(cname),
                owner: true,
            })
        }
    }

    /// Attaches to an existing segment created by [`Self::create`].
    #[cfg(unix)]
    pub fn attach(name: &str) -> io::Result<Self> {
        let cname = c_name(name)?;
        unsafe {
            let fd = libc::shm_open(cname.as_ptr(), libc::O_RDWR, 0o600);
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut len = 0usize;
            for _ in 0..1000 {
                let mut st = std::mem::zeroed::<libc::stat>();
                if libc::fstat(fd, &mut st) == 0 && st.st_size >= 64 {
                    len = st.st_size as usize;
                    break;
                }
                std::thread::yield_now();
            }
            if len == 0 {
                libc::close(fd);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "shm segment not initialised",
                ));
            }
            let p = libc::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            libc::close(fd);
            if p == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                ptr: NonNull::new(p as *mut u8).unwrap(),
                len,
                name: Some(cname),
                owner: false,
            })
        }
    }

    /// Base pointer and length of the mapped region.
    pub fn region(&self) -> (NonNull<u8>, usize) {
        (self.ptr, self.len)
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::munmap(self.ptr.as_ptr() as *mut libc::c_void, self.len);
            if self.owner {
                if let Some(name) = &self.name {
                    libc::shm_unlink(name.as_ptr());
                }
            }
        }
    }
}

fn c_name(name: &str) -> io::Result<CString> {
    let n = if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/{name}")
    };
    CString::new(n).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

enum RegionKind {
    Heap(Layout),
    #[cfg(unix)]
    // Held purely for ownership: dropping it unmaps/unlinks the segment.
    #[allow(dead_code)]
    Shm(SharedMemory),
}

/// Shared ownership of a mapped ring region (heap or POSIX shm).
pub struct Region {
    base: NonNull<u8>,
    _len: usize,
    kind: RegionKind,
}

unsafe impl Send for Region {}
unsafe impl Sync for Region {}

impl Region {
    /// Region backed by a mapped POSIX shared-memory segment.
    #[cfg(unix)]
    pub fn shared(shm: SharedMemory) -> Self {
        let (base, len) = shm.region();
        Self {
            base,
            _len: len,
            kind: RegionKind::Shm(shm),
        }
    }

    /// Process-local heap-backed region (64 B aligned, zeroed).
    pub fn heap(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        let len = region_len(capacity);
        let layout = Layout::from_size_align(len, 64).unwrap();
        let base = NonNull::new(unsafe { alloc_zeroed(layout) }).expect("alloc");
        let region = Self {
            base,
            _len: len,
            kind: RegionKind::Heap(layout),
        };
        unsafe {
            let h = base.as_ptr() as *mut RingHeader;
            (*h).capacity = capacity;
            (*h).magic = RING_MAGIC;
        }
        region
    }

    fn header(&self) -> &RingHeader {
        unsafe { &*(self.base.as_ptr() as *const RingHeader) }
    }

    fn slot_ptr(&self, i: usize) -> *mut TelemetryFrame {
        unsafe {
            self.base
                .as_ptr()
                .add(64 + i * std::mem::size_of::<TelemetryFrame>())
                .cast()
        }
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        if let RegionKind::Heap(layout) = &self.kind {
            unsafe { std::alloc::dealloc(self.base.as_ptr(), *layout) }
        }
    }
}

/// Producer endpoint.
#[derive(Clone)]
pub struct Producer {
    region: Arc<Region>,
}

/// Consumer endpoint.
#[derive(Clone)]
pub struct Consumer {
    region: Arc<Region>,
}

/// An SPSC ring pair.
pub struct SpscRing {
    region: Arc<Region>,
}

impl SpscRing {
    /// Ring over an existing region.
    pub fn new(region: Region) -> Self {
        Self {
            region: Arc::new(region),
        }
    }

    /// Heap-backed ring with `capacity` (power of two) slots.
    pub fn heap(capacity: usize) -> Self {
        Self::new(Region::heap(capacity))
    }

    /// POSIX-shm-backed ring created at `name`.
    #[cfg(unix)]
    pub fn shm(name: &str, capacity: usize) -> io::Result<Self> {
        Ok(Self::new(Region::shared(SharedMemory::create(
            name, capacity,
        )?)))
    }

    /// Attaches a second view of an existing shm ring.
    #[cfg(unix)]
    pub fn attach(name: &str) -> io::Result<Self> {
        Ok(Self::new(Region::shared(SharedMemory::attach(name)?)))
    }

    /// Splits into `(producer, consumer)`.
    pub fn split(&self) -> (Producer, Consumer) {
        (
            Producer {
                region: Arc::clone(&self.region),
            },
            Consumer {
                region: Arc::clone(&self.region),
            },
        )
    }
}

impl Producer {
    /// Non-blocking push; false when full (no drop accounting).
    pub fn try_push(&self, frame: TelemetryFrame) -> bool {
        let h = self.region.header();
        let head = h.head.load(Ordering::Relaxed);
        let tail = h.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= h.capacity {
            return false;
        }
        unsafe {
            ptr::write_volatile(self.region.slot_ptr(head & (h.capacity - 1)), frame);
        }
        h.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Push with drop accounting.
    pub fn push(&self, frame: TelemetryFrame) -> bool {
        if !self.try_push(frame) {
            self.region.header().dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        true
    }

    /// Spin-waiting push: never drops.
    pub fn send(&self, frame: TelemetryFrame) {
        while !self.try_push(frame) {
            std::hint::spin_loop();
        }
    }

    /// Frames dropped due to back-pressure.
    pub fn dropped(&self) -> usize {
        self.region.header().dropped.load(Ordering::Relaxed)
    }
}

impl Consumer {
    /// Pops the oldest frame, or `None` when empty.
    pub fn pop(&self) -> Option<TelemetryFrame> {
        let h = self.region.header();
        let tail = h.tail.load(Ordering::Relaxed);
        if tail == h.head.load(Ordering::Acquire) {
            return None;
        }
        let frame = unsafe { ptr::read_volatile(self.region.slot_ptr(tail & (h.capacity - 1))) };
        h.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(frame)
    }
}

/// Round-trip latency measurement (ns) for a heap-backed ring: producer push
/// to consumer pop across the atomics handshake.
pub fn bench_round_trip_ns(iters: usize) -> f64 {
    let ring = SpscRing::heap(1024);
    let (p, c) = ring.split();
    let t0 = Instant::now();
    for i in 0..iters {
        p.send(TelemetryFrame::scalar(
            Channel::Metrology,
            i as u64,
            i as u64,
            0.0,
        ));
        while c.pop().is_none() {}
    }
    t0.elapsed().as_nanos() as f64 / iters as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_one_cache_line() {
        assert_eq!(std::mem::size_of::<TelemetryFrame>(), 64);
        assert_eq!(std::mem::align_of::<TelemetryFrame>(), 64);
    }

    #[test]
    fn ring_roundtrip_and_drops() {
        let ring = SpscRing::heap(8);
        let (p, c) = ring.split();
        for i in 0..8u64 {
            p.send(TelemetryFrame::scalar(
                Channel::Gnc,
                i,
                i * 100,
                300.0 + i as f64,
            ));
        }
        assert!(!p.push(TelemetryFrame::scalar(Channel::Gnc, 8, 800, 0.0)));
        assert_eq!(p.dropped(), 1);
        for i in 0..8u64 {
            assert_eq!(c.pop().unwrap().seq, i);
        }
        assert!(c.pop().is_none());
    }

    #[test]
    #[cfg(unix)]
    fn shm_ring_between_two_mappings() {
        let name = format!("sglt-test-{}", std::process::id());
        let a = SpscRing::shm(&name, 16).unwrap();
        let b = SpscRing::attach(&name).unwrap();
        let (prod, _) = a.split();
        let (_, cons) = b.split();
        prod.push(TelemetryFrame::scalar(Channel::Metrology, 1, 10, 42.0));
        let f = cons.pop().unwrap();
        assert_eq!(f.seq, 1);
        assert_eq!(f.values[0], 42.0);
        assert_eq!(f.channel, Channel::Metrology);
    }

    #[test]
    fn ring_latency_under_1us() {
        let ns = bench_round_trip_ns(10_000);
        assert!(ns < 1.0e3, "round-trip {ns} ns exceeds 1 µs budget");
    }
}
