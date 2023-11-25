// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(non_upper_case_globals)]
mod vg_lite;

use vg_lite::*;
use std::ptr::null_mut;

pub struct Context(());
impl Context {
    /// Can be called before [`Context::new`] to overwrite the default value: 65536
    pub fn set_command_size(size: u32) -> Result<(), Error> {
        wrap_result((), unsafe {
            vg_lite_set_command_buffer_size(size)
        })
    }
    pub fn new(tess_width: u32, tess_height: u32) -> Result<Self, Error> {
        wrap_result(Context(()), unsafe {
            vg_lite_init(tess_width as i32, tess_height as i32)
        })
    }
    /// Do drawing with blocking
    pub fn finish(self) -> Result<(), Error> {
        wrap_result((), unsafe { vg_lite_finish() })
    }
    /// Do drawing without blocking
    pub fn flush(self) -> Result<(), Error> {
        wrap_result((), unsafe { vg_lite_flush() })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { vg_lite_close(); }
    }
}

enum BufferSource {
    None,
    Allocated,
    Mapped
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    RGBA8888,
    BGRA8888,
    RGB565,
    BGR565
}

impl From<Format> for vg_lite_format_t {
    fn from(format: Format) -> Self {
        match format {
            Format::RGBA8888 => vg_lite_buffer_format_VG_LITE_RGBA8888,
            Format::BGRA8888 => vg_lite_buffer_format_VG_LITE_BGRA8888,
            Format::RGB565 => vg_lite_buffer_format_VG_LITE_RGB565,
            Format::BGR565 => vg_lite_buffer_format_VG_LITE_BGR565
        }
    }
}

pub struct Buffer {
    buffer: vg_lite_buffer,
    source: BufferSource,
}

impl vg_lite_buffer {
    fn new(width: i32, height: i32, format: vg_lite_format) -> Self {
        let mut buffer = Self::default();
        buffer.width = width;
        buffer.height = height;
        buffer.format = format;
        buffer
    }
    fn default() -> Self {
        vg_lite_buffer {
            width: 0, height: 0, stride: 0,
            tiled: 0, format: 0, handle: null_mut(),
            memory: null_mut(), address: 0,
            yuv: vg_lite_yuvinfo {
                swizzle: 0, yuv2rgb: 0,
                uv_planar: 0, v_planar: 0,alpha_planar: 0,
                uv_stride: 0, v_stride: 0, alpha_stride: 0,
                uv_height: 0, v_height: 0,
                uv_memory: null_mut(), v_memory: null_mut(),
                uv_handle: null_mut(), v_handle: null_mut()
            },
            image_mode: 0, transparency_mode: 0,
            fc_enable: 0, fc_buffer: [
                vg_lite_fc_buffer {
                    width: 0, height: 0, stride: 0, handle: null_mut(), memory: null_mut(), address: 0, color: 0
                },
                vg_lite_fc_buffer {
                    width: 0, height: 0, stride: 0, handle: null_mut(), memory: null_mut(), address: 0, color: 0
                },
                vg_lite_fc_buffer {
                    width: 0, height: 0, stride: 0, handle: null_mut(), memory: null_mut(), address: 0, color: 0
                },
            ],
            compress_mode: 0, index_endian: 0
        }
    }
}

pub enum Error {
    InvalidArgument,
    OutOfMemory,
    NoContext,
    Timeout,
    OutOfResource,
    GenericIO,
    NotSupport,
    AlreadyExists,
    NotAligned,
    FlexaTimeOut,
    FlexaHandshakeFail,
    Unknown
}

impl From<vg_lite_error> for Error {
    fn from(error: vg_lite_error) -> Self {
        match error {
            vg_lite_error_VG_LITE_INVALID_ARGUMENT => Self::InvalidArgument,
            vg_lite_error_VG_LITE_OUT_OF_MEMORY => Self::OutOfMemory,
            vg_lite_error_VG_LITE_NO_CONTEXT => Self::NoContext,
            vg_lite_error_VG_LITE_TIMEOUT => Self::Timeout,
            vg_lite_error_VG_LITE_OUT_OF_RESOURCES => Self::OutOfResource,
            vg_lite_error_VG_LITE_GENERIC_IO => Self::GenericIO,
            vg_lite_error_VG_LITE_NOT_SUPPORT => Self::NotSupport,
            vg_lite_error_VG_LITE_ALREADY_EXISTS => Self::AlreadyExists,
            vg_lite_error_VG_LITE_NOT_ALIGNED => Self::NotAligned,
            vg_lite_error_VG_LITE_FLEXA_HANDSHAKE_FAIL => Self::FlexaHandshakeFail,
            vg_lite_error_VG_LITE_FLEXA_TIME_OUT => Self::FlexaTimeOut,
            _ => Self::Unknown
        }
    }
}

fn wrap_result<T>(t: T, error: vg_lite_error) -> Result<T, Error> {
    if error == vg_lite_error_VG_LITE_SUCCESS {
        Ok(t)
    } else {
        Err(error.into())
    }
}

impl Buffer {
    pub fn allocate(width: u32, height: u32, format: Format) -> Result<Self, Error> {
        let mut buffer = Buffer {
            buffer: vg_lite_buffer::new(width as i32, height as i32, format.into()),
            source: BufferSource::None
        };
        let error = unsafe {
            vg_lite_allocate(&mut buffer.buffer)
        };
        if error != 0 {
            Err(error.into())
        } else {
            buffer.source = BufferSource::Allocated;
            Ok(buffer)
        }
    }

    pub fn map(width: u32, height: u32, format: Format, fd: i32) -> Result<Self, Error> {
        let mut buffer = Buffer {
            buffer: vg_lite_buffer::new(width as i32, height as i32, format.into()),
            source: BufferSource::None
        };
        let error = unsafe {
            vg_lite_map(&mut buffer.buffer, vg_lite_map_flag_VG_LITE_MAP_DMABUF, fd)
        };
        if error == vg_lite_error_VG_LITE_SUCCESS {
            buffer.source = BufferSource::Mapped;
            Ok(buffer)
        } else {
            Err(error.into())
        }
    }

    pub fn draw(&mut self) -> Result<(), Error> {
        let error = unsafe {
        };
        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        match self.source {
            BufferSource::Allocated => unsafe { vg_lite_free(&mut self.buffer); }
            BufferSource::Mapped => unsafe { vg_lite_unmap(&mut self.buffer); }
            _ => ()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let _buffer = Buffer::allocate(0, 0, Format::BGR565);
        assert_eq!(4, 4);
    }
}
