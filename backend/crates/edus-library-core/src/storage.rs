use crate::domain::AppError;
use std::path::Path;

pub fn available_bytes(path: &Path) -> Result<u64, AppError> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            available: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
    }
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut available = 0;
    if unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(AppError::new(
            "STORAGE_STATUS_UNAVAILABLE",
            "Не удалось проверить свободное место на диске.",
        ));
    }
    Ok(available)
}
pub fn require_write_space(bytes: u64) -> Result<(), AppError> {
    if bytes < 16 * 1024 * 1024 {
        Err(AppError::new("DISK_FULL","Недостаточно места на диске. Операция не записана; освободите место через администратора."))
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_disk_is_explicit_and_never_deletes_data() {
        assert!(require_write_space(0).is_err());
        assert!(require_write_space(15 * 1024 * 1024).is_err());
        assert!(require_write_space(32 * 1024 * 1024).is_ok());
    }
}
