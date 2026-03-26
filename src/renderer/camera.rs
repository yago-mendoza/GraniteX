use glam::{Mat4, Vec3};

pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,   // horizontal angle (radians)
    pub pitch: f32,  // vertical angle (radians)
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 3.5,
            yaw: std::f32::consts::FRAC_PI_4,       // 45 degrees
            pitch: std::f32::consts::FRAC_PI_6,      // 30 degrees
            aspect,
            fov_y: 45.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 1000.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Orbit: rotate around the target
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.005;
        self.pitch += delta_y * 0.005;
        // Clamp pitch to avoid flipping
        let limit = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    /// Pan: move target perpendicular to view direction
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let eye = self.eye();
        let forward = (self.target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        let speed = self.distance * 0.002;
        self.target += right * (-delta_x * speed) + up * (delta_y * speed);
    }

    /// Zoom: move closer/further from target
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * 0.1;
        self.distance = self.distance.clamp(0.1, 500.0);
    }

    /// Snap to a standard view
    pub fn set_view(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
    }

    /// Frame the camera to fit a bounding box.
    pub fn fit_to_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let extent = (max - min).length();
        self.target = center;
        // Distance so the object fills ~60% of the viewport
        self.distance = (extent / (2.0 * (self.fov_y / 2.0).tan())).max(0.5);
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn uniform(&self) -> CameraUniform {
        let view_proj = self.projection_matrix() * self.view_matrix();
        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
        }
    }
}
