use std::mem::{transmute, size_of};
use std::os::raw::c_void;

use crate::vg_lite::*;
use crate::*;

pub trait OpCodeFormat: Sized + Clone + Copy {
    fn format() -> DataFormat;
    fn transmute(op: u32) -> Self;
}

impl OpCodeFormat for i8 {
    fn format() -> DataFormat { DataFormat::I8 }
    fn transmute(op: u32) -> Self { op as i8 }
}
impl OpCodeFormat for i16 {
    fn format() -> DataFormat { DataFormat::I16 }
    fn transmute(op: u32) -> Self { op as i16 }
}
impl OpCodeFormat for i32 {
    fn format() -> DataFormat { DataFormat::I32 }
    fn transmute(op: u32) -> Self { op as i32 }
}
impl OpCodeFormat for f32 {
    fn format() -> DataFormat { DataFormat::F32 }
    fn transmute(op: u32) -> Self {
        unsafe { transmute(op as u32) }
    }
}
#[derive(Debug, Clone)]
pub enum Opcode<T: OpCodeFormat> {
    End,
    Close,
    Move {
        x: T, y: T
    },
    MoveRel {
        dx: T, dy: T
    },
    Line {
        x: T, y: T
    },
    LineRel {
        dx: T, dy: T
    },
    Quad {
        cx: T, cy: T, x: T, y: T
    },
    QuadRel {
        dcx: T, dcy: T, dx: T, dy: T
    },
    Cubic {
        cx1: T, cy1: T, cx2: T, cy2: T, x: T, y: T
    },
    CubicRel {
        dcx1: T, dcy1: T, dcx2: T, dcy2: T, dx: T, dy: T
    },
    SCCWArc {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    SCCWArcRel {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    SCWArc {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    SCWArcRel {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    LCCWArc {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    LCCWArcRel {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    LCWArc {
        rh: T, rv: T, rot: T, x: T, y: T
    },
    LCWArcRel {
        rh: T, rv: T, rot: T, x: T, y: T
    },
}

#[derive(Debug, Copy, Clone)]
pub enum Quality {
    High, Upper, Medium, Low
}

impl Into<vg_lite_quality> for Quality {
    fn into(self) -> vg_lite_quality {
        match self {
            Self::High => vg_lite_quality_VG_LITE_HIGH,
            Self::Upper => vg_lite_quality_VG_LITE_UPPER,
            Self::Medium => vg_lite_quality_VG_LITE_MEDIUM,
            Self::Low => vg_lite_quality_VG_LITE_LOW
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DataFormat {
    I8, I16, I32, F32
}

impl Into<vg_lite_format> for DataFormat {
    fn into(self) -> vg_lite_format {
        match self {
            Self::I8 => vg_lite_format_VG_LITE_S8,
            Self::I16 => vg_lite_format_VG_LITE_S16,
            Self::I32 => vg_lite_format_VG_LITE_S32,
            Self::F32 => vg_lite_format_VG_LITE_FP32,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Fill {
    NonZero, EvenOdd
}

impl Into<vg_lite_fill> for Fill {
    fn into(self) -> vg_lite_fill {
        match self {
            Fill::NonZero => vg_lite_fill_VG_LITE_FILL_NON_ZERO,
            Fill::EvenOdd => vg_lite_fill_VG_LITE_FILL_EVEN_ODD
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathData<T: OpCodeFormat>(Vec<T>);

impl<T: OpCodeFormat> PathData<T> {
    pub fn append(&mut self, op: Opcode<T>) -> &mut Self {
        let data = &mut self.0;
        match op {
            Opcode::End => { data.push(T::transmute(VLC_OP_END)) }
            Opcode::Close => { data.push(T::transmute(VLC_OP_CLOSE)); }
            Opcode::Move { x, y } => {
                data.push(T::transmute(VLC_OP_MOVE));
                data.push(x);
                data.push(y);
            }
            Opcode::Line { x, y } => {
                data.push(T::transmute(VLC_OP_LINE));
                data.push(x);
                data.push(y);
            }
            Opcode::Cubic { cx1, cy1, cx2, cy2, x, y } => {
                data.push(T::transmute(VLC_OP_CUBIC));
                data.push(cx1);
                data.push(cy1);
                data.push(cx2);
                data.push(cy2);
                data.push(x);
                data.push(y);
            }
            _ => ()
        };
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.append(Opcode::Close)
    }

    pub fn move_to(&mut self, x: T, y: T) -> &mut Self {
        self.append(Opcode::Move { x, y })
    }

    pub fn line_to(&mut self, x: T, y: T) -> &mut Self {
        self.append(Opcode::Line { x, y })
    }

    pub fn curve_to(&mut self, x1: T, y1: T, x2: T, y2: T, x: T, y: T) -> &mut Self {
        self.append(Opcode::Cubic { cx1: x1, cy1: y1, cx2: x2, cy2: y2, x, y })
    }

    pub fn fill(self, quality: Quality) -> Path<T> {
        let mut path = Path::new(self, quality);
        path.path.path_type |= 0b10;
        path
    }

    // TODO: stroke
}

#[derive(Debug, Clone)]
pub struct Path<T: OpCodeFormat> {
    pub path: vg_lite_path,
    #[allow(unused)]
    /// To keep life cycle
    data: PathData<T>
}

impl<T: OpCodeFormat> Path<T> {
    pub fn new(data: PathData<T>, quality: Quality) -> Self {
        Self {
            path: vg_lite_path {
                bounding_box: [0.;4],
                quality: quality.into(),
                format: T::format().into(),
                uploaded: vg_lite_hw_memory {
                    handle: null_mut(),
                    memory: null_mut(),
                    address: 0,
                    bytes: 0,
                    property:0
                },
                path_length: (data.0.len() * size_of::<T>()) as u32,
                path: data.0.as_ptr() as *mut c_void,
                path_changed: 0,
                pdata_internal: 0,
                path_type: vg_lite_path_type_VG_LITE_DRAW_ZERO,
                stroke: null_mut(),
                stroke_path: null_mut(),
                stroke_size: 0,
                stroke_color: 0,
                add_end: 0
            },
            data,
        }
    }
}