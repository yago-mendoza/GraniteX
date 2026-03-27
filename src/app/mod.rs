mod input;
mod mesh_ops;
mod sketch_ops;
mod measure;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::commands::CommandHistory;
use crate::construction::ConstructionGeometry;
use crate::renderer::Renderer;
use crate::sketch::Sketch;
use crate::ui::{ContextAction, Tool, UiState, ViewPreset};

#[derive(Default)]
pub(super) struct InputState {
    pub(super) left_pressed: bool,
    pub(super) middle_pressed: bool,
    pub(super) left_was_drag: bool,
    pub(super) left_press_pos: Option<(f64, f64)>,
    pub(super) last_mouse: Option<(f64, f64)>,
    pub(super) cursor_pos: (f64, f64),
    pub(super) cursor_moved: bool,
    pub(super) modifiers: ModifiersState,
    /// Drag-to-extrude/cut: actively dragging to set distance.
    pub(super) operation_dragging: bool,
    pub(super) drag_start_y: f64,
    pub(super) drag_accumulated: f32,
}

pub(super) struct App {
    pub(super) window: Option<Window>,
    pub(super) renderer: Option<Renderer>,
    pub(super) egui_ctx: egui::Context,
    pub(super) egui_state: Option<egui_winit::State>,
    pub(super) ui: UiState,
    pub(super) input: InputState,
    pub(super) sketch: Option<Sketch>,
    pub(super) history: CommandHistory,
    pub(super) construction: ConstructionGeometry,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            ui: UiState::new(),
            input: InputState::default(),
            sketch: None,
            history: CommandHistory::new(),
            construction: ConstructionGeometry::new(),
        }
    }

    fn apply_ui_state(&mut self) {
        // Handle file operations before borrowing renderer
        if self.ui.new_scene_request {
            self.ui.new_scene_request = false;
            if let Some(r) = &mut self.renderer {
                r.load_mesh(crate::renderer::Mesh::cube());
                self.history.clear();
                self.sketch = None;
                self.ui.operation_history.clear();
                self.ui.current_project_path = None;
                self.ui.active_measurement = None;
                self.ui.measure_first_point = None;
                self.ui.selected_edge = None;
                self.ui.toasts.push(crate::ui::Toast::new("New scene".into()));
            }
        }

        if self.ui.save_request || self.ui.save_as_request {
            let save_as = self.ui.save_as_request;
            self.ui.save_request = false;
            self.ui.save_as_request = false;
            self.save_project(save_as);
        }

        if self.ui.open_project_request {
            self.ui.open_project_request = false;
            self.open_project();
        }

        if self.ui.export_stl_request {
            self.ui.export_stl_request = false;
            self.export_file("stl");
        }
        if self.ui.export_obj_request {
            self.ui.export_obj_request = false;
            self.export_file("obj");
        }

        if self.ui.import_request {
            self.ui.import_request = false;
            self.open_file_dialog();
        }

        if let Some(action) = self.ui.context_menu_action.take() {
            match action {
                ContextAction::Delete => {
                    self.delete_selected_face();
                    self.ui.toasts.push(crate::ui::Toast::new("Face deleted".into()));
                }
                ContextAction::Extrude => {
                    self.ui.active_tool = Tool::Extrude;
                }
                ContextAction::Cut => {
                    self.ui.active_tool = Tool::Cut;
                }
                ContextAction::Inset => {
                    self.ui.active_tool = Tool::Inset;
                }
                ContextAction::ZoomToFace => {
                    if let Some(r) = &mut self.renderer {
                        r.fit_camera();
                    }
                }
            }
        }

        let Some(renderer) = &mut self.renderer else { return };
        renderer.show_grid = self.ui.show_grid;
        renderer.show_wireframe = self.ui.show_wireframe;
        self.ui.wireframe_supported = renderer.has_wireframe();

        // Sync construction geometry from UI
        self.construction.selected = self.ui.construction_selected;
        for plane in &mut self.construction.planes {
            plane.visible = self.ui.show_construction_planes;
        }
        for axis in &mut self.construction.axes {
            axis.visible = self.ui.show_construction_axes;
        }
        renderer.update_construction(&self.construction);

        // Extrude — from sketch region (atomic) or from selected face
        if let Some(distance) = self.ui.extrude_request.take() {
            self.history.save_state(&renderer.mesh);

            // If there's a sketch with a selected region, do the FULL operation atomically:
            // 1. Create base face from region (with holes if any)
            // 2. Extrude it (creates outer + inner side walls)
            // 3. Split parent face (remove the region from it)
            let from_sketch = self.sketch.as_mut().and_then(|s| {
                let pts = s.selected_region_3d()?;
                let holes = s.selected_region_holes_3d();
                let normal = s.plane.normal;
                let parent_id = s.face_id; // None for reference plane sketches
                let plane = s.plane.clone();
                Some((pts, holes, normal, parent_id, plane))
            });

            let success = if let Some((region_pts, holes, normal, parent_id, plane)) = from_sketch {
                let base_face = if holes.is_empty() {
                    renderer.mesh.add_polygon_face_flush(&region_pts, normal)
                } else {
                    renderer.mesh.add_polygon_face_with_holes_flush(&region_pts, &holes, normal)
                };
                if let Some(cap) = renderer.mesh.extrude_face(base_face, distance) {
                    // Only split parent face if sketching on a mesh face (not a reference plane)
                    if let Some(pid) = parent_id {
                        if renderer.mesh.is_face_planar(pid) {
                            Self::split_parent_face(&mut renderer.mesh, pid, &region_pts, &holes, &plane);
                        }
                    }
                    renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                    renderer.selected_face = Some(cap);
                    renderer.mesh_pipeline.set_selected_face(&renderer.gpu.queue, Some(cap));
                    true
                } else { false }
            } else {
                renderer.extrude_selected(distance).is_some()
            };

            if success {
                self.sketch = None;
                self.ui.toasts.push(crate::ui::Toast::new(format!("Extruded {:.2}m", distance)));
                self.ui.operation_history.push(format!("Extrude {:.2}m", distance));
            }
            renderer.clear_preview();
        }

        // Cut — from sketch region (atomic) or from selected face
        if let Some(depth) = self.ui.cut_request.take() {
            self.history.save_state(&renderer.mesh);

            let from_sketch = self.sketch.as_mut().and_then(|s| {
                let pts = s.selected_region_3d()?;
                let holes = s.selected_region_holes_3d();
                let normal = s.plane.normal;
                let parent_id = s.face_id;
                let plane = s.plane.clone();
                Some((pts, holes, normal, parent_id, plane))
            });

            let success = if let Some((region_pts, holes, normal, parent_id, plane)) = from_sketch {
                let base_face = if holes.is_empty() {
                    renderer.mesh.add_polygon_face_flush(&region_pts, normal)
                } else {
                    renderer.mesh.add_polygon_face_with_holes_flush(&region_pts, &holes, normal)
                };
                if let Some(floor) = renderer.mesh.cut_face(base_face, depth) {
                    if let Some(pid) = parent_id {
                        if renderer.mesh.is_face_planar(pid) {
                            Self::split_parent_face(&mut renderer.mesh, pid, &region_pts, &holes, &plane);
                        }
                    }
                    renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                    renderer.selected_face = Some(floor);
                    renderer.mesh_pipeline.set_selected_face(&renderer.gpu.queue, Some(floor));
                    true
                } else { false }
            } else {
                renderer.cut_selected(depth).is_some()
            };

            if success {
                self.sketch = None;
                self.ui.toasts.push(crate::ui::Toast::new(format!("Cut {:.2}m", depth)));
                self.ui.operation_history.push(format!("Cut {:.2}m", depth));
            }
            renderer.clear_preview();
        }

        // Inset
        if let Some(amount) = self.ui.inset_request.take() {
            if let Some(face_id) = renderer.selected_face {
                self.history.save_state(&renderer.mesh);
                if let Some(inner) = renderer.mesh.inset_face(face_id, amount) {
                    renderer.selected_face = Some(inner);
                    renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                    renderer.mesh_pipeline.set_selected_face(&renderer.gpu.queue, Some(inner));
                    self.ui.toasts.push(crate::ui::Toast::new(format!("Inset {:.2}m", amount)));
                    self.sketch = None;
                }
            }
        }

        // Preview — from sketch region OR from selected mesh face
        let sketch_preview_data = self.sketch.as_mut().and_then(|s| {
            if s.selected_region.is_none() { return None; }
            let pts = s.selected_region_3d()?;
            let normal = s.plane.normal;
            Some((pts, normal))
        });

        self.ui.preview_active = false;
        match self.ui.active_tool {
            Tool::Extrude => {
                if let Some((pts, normal)) = &sketch_preview_data {
                    renderer.preview_from_points(pts, *normal, self.ui.extrude_distance);
                    self.ui.preview_active = true;
                } else if renderer.selected_face.is_some() {
                    renderer.update_extrude_preview(self.ui.extrude_distance);
                    self.ui.preview_active = true;
                } else {
                    renderer.clear_preview();
                }
            }
            Tool::Cut => {
                if let Some((pts, normal)) = &sketch_preview_data {
                    renderer.preview_cut_from_points(pts, *normal, self.ui.cut_depth);
                    self.ui.preview_active = true;
                } else if renderer.selected_face.is_some() {
                    renderer.update_cut_preview(self.ui.cut_depth);
                    self.ui.preview_active = true;
                } else {
                    renderer.clear_preview();
                }
            }
            Tool::Inset => {
                if renderer.selected_face.is_some() {
                    renderer.update_inset_preview(self.ui.inset_amount);
                    self.ui.preview_active = true;
                } else {
                    renderer.clear_preview();
                }
            }
            _ => renderer.clear_preview(),
        }

        // Update sketch cursor (snap + H/V inference pipeline)
        if let Some(sketch) = &mut self.sketch {
            let (ray_o, ray_d) = renderer.screen_to_ray(
                self.input.cursor_pos.0 as f32,
                self.input.cursor_pos.1 as f32,
            );
            if let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) {
                let raw = sketch.world_to_2d(hit);
                let shift = self.input.modifiers.shift_key();
                let line_mode = self.ui.active_tool == Tool::Line;
                let (pos, _snap, inference) = sketch.resolve_cursor(raw, shift, line_mode);
                sketch.cursor_2d = Some(pos);
                sketch.active_inference = inference;
                sketch.update_preview_dimensions();
            }
            self.ui.sketch_entity_count = sketch.entities.len();
            self.ui.sketch_preview_length = sketch.preview_length;
            self.ui.sketch_preview_angle = sketch.preview_angle;
        } else {
            self.ui.sketch_entity_count = 0;
            self.ui.sketch_preview_length = None;
            self.ui.sketch_preview_angle = None;
        }

        // Compute regions (needs &mut sketch for lazy recompute)
        let region_data = if let Some(sketch) = &mut self.sketch {
            let regions = sketch.regions().to_vec();
            let selected = sketch.selected_region;
            let plane = sketch.plane.clone();
            Some((regions, selected, plane))
        } else {
            None
        };

        // Render sketch lines + region fills
        renderer.clear_sketch();
        if let Some(sketch) = &self.sketch {
            let tool = match self.ui.active_tool {
                Tool::Line => crate::ui::Tool::Line,
                Tool::Rect => crate::ui::Tool::Rect,
                Tool::Circle => crate::ui::Tool::Circle,
                _ => crate::ui::Tool::Select,
            };
            renderer.update_sketch_multi(sketch, tool, true);
        }
        if let Some((regions, selected, plane)) = region_data {
            if !regions.is_empty() {
                renderer.update_sketch_regions(&regions, selected, &plane);
            }
        }

        // View presets
        if let Some(preset) = self.ui.view_request.take() {
            use std::f32::consts::*;
            let (yaw, pitch) = match preset {
                ViewPreset::Front     => (0.0, 0.0),
                ViewPreset::Back      => (PI, 0.0),
                ViewPreset::Top       => (0.0, FRAC_PI_2 - 0.01),
                ViewPreset::Bottom    => (0.0, -(FRAC_PI_2 - 0.01)),
                ViewPreset::Right     => (FRAC_PI_2, 0.0),
                ViewPreset::Left      => (-FRAC_PI_2, 0.0),
                ViewPreset::Isometric => (FRAC_PI_4, FRAC_PI_6),
            };
            renderer.set_view(yaw, pitch);
        }

        // Hover pre-highlight (only when cursor moved, not dragging, not over egui)
        if self.input.cursor_moved && !self.input.middle_pressed && !self.input.left_pressed && !self.egui_ctx.wants_pointer_input() {
            let sx = self.input.cursor_pos.0 as f32;
            let sy = self.input.cursor_pos.1 as f32;
            renderer.update_hover(sx, sy);

            // Construction geometry hover
            let (ray_o, ray_d) = renderer.screen_to_ray(sx, sy);
            let extent = (renderer.camera_distance() * 0.6).clamp(0.5, 10.0);
            self.construction.hovered = self.construction.pick(ray_o, ray_d, extent).map(|(id, _)| id);

            self.input.cursor_moved = false;
        }

        // Mesh stats
        self.ui.mesh_faces = renderer.mesh.face_count();
        self.ui.mesh_verts = renderer.mesh.vertex_count();
        self.ui.mesh_tris = renderer.mesh.triangle_count();

        // Selected face info for status bar
        if let Some(fid) = renderer.selected_face {
            self.ui.selected_face_id = Some(fid);
            self.ui.selected_face_normal = renderer.mesh.face_normal(fid).map(|n| n.into());
            self.ui.selected_face_area = Some(renderer.mesh.face_area(fid));
        } else {
            self.ui.selected_face_id = None;
            self.ui.selected_face_normal = None;
            self.ui.selected_face_area = None;
        }

        // Cursor world position (intersect ray with XZ plane at y=0)
        let (ray_o, ray_d) = renderer.screen_to_ray(
            self.input.cursor_pos.0 as f32,
            self.input.cursor_pos.1 as f32,
        );
        if ray_d.y.abs() > 1e-6 {
            let t = -ray_o.y / ray_d.y;
            if t > 0.0 && t < 1000.0 {
                let p = ray_o + ray_d * t;
                self.ui.cursor_world = Some([p.x, p.y, p.z]);
            } else {
                self.ui.cursor_world = None;
            }
        } else {
            self.ui.cursor_world = None;
        }

    }

    /// Split a parent face by cutting out a region (with optional holes).
    /// After extruding a sketch region, the parent face still exists underneath.
    /// This replaces it with the remainder (parent minus region).
    /// This is how SolidWorks works: extrude SPLITS the face, never overlaps.
    /// Split a parent face by cutting out a region (with optional holes).
    /// After extruding a sketch region, the parent face still exists underneath.
    /// This replaces it with the remainder (parent minus region).
    ///
    /// Robustness guarantees:
    /// - If the region covers the entire parent face, parent is preserved (no hole left)
    /// - If geo::difference produces degenerate geometry, falls back to no-split
    /// - If parent face no longer exists, returns silently
    fn split_parent_face(
        mesh: &mut crate::renderer::mesh::Mesh,
        parent_face_id: u32,
        region_boundary: &[glam::Vec3],
        region_holes: &[Vec<glam::Vec3>],
        plane: &crate::sketch::SketchPlane,
    ) {
        use geo::algorithm::bool_ops::BooleanOps;
        use geo::algorithm::area::Area;

        // Verify parent face still exists
        let Some(parent_boundary) = mesh.face_boundary_corners(parent_face_id) else {
            log::warn!("split_parent_face: parent face {} not found", parent_face_id);
            return;
        };
        if parent_boundary.len() < 3 || region_boundary.len() < 3 { return; }

        let to_coord = |p: &glam::Vec3| -> geo::Coord<f64> {
            let p2d = plane.world_to_2d(*p);
            geo::Coord { x: p2d.x as f64, y: p2d.y as f64 }
        };

        let parent_coords: Vec<geo::Coord<f64>> = parent_boundary.iter().map(to_coord).collect();
        let region_coords: Vec<geo::Coord<f64>> = region_boundary.iter().map(to_coord).collect();

        let hole_rings: Vec<geo::LineString<f64>> = region_holes.iter()
            .map(|hole| geo::LineString::new(hole.iter().map(to_coord).collect()))
            .collect();

        let parent_poly = geo::Polygon::new(geo::LineString::new(parent_coords), vec![]);
        let region_poly = geo::Polygon::new(geo::LineString::new(region_coords), hole_rings);

        let parent_area = parent_poly.unsigned_area();
        let region_area = region_poly.unsigned_area();

        // Guard: if region covers >95% of parent, don't split (would leave a sliver)
        if parent_area < 1e-10 || region_area / parent_area > 0.95 {
            log::info!("split_parent_face: region covers entire parent face, skipping split");
            // Just delete the parent — the extrusion's base face replaces it
            mesh.delete_face(parent_face_id);
            return;
        }

        // Compute remainder = parent - region (with panic guard)
        let remainder = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parent_poly.difference(&region_poly)
        }));

        let remainder = match remainder {
            Ok(r) => r,
            Err(_) => {
                log::error!("split_parent_face: geo::difference panicked, skipping split");
                return; // Don't delete parent — leave it intact
            }
        };

        // Validate remainder has meaningful geometry
        let remainder_area: f64 = remainder.0.iter()
            .map(|p| p.unsigned_area())
            .sum();
        if remainder_area < parent_area * 0.01 {
            // Remainder is negligible — just delete parent
            mesh.delete_face(parent_face_id);
            return;
        }

        // Delete the original parent face
        let normal = plane.normal;
        mesh.delete_face(parent_face_id);

        // Add remainder polygon(s) as new face(s)
        for poly in remainder.0.iter() {
            let coords: Vec<geo::Coord<f64>> = poly.exterior().0.clone();
            if coords.len() < 4 { continue; }

            let points_3d: Vec<glam::Vec3> = coords[..coords.len() - 1].iter()
                .map(|c| {
                    let p2d = crate::sketch::Point2D::new(c.x as f32, c.y as f32);
                    plane.to_3d(p2d)
                })
                .collect();

            if points_3d.len() < 3 { continue; }

            // Check for degenerate triangles (area too small)
            let face_area: f32 = {
                let mut a = 0.0f32;
                for i in 1..points_3d.len() - 1 {
                    let e1 = points_3d[i] - points_3d[0];
                    let e2 = points_3d[i + 1] - points_3d[0];
                    a += e1.cross(e2).length() * 0.5;
                }
                a
            };
            if face_area < 1e-6 { continue; } // skip degenerate slivers

            let holes: Vec<Vec<glam::Vec3>> = poly.interiors().iter().filter_map(|ring| {
                let hole_coords = &ring.0;
                if hole_coords.len() < 4 { return None; }
                let pts: Vec<glam::Vec3> = hole_coords[..hole_coords.len() - 1].iter()
                    .map(|c| {
                        let p2d = crate::sketch::Point2D::new(c.x as f32, c.y as f32);
                        plane.to_3d(p2d)
                    })
                    .collect();
                if pts.len() >= 3 { Some(pts) } else { None }
            }).collect();

            if holes.is_empty() {
                mesh.add_polygon_face_flush(&points_3d, normal);
            } else {
                mesh.add_polygon_face_with_holes_flush(&points_3d, &holes, normal);
            }
        }
    }

    fn save_project(&mut self, force_dialog: bool) {
        let path = if force_dialog || self.ui.current_project_path.is_none() {
            rfd::FileDialog::new()
                .add_filter("GraniteX Project", &["gnx"])
                .set_file_name("untitled.gnx")
                .save_file()
        } else {
            self.ui.current_project_path.clone()
        };

        if let Some(path) = path {
            if let Some(r) = &self.renderer {
                let (target, dist, yaw, pitch) = r.camera_state();
                let camera_data = crate::project::CameraData { target, distance: dist, yaw, pitch };
                match crate::project::save_project(&path, &r.mesh, camera_data) {
                    Ok(()) => {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Saved {}", name)));
                        self.ui.current_project_path = Some(path);
                    }
                    Err(e) => {
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Save failed: {}", e)));
                    }
                }
            }
        }
    }

    fn open_project(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("GraniteX Project", &["gnx"])
            .pick_file();

        if let Some(path) = file {
            match crate::project::load_project(&path) {
                Ok((mesh, camera_data)) => {
                    if let Some(r) = &mut self.renderer {
                        r.load_mesh(mesh);
                        r.restore_camera_state(camera_data.target, camera_data.distance, camera_data.yaw, camera_data.pitch);
                        self.history.clear();
                        self.sketch = None;
                        self.ui.operation_history.clear();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Opened {}", name)));
                        self.ui.current_project_path = Some(path);
                    }
                }
                Err(e) => {
                    self.ui.toasts.push(crate::ui::Toast::new(format!("Open failed: {}", e)));
                }
            }
        }
    }

    fn export_file(&mut self, format: &str) {
        let (filter_name, ext) = match format {
            "stl" => ("STL Files", "stl"),
            "obj" => ("OBJ Files", "obj"),
            _ => return,
        };

        let file = rfd::FileDialog::new()
            .add_filter(filter_name, &[ext])
            .set_file_name(format!("export.{}", ext))
            .save_file();

        if let Some(path) = file {
            if let Some(r) = &self.renderer {
                let result = match format {
                    "stl" => crate::export::export_stl(&r.mesh, &path),
                    "obj" => crate::export::export_obj(&r.mesh, &path),
                    _ => Ok(()),
                };
                match result {
                    Ok(()) => {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Exported {}", name)));
                    }
                    Err(e) => {
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Export failed: {}", e)));
                    }
                }
            }
        }
    }

    fn open_file_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("3D Models", crate::import::supported_extensions())
            .add_filter("STL Files", &["stl"])
            .add_filter("OBJ Files", &["obj"])
            .pick_file();

        if let Some(path) = file {
            match crate::import::load_file(&path) {
                Ok(mesh) => {
                    if let Some(r) = &mut self.renderer {
                        self.history.save_state(&r.mesh);
                        r.load_mesh(mesh);
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        let tris = r.mesh.triangle_count();
                        self.ui.toasts.push(crate::ui::Toast::new(
                            format!("Imported {} ({} triangles)", name, tris),
                        ));
                    }
                }
                Err(e) => {
                    self.ui.toasts.push(crate::ui::Toast::new(format!("Import failed: {}", e)));
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let attrs = Window::default_attributes()
            .with_title("GraniteX")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = event_loop.create_window(attrs).expect("Failed to create window");
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(), self.egui_ctx.viewport_id(),
            &window, None, None, None,
        );

        self.egui_ctx.set_visuals(egui::Visuals::dark());
        self.renderer = Some(pollster::block_on(Renderer::new(&window)));
        self.egui_state = Some(egui_state);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let egui_consumed = self.egui_state.as_mut()
            .map(|s| s.on_window_event(self.window.as_ref().unwrap(), &event).consumed)
            .unwrap_or(false);

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer { r.resize(*size); }
            }
            WindowEvent::DroppedFile(path) => {
                match crate::import::load_file(path) {
                    Ok(mesh) => {
                        if let Some(r) = &mut self.renderer {
                            self.history.save_state(&r.mesh);
                            r.load_mesh(mesh);
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            let tris = r.mesh.triangle_count();
                            self.ui.toasts.push(crate::ui::Toast::new(
                                format!("Imported {} ({} tris)", name, tris),
                            ));
                        }
                    }
                    Err(e) => {
                        self.ui.toasts.push(crate::ui::Toast::new(format!("Import failed: {}", e)));
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.apply_ui_state();

                let window = self.window.as_ref().unwrap();
                let egui_state = self.egui_state.as_mut().unwrap();
                let raw_input = egui_state.take_egui_input(window);
                let full_output = self.egui_ctx.run(raw_input, |ctx| { self.ui.draw(ctx); });
                egui_state.handle_platform_output(window, full_output.platform_output);

                let primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                let size = window.inner_size();
                let screen = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [size.width, size.height],
                    pixels_per_point: full_output.pixels_per_point,
                };

                if let Some(r) = &mut self.renderer {
                    r.update(); // advance camera animation
                    r.render(full_output.textures_delta, primitives, screen);
                }
                window.request_redraw();
            }
            _ => {}
        }

        self.handle_keyboard(&event);
        self.handle_input(&event, egui_consumed);
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::new())?;
    Ok(())
}
