//! sglt-target-observatory — exoplanet science suite (up1.txt §7):
//! atmospheric transmission spectroscopy for {H₂O, O₂, CO₂, CH₄, O₃},
//! coronagraphic speckle suppression, FITS v4.0 writer, HDF5 exporter.

pub mod exoplanet_atm;
pub mod fits_writer;
pub mod hdf5_exporter;
pub mod spectra_gen;

pub use exoplanet_atm::{Atmosphere, transit_depth};
pub use fits_writer::FitsWriter;
pub use hdf5_exporter::Hdf5Exporter;
pub use spectra_gen::{molecular_cross_section, Species};

/// `repr(C)` mirror of `sglt_fits_export_t` in `include/sglt_v2_abi.h`.
#[repr(C)]
pub struct FitsExportC {
    pub num_wavelength_channels: u32,
    pub spatial_dim_x: u32,
    pub spatial_dim_y: u32,
    pub datacube_ptr: *mut f32,
    pub contrast_rejection_ratio: f64,
}

/// Write a datacube to a FITS v4.0 file. Uses the built-in minimalist FITS
/// writer (valid FITS primary HDU); the `fits` feature swaps in `fitsio`.
///
/// # Safety
/// `filepath` must be a NUL-terminated C string; `export_data` non-null.
#[no_mangle]
pub unsafe extern "C" fn sglt_fits_export_file(
    filepath: *const libc::c_char,
    export_data: *const FitsExportC,
) -> i32 {
    if filepath.is_null() || export_data.is_null() {
        return -1;
    }
    let path = match std::ffi::CStr::from_ptr(filepath).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let d = &*export_data;
    let n = (d.num_wavelength_channels as usize)
        * (d.spatial_dim_x as usize)
        * (d.spatial_dim_y as usize);
    if d.datacube_ptr.is_null() {
        return -3;
    }
    let data = std::slice::from_raw_parts(d.datacube_ptr, n);
    match FitsWriter::write_datacube(
        path,
        data,
        d.num_wavelength_channels as usize,
        d.spatial_dim_y as usize,
        d.spatial_dim_x as usize,
    ) {
        Ok(()) => 0,
        Err(_) => -4,
    }
}
