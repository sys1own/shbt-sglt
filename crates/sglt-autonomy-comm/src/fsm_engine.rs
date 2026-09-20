//! Deterministic 8-phase mission autonomy FSM (up1.txt §5 state matrix).

/// Mission phases P0–P7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MissionPhase {
    Launch = 0,
    Transit = 1,
    Align = 2,
    Acquire = 3,
    Science = 4,
    Downlink = 5,
    Safe = 6,
    Extended = 7,
}

/// Telemetry snapshot evaluated by the transition matrix.
#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry {
    pub health_ok: bool,
    pub distance_au: f64,
    pub transverse_dev_m: f64,
    pub einstein_ring_snr: f64,
    pub frame_fill_frac: f64,
    pub downlink_ack: bool,
    pub thermal_k: f64,
    pub sram_corrupt_rate_s: f64,
    pub ground_override: bool,
}

/// Active FSM with anomaly rollback. GATE-18: 100 % coverage of all 8
/// phases; GATE-19: rollback to Safe executes in < 100 ms.
pub struct MissionStateMachine {
    pub phase: MissionPhase,
    pub phase_entered_s: f64,
    pub rollback_count: u32,
}

impl Default for MissionStateMachine {
    fn default() -> Self {
        Self {
            phase: MissionPhase::Launch,
            phase_entered_s: 0.0,
            rollback_count: 0,
        }
    }
}

impl MissionStateMachine {
    /// Evaluate the transition matrix against `tm` at time `t_s`.
    /// Any-state anomaly triggers (T > 6.5 K or SRAM corrupt > 100/s) force
    /// Safe mode with cold isolation.
    pub fn tick(&mut self, tm: &Telemetry, t_s: f64) -> MissionPhase {
        use MissionPhase::*;
        let in_phase_s = t_s - self.phase_entered_s;

        // ANY STATE → P6 anomaly guard (highest priority, "instant").
        if tm.thermal_k > 6.5 || tm.sram_corrupt_rate_s > 100.0 {
            if self.phase != Safe {
                self.phase = Safe;
                self.rollback_count += 1;
            }
            return self.phase;
        }

        let next = match self.phase {
            // P0→P1: fairing sep + array deploy + health, timeout 12 h
            Launch if tm.health_ok => Transit,
            // P1→P2: r ≥ 547.8 AU & SGL image lock, timeout 120 d
            Transit if tm.distance_au >= 547.8 => Align,
            // P2→P3: ρ ≤ 1.0 m from image axis, timeout 48 h
            Align if tm.transverse_dev_m <= 1.0 => Acquire,
            // P3→P4: Einstein-ring centroid lock SNR ≥ 25, timeout 6 h
            Acquire if tm.einstein_ring_snr >= 25.0 => Science,
            // P4→P5: integration array complete, timeout 0.5 h
            Science if tm.frame_fill_frac >= 1.0 => Downlink,
            // P5→P2: telemetry ACK, timeout 1 h → re-transmit
            Downlink if tm.downlink_ack => Align,
            // P6→P7: diagnostics pass + ground override, timeout 72 h
            Safe if tm.ground_override && tm.health_ok => Extended,
            // Phase timeout → spec rollback action → Safe is the safe sink
            _ if self.timed_out(in_phase_s) => Safe,
            _ => return self.phase,
        };
        self.phase = next;
        self.phase_entered_s = t_s;
        next
    }

    fn timed_out(&self, elapsed_s: f64) -> bool {
        let hours = elapsed_s / 3600.0;
        let limit = match self.phase {
            MissionPhase::Launch => 12.0,
            MissionPhase::Transit => 120.0 * 24.0,
            MissionPhase::Align => 48.0,
            MissionPhase::Acquire => 6.0,
            MissionPhase::Science => 0.5,
            MissionPhase::Downlink => 1.0,
            MissionPhase::Safe => 72.0,
            MissionPhase::Extended => f64::INFINITY,
        };
        hours > limit
    }
}
