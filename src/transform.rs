use crate::vg_lite::*;

pub type Transform = vg_lite_matrix;

impl Transform {
    pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
        unsafe { vg_lite_translate(x, y, self) };
        self
    }

    pub fn scale(&mut self, x: f32, y: f32) -> &mut Self {
        unsafe { vg_lite_scale(x, y, self) };
        self
    }

    pub fn rotate(&mut self, degrees: f32) -> &mut Self {
        unsafe { vg_lite_rotate(degrees, self) };
        self
    }
}

impl Default for Transform {
    /// Identity matrix
    fn default() -> Self {
        vg_lite_matrix { m: [
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.]
        ]}
    }
}
