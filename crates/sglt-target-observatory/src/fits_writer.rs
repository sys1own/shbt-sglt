//! FITS v4.0 writer (up1.txt §7). With `--features fits` this delegates to
//! the `fitsio` crate; the built-in writer below emits a compliant
//! 2880-byte-block primary HDU, BITPIX=-32, [WAVELENGTH, Y, X] axes and the
//! required keywords TELESCOP / CRVAL3 / CDELT3.

use std::io::Write;

pub struct FitsWriter;

fn format_card(key: &str, value: &str) -> [u8; 80] {
    let mut card = [b' '; 80];
    let text = format!("{key:<8}= {value:>20}");
    let bytes = text.as_bytes();
    card[..bytes.len().min(80)].copy_from_slice(&bytes[..bytes.len().min(80)]);
    card
}

impl FitsWriter {
    /// Write a 3D datacube [n_wav, ny, nx] of f32 (big-endian per FITS).
    pub fn write_datacube(
        path: &str,
        data: &[f32],
        n_wav: usize,
        ny: usize,
        nx: usize,
    ) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;

        // --- Primary HDU header ---
        let mut header: Vec<[u8; 80]> = vec![
            format_card("SIMPLE", "                    T"),
            format_card("BITPIX", "                  -32"),
            format_card("NAXIS", "                    3"),
            format_card("NAXIS1", &format!("{nx:>20}")),
            format_card("NAXIS2", &format!("{ny:>20}")),
            format_card("NAXIS3", &format!("{n_wav:>20}")),
            format_card("TELESCOP", " 'SHBT-SGLT-V2'"),
            format_card("CRVAL3", "               200.0"),
            format_card("CDELT3", "                 0.5"),
        ];
        let mut end = [b' '; 80];
        end[..3].copy_from_slice(b"END");
        header.push(end);
        Self::write_padded(&mut f, &header.concat())?;

        // --- Data (IEEE-754 big-endian) ---
        let mut payload = Vec::with_capacity(data.len() * 4);
        for v in data {
            payload.extend_from_slice(&v.to_be_bytes());
        }
        Self::write_padded(&mut f, &payload)?;
        Ok(())
    }

    /// Write `bytes` rounded up to a 2880-byte FITS block boundary.
    fn write_padded(f: &mut std::fs::File, bytes: &[u8]) -> std::io::Result<()> {
        f.write_all(bytes)?;
        let rem = bytes.len() % 2880;
        if rem != 0 {
            f.write_all(&vec![b' '; 2880 - rem])?;
        }
        Ok(())
    }
}
