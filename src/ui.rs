#[derive(PartialEq, Clone, Copy)]
pub enum RightPanelTab {
    Inspector,
    Agent,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tool {
    Select,
    Move,
    Extrude,
    Cut,
    Inset,
    #[allow(dead_code)]
    Fillet,
    Line,
    Rect,
    Circle,
    CLine,
    Measure,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SelectionMode {
    Face,
    Edge,
}

pub struct Measurement {
    pub point_a: [f32; 3],
    pub point_b: [f32; 3],
    pub distance: f32,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SketchTool {
    Line,
    Rect,
    Circle,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ViewPreset {
    Front,
    Back,
    Top,
    Bottom,
    Left,
    Right,
    Isometric,
}

// --- Command Palette ---

pub struct PaletteCommand {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub action: PaletteAction,
}

#[derive(Clone, Copy)]
pub enum PaletteAction {
    SetTool(Tool),
    SetView(ViewPreset),
    Undo,
    Redo,
    Delete,
    NewScene,
    Import,
    Save,
    Open,
    ExportStl,
    ExportObj,
    FitCamera,
    ToggleGrid,
    ToggleWireframe,
    SelectionModeFace,
    SelectionModeEdge,
}

pub fn all_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand { name: "Select Tool", shortcut: "S", action: PaletteAction::SetTool(Tool::Select) },
        PaletteCommand { name: "Move Tool", shortcut: "G", action: PaletteAction::SetTool(Tool::Move) },
        PaletteCommand { name: "Extrude Tool", shortcut: "E", action: PaletteAction::SetTool(Tool::Extrude) },
        PaletteCommand { name: "Cut Tool", shortcut: "X", action: PaletteAction::SetTool(Tool::Cut) },
        PaletteCommand { name: "Inset Tool", shortcut: "I", action: PaletteAction::SetTool(Tool::Inset) },
        PaletteCommand { name: "Line Tool", shortcut: "L", action: PaletteAction::SetTool(Tool::Line) },
        PaletteCommand { name: "Rectangle Tool", shortcut: "R", action: PaletteAction::SetTool(Tool::Rect) },
        PaletteCommand { name: "Circle Tool", shortcut: "C", action: PaletteAction::SetTool(Tool::Circle) },
        PaletteCommand { name: "Measure Tool", shortcut: "M", action: PaletteAction::SetTool(Tool::Measure) },
        PaletteCommand { name: "View Front", shortcut: "Num 1", action: PaletteAction::SetView(ViewPreset::Front) },
        PaletteCommand { name: "View Top", shortcut: "Num 7", action: PaletteAction::SetView(ViewPreset::Top) },
        PaletteCommand { name: "View Right", shortcut: "Num 3", action: PaletteAction::SetView(ViewPreset::Right) },
        PaletteCommand { name: "View Isometric", shortcut: "Num 0", action: PaletteAction::SetView(ViewPreset::Isometric) },
        PaletteCommand { name: "View Back", shortcut: "Ctrl+1", action: PaletteAction::SetView(ViewPreset::Back) },
        PaletteCommand { name: "View Bottom", shortcut: "Ctrl+7", action: PaletteAction::SetView(ViewPreset::Bottom) },
        PaletteCommand { name: "View Left", shortcut: "Ctrl+3", action: PaletteAction::SetView(ViewPreset::Left) },
        PaletteCommand { name: "Undo", shortcut: "Ctrl+Z", action: PaletteAction::Undo },
        PaletteCommand { name: "Redo", shortcut: "Ctrl+Y", action: PaletteAction::Redo },
        PaletteCommand { name: "Delete Face", shortcut: "Del", action: PaletteAction::Delete },
        PaletteCommand { name: "New Scene", shortcut: "Ctrl+N", action: PaletteAction::NewScene },
        PaletteCommand { name: "Import File", shortcut: "Ctrl+I", action: PaletteAction::Import },
        PaletteCommand { name: "Save Project", shortcut: "Ctrl+S", action: PaletteAction::Save },
        PaletteCommand { name: "Open Project", shortcut: "Ctrl+O", action: PaletteAction::Open },
        PaletteCommand { name: "Export STL", shortcut: "", action: PaletteAction::ExportStl },
        PaletteCommand { name: "Export OBJ", shortcut: "", action: PaletteAction::ExportObj },
        PaletteCommand { name: "Fit Camera", shortcut: "Home", action: PaletteAction::FitCamera },
        PaletteCommand { name: "Toggle Grid", shortcut: "", action: PaletteAction::ToggleGrid },
        PaletteCommand { name: "Toggle Wireframe", shortcut: "W", action: PaletteAction::ToggleWireframe },
        PaletteCommand { name: "Face Selection Mode", shortcut: "Tab", action: PaletteAction::SelectionModeFace },
        PaletteCommand { name: "Edge Selection Mode", shortcut: "Tab", action: PaletteAction::SelectionModeEdge },
    ]
}

pub struct UiState {
    pub show_grid: bool,
    pub show_wireframe: bool,
    pub show_chat: bool,
    pub active_tool: Tool,
    pub view_request: Option<ViewPreset>,
    pub extrude_request: Option<f32>,
    pub extrude_distance: f32,
    pub cut_request: Option<f32>,
    pub cut_depth: f32,
    pub inset_request: Option<f32>,
    pub inset_amount: f32,
    pub import_request: bool,
    pub new_scene_request: bool,
    pub save_request: bool,
    pub save_as_request: bool,
    pub open_project_request: bool,
    pub export_stl_request: bool,
    pub export_obj_request: bool,
    pub current_project_path: Option<std::path::PathBuf>,
    // Measurement
    pub measure_first_point: Option<[f32; 3]>,
    pub active_measurement: Option<Measurement>,
    // Selection mode
    pub selection_mode: SelectionMode,
    pub selected_edge: Option<([f32; 3], [f32; 3])>,
    // Dirty (unsaved changes) indicator
    pub dirty: bool,
    // Auto-save toggle
    pub auto_save_enabled: bool,
    pub chat_input: String,
    pub chat_history: Vec<ChatMessage>,
    pub selected_feature: Option<String>,
    pub operation_history: Vec<String>,
    pub mesh_faces: u32,
    pub mesh_verts: usize,
    pub mesh_tris: usize,
    pub sketch_entity_count: usize,
    pub wireframe_supported: bool,
    // Status bar info (updated per-frame)
    pub cursor_world: Option<[f32; 3]>,
    pub selected_face_id: Option<u32>,
    pub selected_face_normal: Option<[f32; 3]>,
    pub selected_face_area: Option<f32>,
    pub toasts: Vec<Toast>,
    // Right-click context menu
    pub context_menu_pos: Option<egui::Pos2>,
    pub context_menu_face: Option<u32>,
    pub context_menu_action: Option<ContextAction>,
    /// Whether a preview (extrude/cut/inset ghost) is currently showing
    pub preview_active: bool,
    /// Sketch preview dimensions (shown as floating labels)
    pub sketch_preview_length: Option<f32>,
    pub sketch_preview_angle: Option<f32>,
    // Construction geometry
    pub construction_selected: Option<crate::construction::ConstructionId>,
    pub show_construction_planes: bool,
    pub show_construction_axes: bool,
    // Right panel inspector
    pub right_panel_tab: RightPanelTab,
    pub face_centroid: Option<[f32; 3]>,
    pub face_planar: Option<bool>,
    pub mesh_bbox_min: Option<[f32; 3]>,
    pub mesh_bbox_max: Option<[f32; 3]>,
    // Transform sliders for move operation
    pub move_x: f32,
    pub move_y: f32,
    pub move_z: f32,
    // Command palette
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub command_palette_selected: usize,
    pub command_palette_action: Option<PaletteAction>,
    pub dark_mode: bool,
}

pub struct ChatMessage {
    pub sender: Sender,
    pub text: String,
}

pub enum Sender {
    User,
    Agent,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ContextAction {
    Extrude,
    Cut,
    Inset,
    Delete,
    ZoomToFace,
}

pub struct Toast {
    pub text: String,
    pub created: std::time::Instant,
    pub duration_secs: f32,
}

impl Toast {
    pub fn new(text: String) -> Self {
        Self { text, created: std::time::Instant::now(), duration_secs: 3.0 }
    }

    pub fn is_expired(&self) -> bool {
        self.created.elapsed().as_secs_f32() > self.duration_secs
    }

    pub fn alpha(&self) -> f32 {
        let elapsed = self.created.elapsed().as_secs_f32();
        let fade_start = self.duration_secs - 0.5;
        if elapsed > fade_start {
            1.0 - ((elapsed - fade_start) / 0.5).min(1.0)
        } else {
            1.0
        }
    }
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_grid: true,
            show_wireframe: false,
            show_chat: true,
            active_tool: Tool::Select,
            view_request: None,
            extrude_request: None,
            extrude_distance: 0.5,
            cut_request: None,
            cut_depth: 0.3,
            inset_request: None,
            inset_amount: 0.1,
            import_request: false,
            new_scene_request: false,
            save_request: false,
            save_as_request: false,
            open_project_request: false,
            export_stl_request: false,
            export_obj_request: false,
            current_project_path: None,
            measure_first_point: None,
            active_measurement: None,
            selection_mode: SelectionMode::Face,
            selected_edge: None,
            dirty: false,
            auto_save_enabled: true,
            chat_input: String::new(),
            chat_history: vec![
                ChatMessage {
                    sender: Sender::Agent,
                    text: "Welcome to GraniteX. Describe what you want to build.".into(),
                },
            ],
            selected_feature: None,
            operation_history: Vec::new(),
            mesh_faces: 6,
            mesh_verts: 24,
            mesh_tris: 12,
            sketch_entity_count: 0,
            wireframe_supported: false,
            cursor_world: None,
            selected_face_id: None,
            selected_face_normal: None,
            selected_face_area: None,
            toasts: Vec::new(),
            context_menu_pos: None,
            context_menu_face: None,
            context_menu_action: None,
            preview_active: false,
            sketch_preview_length: None,
            sketch_preview_angle: None,
            construction_selected: None,
            show_construction_planes: true,
            show_construction_axes: true,
            right_panel_tab: RightPanelTab::Inspector,
            face_centroid: None,
            face_planar: None,
            mesh_bbox_min: None,
            mesh_bbox_max: None,
            move_x: 0.0,
            move_y: 0.0,
            move_z: 0.0,
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            command_palette_action: None,
            dark_mode: true,
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        // Apply theme based on dark_mode toggle
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        self.draw_top_toolbar(ctx);
        self.draw_bottom_bar(ctx);
        self.draw_left_panel(ctx);
        if self.show_chat {
            self.draw_chat_panel(ctx);
        }
        self.draw_toasts(ctx);
        self.draw_context_menu(ctx);
        self.draw_dimension_label(ctx);
        self.draw_sketch_dimensions(ctx);
        self.draw_command_palette(ctx);
    }

    fn draw_top_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .exact_height(30.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;

                    let title = if self.dirty { "GraniteX *" } else { "GraniteX" };
                    ui.label(egui::RichText::new(title).strong().size(13.0));
                    ui.separator();

                    // File operations
                    if ui.add(egui::Button::new(egui::RichText::new("New").size(11.0))).clicked() {
                        self.new_scene_request = true;
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("Save").size(11.0))).clicked() {
                        self.save_request = true;
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("Open").size(11.0))).clicked() {
                        self.open_project_request = true;
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("Import").size(11.0))).clicked() {
                        self.import_request = true;
                    }

                    // Export dropdown
                    egui::ComboBox::from_id_salt("export_combo")
                        .selected_text(egui::RichText::new("Export").size(11.0))
                        .width(50.0)
                        .show_ui(ui, |ui| {
                            if ui.button("STL").clicked() { self.export_stl_request = true; }
                            if ui.button("OBJ").clicked() { self.export_obj_request = true; }
                        });

                    ui.separator();

                    // Operations
                    let active_ops: &[(Tool, &str)] = &[
                        (Tool::Select,  "Select"),
                        (Tool::Move,    "Move"),
                        (Tool::Extrude, "Extrude"),
                        (Tool::Cut,     "Cut"),
                        (Tool::Inset,   "Inset"),
                    ];
                    for (tool, label) in active_ops {
                        let btn = egui::Button::new(egui::RichText::new(*label).size(11.0))
                            .selected(self.active_tool == *tool);
                        if ui.add(btn).clicked() {
                            self.active_tool = if self.active_tool == *tool { Tool::Select } else { *tool };
                        }
                    }
                    // Fillet — not yet implemented, shown disabled
                    ui.add_enabled(false, egui::Button::new(
                        egui::RichText::new("Fillet").size(11.0).weak()
                    )).on_disabled_hover_text("Coming soon");

                    // Measure tool
                    let measure_btn = egui::Button::new(egui::RichText::new("Measure").size(11.0))
                        .selected(self.active_tool == Tool::Measure);
                    if ui.add(measure_btn).clicked() {
                        self.active_tool = if self.active_tool == Tool::Measure { Tool::Select } else { Tool::Measure };
                        if self.active_tool != Tool::Measure {
                            self.measure_first_point = None;
                            self.active_measurement = None;
                        }
                    }

                    ui.separator();

                    // Selection mode toggle
                    let mode_label = match self.selection_mode {
                        SelectionMode::Face => "Face",
                        SelectionMode::Edge => "Edge",
                    };
                    if ui.add(egui::Button::new(egui::RichText::new(mode_label).size(11.0))).on_hover_text("Tab to toggle").clicked() {
                        self.selection_mode = match self.selection_mode {
                            SelectionMode::Face => SelectionMode::Edge,
                            SelectionMode::Edge => SelectionMode::Face,
                        };
                    }

                    ui.separator();

                    // Drawing tools
                    ui.label(egui::RichText::new("Draw:").weak().size(10.0));
                    let draw_tools = [
                        (Tool::Line,   "Line"),
                        (Tool::Rect,   "Rect"),
                        (Tool::Circle, "Circle"),
                        (Tool::CLine,  "CLine"),
                    ];
                    for (tool, label) in &draw_tools {
                        let btn = egui::Button::new(egui::RichText::new(*label).size(11.0))
                            .selected(self.active_tool == *tool);
                        if ui.add(btn).clicked() {
                            self.active_tool = if self.active_tool == *tool { Tool::Select } else { *tool };
                        }
                    }

                    ui.separator();

                    // View presets
                    ui.label(egui::RichText::new("View:").weak().size(10.0));
                    for (preset, label) in [
                        (ViewPreset::Front, "F"),
                        (ViewPreset::Back, "Bk"),
                        (ViewPreset::Top, "T"),
                        (ViewPreset::Bottom, "Bt"),
                        (ViewPreset::Right, "R"),
                        (ViewPreset::Left, "L"),
                        (ViewPreset::Isometric, "Iso"),
                    ] {
                        if ui.small_button(label).clicked() {
                            self.view_request = Some(preset);
                        }
                    }

                    ui.separator();
                    ui.checkbox(&mut self.show_grid, egui::RichText::new("Grid").size(10.0));
                    if self.wireframe_supported {
                        ui.checkbox(&mut self.show_wireframe, egui::RichText::new("Wire").size(10.0));
                    }
                    ui.checkbox(&mut self.show_chat, egui::RichText::new("Chat").size(10.0));
                    ui.checkbox(&mut self.auto_save_enabled, egui::RichText::new("Auto-save").size(10.0));
                    ui.separator();
                    if ui.selectable_label(self.dark_mode, egui::RichText::new("Dark").size(10.0)).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                });
            });
    }

    fn draw_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("feature_tree")
            .exact_width(150.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Features").strong().size(12.0));
                ui.separator();

                egui::CollapsingHeader::new(egui::RichText::new("Origin").size(11.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        // Planes
                        let plane_names = ["XY Plane", "XZ Plane", "YZ Plane"];
                        let plane_colors = [
                            egui::Color32::from_rgb(80, 80, 230),  // XY = blue
                            egui::Color32::from_rgb(50, 200, 50),  // XZ = green
                            egui::Color32::from_rgb(230, 50, 50),  // YZ = red
                        ];
                        for (i, (name, color)) in plane_names.iter().zip(plane_colors.iter()).enumerate() {
                            let id = crate::construction::ConstructionId::Plane(i);
                            let is_sel = self.construction_selected == Some(id);
                            ui.horizontal(|ui| {
                                let dot = egui::RichText::new("*").size(11.0).color(*color);
                                ui.label(dot);
                                let label = if is_sel {
                                    egui::RichText::new(*name).size(11.0).strong().color(egui::Color32::from_rgb(100, 160, 255))
                                } else {
                                    egui::RichText::new(*name).size(11.0)
                                };
                                if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                                    self.construction_selected = if is_sel { None } else { Some(id) };
                                    self.selected_feature = Some(name.to_string());
                                }
                            });
                        }

                        // Axes
                        let axis_names = ["X Axis", "Y Axis", "Z Axis"];
                        let axis_colors = [
                            egui::Color32::from_rgb(230, 50, 50),  // X = red
                            egui::Color32::from_rgb(50, 200, 50),  // Y = green
                            egui::Color32::from_rgb(80, 100, 230), // Z = blue
                        ];
                        for (i, (name, color)) in axis_names.iter().zip(axis_colors.iter()).enumerate() {
                            let id = crate::construction::ConstructionId::Axis(i);
                            let is_sel = self.construction_selected == Some(id);
                            ui.horizontal(|ui| {
                                let dot = egui::RichText::new("-").size(11.0).color(*color);
                                ui.label(dot);
                                let label = if is_sel {
                                    egui::RichText::new(*name).size(11.0).strong().color(egui::Color32::from_rgb(100, 160, 255))
                                } else {
                                    egui::RichText::new(*name).size(11.0)
                                };
                                if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                                    self.construction_selected = if is_sel { None } else { Some(id) };
                                    self.selected_feature = Some(name.to_string());
                                }
                            });
                        }

                        // Visibility toggles
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_construction_planes, egui::RichText::new("Planes").size(9.0));
                            ui.checkbox(&mut self.show_construction_axes, egui::RichText::new("Axes").size(9.0));
                        });
                    });

                ui.separator();

                egui::CollapsingHeader::new(egui::RichText::new("History").size(11.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("  Cube (base)").size(10.0).weak());
                        for (_i, op) in self.operation_history.iter().enumerate() {
                            let icon = if op.starts_with("Extrude") { "+" }
                                else if op.starts_with("Cut") { "-" }
                                else if op.starts_with("Inset") { ">" }
                                else if op.starts_with("Import") { "@" }
                                else { "*" };
                            let text = format!("  {} {}", icon, op);
                            let color = if op.starts_with("Cut") {
                                egui::Color32::from_rgb(230, 100, 80)
                            } else {
                                egui::Color32::from_rgb(180, 180, 190)
                            };
                            ui.label(egui::RichText::new(text).size(10.0).color(color));
                        }
                        if self.operation_history.is_empty() {
                            ui.label(egui::RichText::new("  (no operations yet)").size(9.0).weak());
                        }
                    });

                ui.separator();
                ui.add_space(8.0);

                // Tool-specific controls
                match self.active_tool {
                    Tool::Extrude => {
                        ui.label(egui::RichText::new("Extrude").strong().size(11.0));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Dist:").size(10.0));
                            ui.add(egui::DragValue::new(&mut self.extrude_distance)
                                .speed(0.01)
                                .range(-10.0..=10.0)
                                .suffix(" m"));
                        });
                        ui.add_space(4.0);
                        if ui.button(egui::RichText::new("Apply Extrude").size(11.0)).clicked() {
                            self.extrude_request = Some(self.extrude_distance);
                        }
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Or drag on viewport").weak().size(9.0));
                    }
                    Tool::Line | Tool::Rect | Tool::Circle | Tool::CLine => {
                        let name = match self.active_tool {
                            Tool::Line => "Line",
                            Tool::Rect => "Rectangle",
                            Tool::Circle => "Circle",
                            Tool::CLine => "Construction Line",
                            _ => "",
                        };
                        let color = if self.active_tool == Tool::CLine {
                            egui::Color32::from_rgb(255, 165, 50) // orange for construction
                        } else {
                            egui::Color32::from_rgb(100, 200, 100)
                        };
                        ui.label(egui::RichText::new(name).strong().size(11.0).color(color));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Click on a face or plane").weak().size(9.0));
                        if self.sketch_entity_count > 0 {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(format!("Entities: {}", self.sketch_entity_count)).size(10.0));
                        }
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Esc = cancel | Ctrl+Z = undo").weak().size(8.0));
                        ui.label(egui::RichText::new("Right-click = stop chain").weak().size(8.0));
                    }
                    Tool::Cut => {
                        ui.label(egui::RichText::new("Cut").strong().size(11.0).color(egui::Color32::from_rgb(230, 80, 60)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Depth:").size(10.0));
                            ui.add(egui::DragValue::new(&mut self.cut_depth)
                                .speed(0.01)
                                .range(0.01..=10.0)
                                .suffix(" m"));
                        });
                        ui.add_space(4.0);
                        if ui.button(egui::RichText::new("Apply Cut").size(11.0)).clicked() {
                            self.cut_request = Some(self.cut_depth);
                        }
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Select a face, then cut").weak().size(9.0));
                    }
                    Tool::Inset => {
                        ui.label(egui::RichText::new("Inset").strong().size(11.0).color(egui::Color32::from_rgb(100, 200, 200)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Amount:").size(10.0));
                            ui.add(egui::DragValue::new(&mut self.inset_amount)
                                .speed(0.005)
                                .range(0.01..=5.0)
                                .suffix(" m"));
                        });
                        ui.add_space(4.0);
                        if ui.button(egui::RichText::new("Apply Inset").size(11.0)).clicked() {
                            self.inset_request = Some(self.inset_amount);
                        }
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Select a face, then inset").weak().size(9.0));
                    }
                    Tool::Move => {
                        ui.label(egui::RichText::new("Move (Translate)").strong().size(11.0));
                        ui.separator();
                        if self.selected_face_id.is_some() {
                            ui.label(egui::RichText::new("Drag an axis arrow to move").weak().size(9.0));
                            ui.label(egui::RichText::new("the selected face.").weak().size(9.0));
                        } else {
                            ui.label(egui::RichText::new("Select a face first.").weak().size(9.0));
                        }
                    }
                    Tool::Measure => {
                        ui.label(egui::RichText::new("Measure").strong().size(11.0).color(egui::Color32::from_rgb(255, 200, 50)));
                        ui.add_space(4.0);
                        if let Some(ref m) = self.active_measurement {
                            ui.label(egui::RichText::new(format!("Distance: {:.4} m", m.distance)).size(11.0));
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(format!("dX: {:.4}", m.point_b[0] - m.point_a[0])).size(10.0).weak());
                            ui.label(egui::RichText::new(format!("dY: {:.4}", m.point_b[1] - m.point_a[1])).size(10.0).weak());
                            ui.label(egui::RichText::new(format!("dZ: {:.4}", m.point_b[2] - m.point_a[2])).size(10.0).weak());
                            ui.add_space(4.0);
                            if ui.button(egui::RichText::new("Clear").size(11.0)).clicked() {
                                self.active_measurement = None;
                                self.measure_first_point = None;
                            }
                        } else if self.measure_first_point.is_some() {
                            ui.label(egui::RichText::new("Click second point").weak().size(9.0));
                        } else {
                            ui.label(egui::RichText::new("Click first point on model").weak().size(9.0));
                        }
                    }
                    _ => {
                        let name = match self.active_tool {
                            Tool::Select => "Select",
                            Tool::Fillet => "Fillet",
                            _ => "",
                        };
                        ui.label(egui::RichText::new(format!("Tool: {}", name)).weak().size(10.0));
                    }
                }
            });
    }

    fn draw_chat_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("chat_panel")
            .default_width(240.0)
            .min_width(180.0)
            .max_width(400.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Agent").strong().size(12.0));
                ui.separator();

                let available_height = ui.available_height() - 32.0;
                egui::ScrollArea::vertical()
                    .max_height(available_height.max(50.0))
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in &self.chat_history {
                            let (prefix, color) = match msg.sender {
                                Sender::Agent => ("Agent", egui::Color32::from_rgb(100, 160, 255)),
                                Sender::User => ("You", egui::Color32::from_rgb(170, 170, 170)),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(format!("{}: ", prefix)).strong().color(color).size(11.0));
                                ui.label(egui::RichText::new(&msg.text).size(11.0));
                            });
                            ui.add_space(3.0);
                        }
                    });

                ui.separator();
                let response = ui.horizontal(|ui| {
                    let text_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.chat_input)
                            .hint_text("Type here...")
                            .desired_width(ui.available_width() - 40.0)
                            .font(egui::TextStyle::Small),
                    );
                    let send = ui.small_button(">");
                    text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || send.clicked()
                });

                if response.inner && !self.chat_input.trim().is_empty() {
                    let user_text = self.chat_input.trim().to_string();
                    self.chat_history.push(ChatMessage { sender: Sender::User, text: user_text.clone() });
                    self.chat_history.push(ChatMessage {
                        sender: Sender::Agent,
                        text: format!("Understood: \"{}\". (Engine not connected yet)", user_text),
                    });
                    self.chat_input.clear();
                }
            });
    }

    fn draw_bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(20.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Active tool
                    let tool_name = match self.active_tool {
                        Tool::Select => "Select", Tool::Move => "Move",
                        Tool::Extrude => "Extrude",
                        Tool::Cut => "Cut", Tool::Inset => "Inset",
                        Tool::Fillet => "Fillet", Tool::Line => "Line",
                        Tool::Rect => "Rect", Tool::Circle => "Circle",
                        Tool::Measure => "Measure", Tool::CLine => "CLine",
                    };
                    ui.label(egui::RichText::new(tool_name).strong().size(10.0));
                    ui.separator();

                    // Mesh stats
                    ui.label(egui::RichText::new(format!("F:{} V:{} T:{}", self.mesh_faces, self.mesh_verts, self.mesh_tris)).weak().size(10.0));

                    // Selected face info
                    if let Some(fid) = self.selected_face_id {
                        ui.separator();
                        let mut info = format!("Face {}", fid);
                        if let Some(n) = self.selected_face_normal {
                            info += &format!("  N:({:.2},{:.2},{:.2})", n[0], n[1], n[2]);
                        }
                        if let Some(a) = self.selected_face_area {
                            info += &format!("  A:{:.4}", a);
                        }
                        ui.label(egui::RichText::new(info).size(10.0).color(egui::Color32::from_rgb(100, 160, 255)));
                    }

                    // Measurement display
                    if let Some(ref m) = self.active_measurement {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Dist: {:.4} m", m.distance))
                            .size(10.0).strong().color(egui::Color32::from_rgb(255, 200, 50)));
                    } else if self.measure_first_point.is_some() {
                        ui.separator();
                        ui.label(egui::RichText::new("Click second point...")
                            .size(10.0).color(egui::Color32::from_rgb(255, 200, 50)));
                    }

                    // Edge selection info
                    if self.selection_mode == SelectionMode::Edge {
                        ui.separator();
                        ui.label(egui::RichText::new("[Edge]").size(10.0).color(egui::Color32::from_rgb(200, 150, 255)));
                    }

                    // Cursor 3D position
                    if let Some(pos) = self.cursor_world {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("({:.3}, {:.3}, {:.3})", pos[0], pos[1], pos[2])).weak().size(10.0));
                        });
                    }
                });
            });
    }

    fn draw_context_menu(&mut self, ctx: &egui::Context) {
        let Some(pos) = self.context_menu_pos else { return };
        let Some(face_id) = self.context_menu_face else {
            self.context_menu_pos = None;
            return;
        };

        let area = egui::Area::new(egui::Id::new("face_context_menu"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .interactable(true);

        let resp = area.show(ctx, |ui| {
            let frame = egui::Frame::popup(ui.style());
            frame.show(ui, |ui| {
                ui.set_min_width(130.0);
                ui.label(egui::RichText::new(format!("Face {}", face_id)).strong().size(11.0));
                ui.separator();

                if ui.button(egui::RichText::new("Extrude").size(11.0)).clicked() {
                    self.active_tool = Tool::Extrude;
                    self.context_menu_action = Some(ContextAction::Extrude);
                    self.context_menu_pos = None;
                }
                if ui.button(egui::RichText::new("Cut").size(11.0)).clicked() {
                    self.active_tool = Tool::Cut;
                    self.context_menu_action = Some(ContextAction::Cut);
                    self.context_menu_pos = None;
                }
                if ui.button(egui::RichText::new("Inset").size(11.0)).clicked() {
                    self.active_tool = Tool::Inset;
                    self.context_menu_action = Some(ContextAction::Inset);
                    self.context_menu_pos = None;
                }
                ui.separator();
                if ui.button(egui::RichText::new("Delete").size(11.0).color(egui::Color32::from_rgb(230, 80, 60))).clicked() {
                    self.context_menu_action = Some(ContextAction::Delete);
                    self.context_menu_pos = None;
                }
                ui.separator();
                if ui.button(egui::RichText::new("Zoom to Face").size(11.0)).clicked() {
                    self.context_menu_action = Some(ContextAction::ZoomToFace);
                    self.context_menu_pos = None;
                }
            });
        });

        // Close on click outside
        if ctx.input(|i| i.pointer.any_pressed()) && !resp.response.hovered() {
            self.context_menu_pos = None;
        }
    }

    fn draw_dimension_label(&self, ctx: &egui::Context) {
        if !self.preview_active { return; }

        let (label, color) = match self.active_tool {
            Tool::Extrude => {
                let d = self.extrude_distance;
                (format!("{:.3} m", d), egui::Color32::from_rgb(100, 160, 255))
            }
            Tool::Cut => {
                let d = self.cut_depth;
                (format!("{:.3} m", d), egui::Color32::from_rgb(230, 100, 80))
            }
            Tool::Inset => {
                let d = self.inset_amount;
                (format!("{:.3} m", d), egui::Color32::from_rgb(100, 200, 200))
            }
            _ => return,
        };

        // Show floating dimension label near center-right of viewport
        let screen = ctx.screen_rect();
        let pos = egui::pos2(screen.center().x + 60.0, screen.center().y - 30.0);

        egui::Area::new(egui::Id::new("dimension_label"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 200))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .rounding(3.0)
                    .stroke(egui::Stroke::new(1.0, color));
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(label)
                        .size(16.0)
                        .strong()
                        .color(color));
                });
            });
    }

    fn draw_sketch_dimensions(&self, ctx: &egui::Context) {
        let Some(length) = self.sketch_preview_length else { return };
        if length < 0.001 { return; }

        let mut text = format!("{:.3} m", length);
        if let Some(angle) = self.sketch_preview_angle {
            text += &format!("  {:.1}\u{00B0}", angle);
        }

        // Show near cursor
        let screen = ctx.screen_rect();
        let pos = egui::pos2(screen.center().x + 40.0, screen.center().y + 20.0);

        egui::Area::new(egui::Id::new("sketch_dim_label"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 25, 20, 180))
                    .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                    .rounding(3.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 100)));
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(text)
                        .size(13.0)
                        .color(egui::Color32::from_rgb(150, 230, 150)));
                });
            });
    }

    fn draw_toasts(&mut self, ctx: &egui::Context) {
        self.toasts.retain(|t| !t.is_expired());

        let screen = ctx.screen_rect();
        let mut y = screen.max.y - 40.0;

        for toast in self.toasts.iter().rev().take(3) {
            let alpha = (toast.alpha() * 255.0) as u8;
            let area = egui::Area::new(egui::Id::new(toast.created))
                .fixed_pos(egui::pos2(screen.max.x - 280.0, y))
                .order(egui::Order::Foreground);

            area.show(ctx, |ui| {
                let frame = egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 35, alpha))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 80, 90, alpha)));
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(&toast.text)
                        .size(11.0)
                        .color(egui::Color32::from_rgba_unmultiplied(220, 220, 220, alpha)));
                });
            });
            y -= 30.0;
        }
    }
}
