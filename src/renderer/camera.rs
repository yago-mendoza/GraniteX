use glam::{Mat4, Vec3};

pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
    // Animation state
    target_yaw: Option<f32>,
    target_pitch: Option<f32>,
    start_yaw: f32,
    start_pitch: f32,
    anim_progress: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
            z_near: 0.05,
            z_far: 500.0,
            target_yaw: None,
            target_pitch: None,
            start_yaw: std::f32::consts::FRAC_PI_4,
            start_pitch: std::f32::consts::FRAC_PI_6,
            anim_progress: 0.0,
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

    /// Pan: move target perpendicular to view direction.
    /// Safe at all pitch angles (no NaN when looking straight up/down).
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let eye = self.eye();
        let forward = (self.target - eye).normalize();

        // Use world up unless looking nearly straight up/down
        let world_up = if forward.dot(Vec3::Y).abs() > 0.99 {
            Vec3::Z // fallback when looking up/down
        } else {
            Vec3::Y
        };

        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward).normalize();

        let speed = self.distance * 0.002;
        self.target += right * (-delta_x * speed) + up * (delta_y * speed);
    }

    /// Zoom: move closer/further from target
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * 0.1;
        self.distance = self.distance.clamp(0.1, 500.0);
    }

    /// Start animating to a target view (smooth transition).
    pub fn set_view(&mut self, yaw: f32, pitch: f32) {
        self.start_yaw = self.yaw;
        self.start_pitch = self.pitch;
        self.target_yaw = Some(yaw);
        self.target_pitch = Some(pitch);
        self.anim_progress = 0.0;
    }

    /// Snap immediately (no animation).
    #[allow(dead_code)]
    pub fn set_view_instant(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
        self.target_yaw = None;
        self.target_pitch = None;
    }

    /// Advance camera animation. Call once per frame.
    pub fn update_animation(&mut self, dt: f32) {
        if let (Some(ty), Some(tp)) = (self.target_yaw, self.target_pitch) {
            self.anim_progress += dt / 0.25; // 0.25s animation duration
            let t = self.anim_progress.clamp(0.0, 1.0);
            // Ease-out cubic
            let t = 1.0 - (1.0 - t).powi(3);

            self.yaw = lerp(self.start_yaw, ty, t);
            self.pitch = lerp(self.start_pitch, tp, t);

            if self.anim_progress >= 1.0 {
                self.yaw = ty;
                self.pitch = tp;
                self.target_yaw = None;
                self.target_pitch = None;
            }
        }
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
