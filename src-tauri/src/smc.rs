// SMC (System Management Controller) communication module for Apple Silicon Macs.
// Uses IOKit framework to read/write SMC keys for battery charging control.

use std::ffi::CString;
use std::io;

// ── IOKit FFI declarations ──────────────────────────────────────────────────

#[cfg(target_os = "macos")]
extern "C" {
    fn IOServiceMatching(name: *const std::os::raw::c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingService(master_port: u32, matching: CFMutableDictionaryRef) -> u32;
    fn IOServiceOpen(service: u32, task: u32, type_: u32, connect: *mut u32) -> i32;
    fn IOConnectCallMethod(
        connection: u32,
        selector: u32,
        input_scalars: *const u64,
        input_scalars_cnt: u32,
        input_struct: *const u8,
        input_struct_cnt: usize,
        output_scalars: *mut u64,
        output_scalars_cnt: *mut u32,
        output_struct: *mut u8,
        output_struct_cnt: *mut usize,
    ) -> i32;
    fn IOConnectCallStructMethod(
        connection: u32,
        selector: u32,
        input: *const u8,
        input_size: usize,
        output: *mut u8,
        output_size: *mut usize,
    ) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOServiceClose(connect: u32) -> i32;
    fn mach_task_self() -> u32;
}

type CFMutableDictionaryRef = *mut std::ffi::c_void;

// ── SMC Constants ───────────────────────────────────────────────────────────

const K_SMC_USER_CLIENT_OPEN: u8 = 0;
const K_SMC_USER_CLIENT_CLOSE: u8 = 1;
const K_SMC_HANDLE_YPC_EVENT: u8 = 2;
const K_SMC_READ_KEY: u8 = 5;
const K_SMC_WRITE_KEY: u8 = 6;
const K_SMC_GET_KEY_INFO: u8 = 9;
const K_SMC_SUCCESS: u8 = 0;

// ── SMC Data Structures ─────────────────────────────────────────────────────

/// Key metadata returned by SMC for each key.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SMCKeyInfoData {
    pub data_size: u32,
    pub data_type: u32,
    pub data_attributes: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SMCVersion {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SMCPLimitData {
    version: u16,
    length: u16,
    cpu_p_limit: u32,
    gpu_p_limit: u32,
    mem_p_limit: u32,
}

/// 80-byte parameter struct passed to IOConnectCallStructMethod for all SMC operations.
/// This mirrors the original AppleSMC C layout exactly.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SMCParamStructRaw {
    key: u32,
    vers: SMCVersion,
    p_limit_data: SMCPLimitData,
    key_info: SMCKeyInfoData,
    result: u8,
    status: u8,
    data8: u8,
    _padding0: u8,
    data32: u32,
    bytes: [u8; 32],
}

// Compile-time size check
const _: () = assert!(std::mem::size_of::<SMCParamStructRaw>() == 80);

impl SMCParamStructRaw {
    fn to_key_info(&self) -> SMCKeyInfoData {
        self.key_info
    }

    fn set_key_info(&mut self, info: &SMCKeyInfoData) {
        self.key_info = *info;
    }
}

// ── Public SMC Handle ───────────────────────────────────────────────────────

/// Thread-safe handle to an open SMC connection.
pub struct SmcHandle {
    connect: u32,
}

// IOConnectCallStructMethod is safe to call from multiple threads when using
// separate input/output buffers, but to be safe we use a Mutex in the caller.

impl SmcHandle {
    /// Open a new connection to the Apple SMC.
    #[cfg(target_os = "macos")]
    pub fn open() -> io::Result<Self> {
        let connect = smc_open()?;
        Ok(Self { connect })
    }

    #[cfg(not(target_os = "macos"))]
    pub fn open() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SMC is only available on macOS",
        ))
    }

    /// Read an SMC key. Returns (data_bytes, key_info).
    #[allow(dead_code)]
    pub fn read_key(&self, key: &[u8; 4]) -> io::Result<(Vec<u8>, SMCKeyInfoData)> {
        smc_read_key(self.connect, key)
    }

    /// Write data to an SMC key. Verifies by reading back.
    pub fn write_key(&self, key: &[u8; 4], data: &[u8]) -> io::Result<()> {
        smc_write_key(self.connect, key, data)
    }

    /// Get metadata for an SMC key.
    pub fn get_key_info(&self, key: &[u8; 4]) -> io::Result<SMCKeyInfoData> {
        smc_get_key_info(self.connect, key)
    }
}

impl Drop for SmcHandle {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        smc_close(self.connect);
    }
}

// ── Internal IOKit Functions ────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn smc_open() -> io::Result<u32> {
    unsafe {
        let name = CString::new("AppleSMC").unwrap();
        let matching = IOServiceMatching(name.as_ptr());
        if matching.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "IOServiceMatching returned null",
            ));
        }

        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "AppleSMC service not found",
            ));
        }

        let mut connect: u32 = 0;
        let kr = IOServiceOpen(service, mach_task_self(), 1, &mut connect);
        IOObjectRelease(service);

        if kr != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("IOServiceOpen failed: 0x{:x}", kr),
            ));
        }

        // Send kSMCUserClientOpen
        let kr = IOConnectCallMethod(
            connect,
            K_SMC_USER_CLIENT_OPEN as u32,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if kr != 0 {
            IOServiceClose(connect);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("kSMCUserClientOpen failed: 0x{:x}", kr),
            ));
        }

        Ok(connect)
    }
}

#[cfg(target_os = "macos")]
fn smc_close(connect: u32) {
    unsafe {
        let _ = IOConnectCallMethod(
            connect,
            K_SMC_USER_CLIENT_CLOSE as u32,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        IOServiceClose(connect);
    }
}

/// Perform an SMC call: takes an input param struct, returns the output param struct.
#[cfg(target_os = "macos")]
fn smc_call(connect: u32, input: &SMCParamStructRaw) -> io::Result<SMCParamStructRaw> {
    unsafe {
        let mut output = *input;
        let mut out_size = std::mem::size_of::<SMCParamStructRaw>();
        let kr = IOConnectCallStructMethod(
            connect,
            K_SMC_HANDLE_YPC_EVENT as u32,
            input as *const _ as *const u8,
            std::mem::size_of::<SMCParamStructRaw>(),
            &mut output as *mut _ as *mut u8,
            &mut out_size,
        );
        if kr != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("IOConnectCallStructMethod failed: 0x{:x}", kr),
            ));
        }
        Ok(output)
    }
}

/// Get key info (data size and type) for an SMC key.
#[cfg(target_os = "macos")]
fn smc_get_key_info(connect: u32, key: &[u8; 4]) -> io::Result<SMCKeyInfoData> {
    let key_u32 = u32::from_be_bytes(*key);
    let mut param = SMCParamStructRaw::default();
    param.key = key_u32;
    param.data8 = K_SMC_GET_KEY_INFO;

    let result = smc_call(connect, &param)?;
    if result.result != K_SMC_SUCCESS {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "SMC GetKeyInfo for {:?} returned error: {}",
                key, result.result
            ),
        ));
    }
    Ok(result.to_key_info())
}

/// Read an SMC key. Returns the data bytes and key info.
#[cfg(target_os = "macos")]
fn smc_read_key(connect: u32, key: &[u8; 4]) -> io::Result<(Vec<u8>, SMCKeyInfoData)> {
    let key_info = smc_get_key_info(connect, key)?;
    let key_u32 = u32::from_be_bytes(*key);

    let mut param = SMCParamStructRaw::default();
    param.key = key_u32;
    param.data8 = K_SMC_READ_KEY;
    param.set_key_info(&key_info);

    let result = smc_call(connect, &param)?;
    if result.result != K_SMC_SUCCESS {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("SMC ReadKey {:?} returned error: {}", key, result.result),
        ));
    }

    let size = key_info.data_size as usize;
    if size > 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("SMC key data size {} exceeds buffer", size),
        ));
    }

    let data = result.bytes[..size].to_vec();
    Ok((data, key_info))
}

/// Write data to an SMC key, then verify by reading back.
#[cfg(target_os = "macos")]
fn smc_write_key(connect: u32, key: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let key_info = smc_get_key_info(connect, key)?;
    let key_u32 = u32::from_be_bytes(*key);

    if data.len() > 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Data exceeds 32-byte SMC buffer",
        ));
    }

    let mut param = SMCParamStructRaw::default();
    param.key = key_u32;
    param.data8 = K_SMC_WRITE_KEY;
    param.set_key_info(&key_info);
    param.bytes[..data.len()].copy_from_slice(data);

    let result = smc_call(connect, &param)?;
    if result.result != K_SMC_SUCCESS {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("SMC WriteKey {:?} returned error: {}", key, result.result),
        ));
    }

    // Verify write by reading back
    let (read_data, _) = smc_read_key(connect, key)?;
    if read_data.len() != data.len() || read_data[..] != *data {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "SMC WriteKey {:?} verification failed: wrote {:?}, read {:?}",
                key, data, read_data
            ),
        ));
    }

    Ok(())
}

// ── Battery-related SMC Key Helpers ─────────────────────────────────────────

/// Well-known SMC keys as byte arrays.
pub mod keys {
    pub const CHTE: &[u8; 4] = b"CHTE"; // Charging control (ui32, 4 bytes)
    pub const CH0C: &[u8; 4] = b"CH0C"; // Charging control alt (hex_, 1 byte)
    pub const CHIE: &[u8; 4] = b"CHIE"; // Power adapter control (hex_, 1 byte)
    pub const CH0J: &[u8; 4] = b"CH0J"; // Power adapter control alt (ui8, 1 byte)
    pub const ACLC: &[u8; 4] = b"ACLC"; // MagSafe LED (ui8, 1 byte)
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedKeys {
    pub charge_key: &'static str,
    pub adapter_key: &'static str,
}

pub fn probe_supported(handle: &SmcHandle) -> io::Result<SupportedKeys> {
    let charge_key = if handle.get_key_info(keys::CHTE).is_ok() {
        "CHTE"
    } else if handle.get_key_info(keys::CH0C).is_ok() {
        "CH0C"
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No supported charging key found",
        ));
    };

    let adapter_key = if handle.get_key_info(keys::CHIE).is_ok() {
        "CHIE"
    } else if handle.get_key_info(keys::CH0J).is_ok() {
        "CH0J"
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No supported adapter key found",
        ));
    };

    Ok(SupportedKeys {
        charge_key,
        adapter_key,
    })
}

/// Try to disable charging using CHTE first, falling back to CH0C.
pub fn disable_charging(handle: &SmcHandle) -> io::Result<()> {
    if handle
        .write_key(keys::CHTE, &[0x01, 0x00, 0x00, 0x00])
        .is_ok()
    {
        return Ok(());
    }
    handle.write_key(keys::CH0C, &[0x01])
}

/// Try to enable charging using CHTE first, falling back to CH0C.
pub fn enable_charging(handle: &SmcHandle) -> io::Result<()> {
    if handle
        .write_key(keys::CHTE, &[0x00, 0x00, 0x00, 0x00])
        .is_ok()
    {
        return Ok(());
    }
    handle.write_key(keys::CH0C, &[0x00])
}

/// Try to disable the power adapter using CHIE first, falling back to CH0J.
pub fn disable_adapter(handle: &SmcHandle) -> io::Result<()> {
    // CHIE: disable = 0x08
    if handle.write_key(keys::CHIE, &[0x08]).is_ok() {
        return Ok(());
    }
    // CH0J: disable = 0x20
    handle.write_key(keys::CH0J, &[0x20])
}

/// Try to enable the power adapter using CHIE first, falling back to CH0J.
pub fn enable_adapter(handle: &SmcHandle) -> io::Result<()> {
    // CHIE: enable = 0x00
    if handle.write_key(keys::CHIE, &[0x00]).is_ok() {
        return Ok(());
    }
    // CH0J: enable = 0x00
    handle.write_key(keys::CH0J, &[0x00])
}

/// Set MagSafe LED state.
/// 0x00 = system controlled, 0x01 = off, 0x03 = green, 0x04 = orange.
pub fn set_magsafe_led(handle: &SmcHandle, value: u8) -> io::Result<()> {
    handle.write_key(keys::ACLC, &[value])
}
