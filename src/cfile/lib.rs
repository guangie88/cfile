extern crate file;

use std::ffi::CStr;
use std::os::raw::{c_char, c_longlong};
use std::ptr;

enum Error {
    Ffi(std::str::Utf8Error),
    IO(std::io::Error),
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::Ffi(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

type Result<T> = std::result::Result<T, Error>;
const CFILE_READ_ERROR: c_longlong = -1;

#[no_mangle]
pub fn cfile_read(file_path: *const c_char, file_len: *mut c_longlong) -> *mut c_char  {
    let fn_impl = || -> Result<Vec<u8>> {
        let file_path = unsafe { CStr::from_ptr(file_path) }.to_str()?;
        let buf = file::get(file_path)?;
        Ok(buf)
    };
    
    match fn_impl() {
        Ok(buf) => {
            let buf_len = buf.len();
            let buf_raw = Box::into_raw(buf.into_boxed_slice());

            unsafe { *file_len = buf_len as c_longlong };
            buf_raw as *mut c_char
        },

        Err(_) => {
            unsafe { *file_len = CFILE_READ_ERROR };
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub fn cfile_close(read_buf: *mut c_char) {
    let read_buf = read_buf as *mut u8;
    unsafe { Box::from_raw(read_buf) };
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn test_has_file() {
        const FILE_PATH: &'static str = "test_has_file.txt";
        let content = b"How are you?";
        file::put(FILE_PATH, &content).unwrap();

        let file_path_with_nul = format!("{}\0", FILE_PATH);
        let file_path_cstr = CStr::from_bytes_with_nul(file_path_with_nul.as_bytes()).unwrap();

        let mut file_len: c_longlong = 0;
        let file_buf = cfile_read(file_path_cstr.as_ptr(), &mut file_len);

        // check status code and file length
        assert!(file_len != CFILE_READ_ERROR);
        assert!(file_len as usize == content.len());

        // check content
        for i in 0..content.len() {
            assert!(unsafe { *file_buf.offset(i as isize) as u8 == content[i] });
        }

        cfile_close(file_buf);
        fs::remove_file(FILE_PATH).unwrap();
    }

    #[test]
    fn test_no_file() {
        const FILE_PATH: &'static str = "test_no_file.txt";

        // force remove file, ignore error
        let _ = fs::remove_file(FILE_PATH);

        let file_path_with_nul = format!("{}\0", FILE_PATH);
        let file_path_cstr = CStr::from_bytes_with_nul(file_path_with_nul.as_bytes()).unwrap();

        let mut file_len: c_longlong = 0;
        let _ = cfile_read(file_path_cstr.as_ptr(), &mut file_len);

        // check status code must be an error
        // error means no need to close file buffer
        assert!(file_len == CFILE_READ_ERROR);
    }
}