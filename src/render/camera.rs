
use nalgebra_glm as glm;
use crate::util::bytes;


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GimbalCamera {
    view:   glm::Mat4,
    pos:    glm::Vec3,
    center: glm::Vec3,
    dir:    glm::Vec3,
    top:    glm::Vec3,
}

unsafe impl bytes::IntoBytes for GimbalCamera {}

impl GimbalCamera {
    pub fn new(pos: glm::Vec3, center: glm::Vec3, top: glm::Vec3) -> Self {
        let dir = (center - pos).normalize();
        GimbalCamera {
            view: glm::look_at_lh(&pos, &center, &top),
            pos, center, dir, top,
        }
    }

    #[inline]
    fn refresh_view_matrix(&mut self) {
        self.view = glm::look_at_lh(&self.pos, &(self.pos + self.dir), &self.top);
    }

    #[inline]
    pub fn translate(&mut self, dpos: glm::Vec3) {
        self.pos += dpos;
        self.refresh_view_matrix();
    }

    #[inline]
    pub fn translate_rel(&mut self, drel: glm::Vec3) {
        let dxt = self.dir.cross(&self.top);
        self.translate(
            drel.x * dxt +
            drel.y * self.top +
            drel.z * self.dir
        );
    }

    #[inline]
    pub fn zoom(&mut self, ratio: f32) {
        self.pos = glm::lerp(&self.pos, &self.center, ratio);
        self.refresh_view_matrix();
    }

    #[inline]
    pub fn gimbal_ud(&mut self, degrees: f32) {
        let dxt = self.dir.cross(&self.top);
        let rot = glm::rotation(degrees, &dxt.normalize());
        self.top = rot.transform_vector(&(self.top - self.center)) + self.center;
        self.pos = rot.transform_vector(&(self.pos - self.center)) + self.center;
        self.dir = rot.transform_vector(&(self.dir - self.center)) + self.center;
        self.refresh_view_matrix();
    }

    #[inline]
    pub fn gimbal_lr(&mut self, degrees: f32) {
        let rot = glm::rotation(degrees, &self.top);
        self.top = rot.transform_vector(&(self.top - self.center)) + self.center;
        self.pos = rot.transform_vector(&(self.pos - self.center)) + self.center;
        self.dir = rot.transform_vector(&(self.dir - self.center)) + self.center;
        self.refresh_view_matrix();
    }
}
