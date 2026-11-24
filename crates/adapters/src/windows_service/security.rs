//! Account lookup of `--operator` names and the service operator rights
//! (cli-v1 §1.9, Q245). windows-service has neither, so these Win32 calls
//! are the only `unsafe` of the service code (ADR-0010 §4).

use std::ffi::c_void;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{
    self, ERROR_INSUFFICIENT_BUFFER, ERROR_NONE_MAPPED, ERROR_SUCCESS, WIN32_ERROR,
};
use windows_sys::Win32::Security::Authorization::{
    self, EXPLICIT_ACCESS_W, GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, SE_SERVICE, TRUSTEE_IS_ALIAS,
    TRUSTEE_IS_GROUP, TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_TYPE,
    TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    self, ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE, PSECURITY_DESCRIPTOR, SID_NAME_USE,
    SidTypeAlias, SidTypeGroup, SidTypeUser, SidTypeWellKnownGroup,
};
use windows_sys::Win32::System::Services::{
    SC_HANDLE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_START, SERVICE_STOP,
};

/// Start, stop, status query, and config query: the rights of a service
/// operator (cli-v1 §1.9 "접근 권한").
const OPERATOR_RIGHTS: u32 =
    SERVICE_START | SERVICE_STOP | SERVICE_QUERY_STATUS | SERVICE_QUERY_CONFIG;

/// A Windows user or group found for one `--operator` name.
pub struct Operator {
    /// The SID. `u32` words keep the alignment that Windows expects of a SID.
    sid: Vec<u32>,
    kind: TRUSTEE_TYPE,
}

/// Looks up `name` as a Windows account. `Ok(None)` when Windows maps no
/// account to the name, or the account is not a user or a group.
pub fn lookup(name: &str) -> io::Result<Option<Operator>> {
    if name.contains('\0') {
        return Ok(None);
    }
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sid_bytes = 0_u32;
    let mut domain_len = 0_u32;
    let mut use_kind: SID_NAME_USE = 0;
    // SAFETY: `wide` is NUL-terminated and lives for the call. Null SID and
    // domain buffers with zero sizes ask only for the sizes, which go to the
    // writable locals.
    let found = unsafe {
        Security::LookupAccountNameW(
            ptr::null(),
            wide.as_ptr(),
            ptr::null_mut(),
            &mut sid_bytes,
            ptr::null_mut(),
            &mut domain_len,
            &mut use_kind,
        )
    };
    if found == 0 {
        let error = io::Error::last_os_error();
        match last_code(&error) {
            Some(ERROR_INSUFFICIENT_BUFFER) => {}
            Some(ERROR_NONE_MAPPED) => return Ok(None),
            _ => return Err(error),
        }
    }
    let mut sid = vec![0_u32; (sid_bytes as usize).div_ceil(4)];
    let mut domain = vec![0_u16; domain_len as usize];
    // SAFETY: `sid` holds at least `sid_bytes` bytes and `domain` holds
    // `domain_len` UTF-16 units, the sizes from the call above. Both live for
    // the call, as does the NUL-terminated `wide`.
    let found = unsafe {
        Security::LookupAccountNameW(
            ptr::null(),
            wide.as_ptr(),
            sid.as_mut_ptr().cast(),
            &mut sid_bytes,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut use_kind,
        )
    };
    if found == 0 {
        let error = io::Error::last_os_error();
        return match last_code(&error) {
            Some(ERROR_NONE_MAPPED) => Ok(None),
            _ => Err(error),
        };
    }
    let kind = match use_kind {
        kind if kind == SidTypeUser => TRUSTEE_IS_USER,
        kind if kind == SidTypeGroup => TRUSTEE_IS_GROUP,
        kind if kind == SidTypeAlias => TRUSTEE_IS_ALIAS,
        kind if kind == SidTypeWellKnownGroup => TRUSTEE_IS_WELL_KNOWN_GROUP,
        // A domain, a computer, or a deleted account cannot start a service.
        _ => return Ok(None),
    };
    Ok(Some(Operator { sid, kind }))
}

/// Adds an allow entry with the operator rights for each operator to the
/// DACL of `service`. The other entries stay as they are.
///
/// `service` must be open with `READ_CONTROL` and `WRITE_DAC`.
pub fn grant(service: SC_HANDLE, operators: &[Operator]) -> io::Result<()> {
    if operators.is_empty() {
        return Ok(());
    }
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: the caller keeps `service` open with READ_CONTROL for the call.
    // Null owner, group, and SACL pointers ask for the DACL only. Windows
    // writes the DACL and descriptor pointers into the writable locals.
    let status = unsafe {
        Authorization::GetSecurityInfo(
            service,
            SE_SERVICE,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    check(status)?;
    // `dacl` points into the descriptor, so the descriptor outlives its use.
    let _descriptor = LocalMemory(descriptor);
    let entries: Vec<EXPLICIT_ACCESS_W> = operators
        .iter()
        .map(|operator| EXPLICIT_ACCESS_W {
            grfAccessPermissions: OPERATOR_RIGHTS,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: operator.kind,
                // With TRUSTEE_IS_SID the name field carries the SID pointer.
                ptstrName: operator.sid.as_ptr().cast_mut().cast(),
            },
        })
        .collect();
    let count = u32::try_from(entries.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many operators"))?;
    let mut new_dacl: *mut ACL = ptr::null_mut();
    // SAFETY: `entries` holds `count` initialized entries whose SID pointers
    // point into `operators`, which outlives the call. `dacl` is the DACL from
    // the descriptor kept above. Windows writes the new ACL pointer into the
    // writable local.
    let status =
        unsafe { Authorization::SetEntriesInAclW(count, entries.as_ptr(), dacl, &mut new_dacl) };
    check(status)?;
    let _new_dacl = LocalMemory(new_dacl.cast());
    // SAFETY: the caller keeps `service` open with WRITE_DAC for the call.
    // `new_dacl` is the valid ACL from SetEntriesInAclW, kept until the end of
    // this function. Null owner, group, and SACL leave those parts unchanged.
    let status = unsafe {
        Authorization::SetSecurityInfo(
            service,
            SE_SERVICE,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            new_dacl,
            ptr::null(),
        )
    };
    check(status)
}

fn last_code(error: &io::Error) -> Option<WIN32_ERROR> {
    error
        .raw_os_error()
        .and_then(|code| WIN32_ERROR::try_from(code).ok())
}

fn check(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

/// Memory that a Win32 call allocated with `LocalAlloc`, freed on drop.
struct LocalMemory(*mut c_void);

impl Drop for LocalMemory {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the pointer came from a Win32 call that allocates with
            // LocalAlloc, and it is freed only here.
            unsafe { Foundation::LocalFree(self.0) };
        }
    }
}
