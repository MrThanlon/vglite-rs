// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(non_upper_case_globals)]
mod vg_lite;
mod path;
mod transform;

use vg_lite::*;
pub use path::*;
pub use transform::*;
use std::{ffi::c_void, ptr::null_mut};

pub struct Context(());
impl Context {
    /// Can be called before [`Context::new`] to overwrite the default value: 65536
    pub fn set_command_size(size: u32) -> Result<(), Error> {
        wrap_result(unsafe {
            vg_lite_set_command_buffer_size(size)
        }, ())
    }
    pub fn new(tess_width: u32, tess_height: u32) -> Result<Self, Error> {
        wrap_result(unsafe {
            vg_lite_init(tess_width as i32, tess_height as i32)
        }, Context(()))
    }
    /// Do drawing with blocking
    pub fn finish(&self) -> Result<(), Error> {
        wrap_result(unsafe { vg_lite_finish() }, ())
    }
    /// Do drawing without blocking
    pub fn flush(&self) -> Result<(), Error> {
        wrap_result(unsafe { vg_lite_flush() }, ())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { vg_lite_close(); }
    }
}

enum BufferSource {
    #[allow(dead_code)]
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

impl Format {
    fn bpp(&self) -> u32 {
        match self {
            Self::BGR565 | Self::RGB565 => 2,
            Self::BGRA8888 | Self::RGBA8888 => 4
        }
    }
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

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b:u8,
    pub a: u8
}

impl Into<u32> for Color {
    fn into(self) -> u32 {
        ((self.a as u32) << 24) |
        ((self.r as u32) << 16) |
        ((self.g as u32) << 8) |
        (self.b as u32)
    }
}

pub struct Buffer {
    buffer: vg_lite_buffer,
    source: BufferSource,
}

impl Default for vg_lite_buffer {
    fn default() -> Self {
        Self {
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

impl vg_lite_buffer {
    fn new(width: i32, height: i32, format: vg_lite_format) -> Self {
        let mut buffer = Self::default();
        buffer.width = width;
        buffer.height = height;
        buffer.format = format;
        buffer
    }
}

#[derive(Debug, Clone, Copy)]
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

fn wrap_result<T>(error: vg_lite_error, t: T) -> Result<T, Error> {
    if error == vg_lite_error_VG_LITE_SUCCESS {
        Ok(t)
    } else {
        // eprintln!("vg_lite error: {}", error);
        Err(error.into())
    }
}

pub type Rectangle = vg_lite_rectangle;

impl Buffer {
    pub fn allocate(width: u32, height: u32, format: Format) -> Result<Self, Error> {
        let mut buffer = Buffer {
            buffer: vg_lite_buffer::new(width as i32, height as i32, format.into()),
            source: BufferSource::Allocated
        };
        let error = unsafe {
            vg_lite_allocate(&mut buffer.buffer)
        };
        dbg!(buffer.buffer.memory);
        wrap_result(error, buffer)
    }

    pub fn map(width: u32, height: u32, format: Format, dmabuf_fd: i32, memory: *mut c_void) -> Result<Self, Error> {
        let mut buffer = Buffer {
            buffer: vg_lite_buffer::new(width as i32, height as i32, format.into()),
            source: BufferSource::Mapped
        };
        buffer.buffer.address = 0xdeaddead;
        buffer.buffer.stride = (width * format.bpp()) as i32;
        buffer.buffer.memory = memory;
        wrap_result(unsafe {
            vg_lite_map(&mut buffer.buffer, vg_lite_map_flag_VG_LITE_MAP_DMABUF, dmabuf_fd)
        }, buffer)
    }

    pub fn data(&self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.buffer.memory as *mut u8,
                (self.buffer.stride * self.buffer.height) as usize
            )
        }
    }

    pub fn clear(&mut self, rectangle: Option<&mut Rectangle>, color: Color) -> Result<(), Error> {
        wrap_result(unsafe {
            vg_lite_clear(
                &mut self.buffer,
                match rectangle {
                    Some(rect) => rect,
                    None => null_mut()
                },
                color.into()
            )
        }, ())
    }

    pub fn blit(
        &mut self,
        source: &mut Buffer,
        matrix: &mut Transform,
        blend: Blend,
        color: Color,
        filter: Filter
    ) -> Result<(), Error> {
        wrap_result(unsafe {
            vg_lite_blit(
                &mut self.buffer,
                &mut source.buffer,
                matrix,
                blend.into(),
                color.into(),
                filter.into()
            )
        }, ())
    }

    pub fn draw<T: OpCodeFormat>(
        &mut self,
        path: &mut Path<T>,
        fill_rule: Fill,
        transform: &mut Transform,
        blend: Blend,
        color: Color
    ) -> Result<(), Error> {
        wrap_result(unsafe {
            vg_lite_draw(
                &mut self.buffer,
                &mut path.path,
                fill_rule.into(),
                transform,
                blend.into(),
                color.into()
            )
        }, ())
    }

    pub fn draw_pattern<T: OpCodeFormat>(
        &mut self,
        path: &mut Path<T>,
        fill_rule: Fill,
        path_transform: &mut Transform,
        pattern: &mut Buffer,
        pattern_matrix: &mut Transform,
        blend: Blend,
        pattern_mode: PatternMode,
        color: Color,
        filter: Filter
    ) -> Result<(), Error> {
        wrap_result(unsafe {
            vg_lite_draw_pattern(
                &mut self.buffer,
                &mut path.path,
                fill_rule.into(),
                path_transform,
                &mut pattern.buffer,
                pattern_matrix,
                blend.into(),
                pattern_mode.into(),
                color.into(),
                filter.into()
            )
        }, ())
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

#[derive(Clone, Copy)]
pub enum Filter {
    Pointer, Linear, Bilinear
}

impl Into<vg_lite_filter> for Filter {
    fn into(self) -> vg_lite_filter {
        match self {
            Self::Pointer => vg_lite_filter_VG_LITE_FILTER_POINT,
            Self::Linear => vg_lite_filter_VG_LITE_FILTER_LINEAR,
            Self::Bilinear => vg_lite_filter_VG_LITE_FILTER_BI_LINEAR
        }
    }
}

#[derive(Clone, Copy)]
pub enum Blend {
    None = vg_lite_blend_VG_LITE_BLEND_NONE as isize,
    SourceOver = vg_lite_blend_VG_LITE_BLEND_SRC_OVER as isize,
}

impl Into<vg_lite_blend> for Blend {
    fn into(self) -> vg_lite_blend {
        self as vg_lite_blend
    }
}

#[derive(Clone, Copy)]
pub enum PatternMode {
    Color = vg_lite_pattern_mode_VG_LITE_PATTERN_COLOR as isize,
    Pad = vg_lite_pattern_mode_VG_LITE_PATTERN_PAD as isize,
    Repeat = vg_lite_pattern_mode_VG_LITE_PATTERN_REPEAT as isize,
    Reflect = vg_lite_pattern_mode_VG_LITE_PATTERN_REFLECT as isize,
}

impl Into<vg_lite_pattern_mode> for PatternMode {
    fn into(self) -> vg_lite_pattern_mode {
        self as vg_lite_pattern_mode
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn init_and_deinit() {
        let ctx = Context::new(640, 480).unwrap();
        let mut buffer = Buffer::allocate(640, 480, Format::BGRA8888).unwrap();
        buffer.clear(None, Color { r: 0, g: 0, b: 0, a: 0 }).unwrap();
        ctx.finish().unwrap();
    }
}
