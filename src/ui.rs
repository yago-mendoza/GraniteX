use egui;

#[derive(PartialEq, Clone, Copy)]
pub enum Tool {
    Select,
    Extrude,
    Cut,
    Inset,
    Fillet,
    Line,
    Rect,
    Circle,
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
    pub chat_input: String,
    pub chat_history: Vec<ChatMessage>,
    pub selected_feature: Option<String>,
    pub mesh_faces: u32,
    pub mesh_verts: usize,
    pub mesh_tris: usize,
    pub sketch_entity_count: usize,
    pub wireframe_supported: bool,
}

pub struct ChatMessage {
    pub sender: Sender,
    pub text: String,
}

pub enum Sender {
    User,
    Agent,
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
            chat_input: String::new(),
            chat_history: vec![
                ChatMessage {
                    sender: Sender::Agent,
                    text: "Welcome to GraniteX. Describe what you want to build.".into(),
                },
            ],
            selected_feature: None,
            mesh_faces: 6,
            mesh_verts: 24,
            mesh_tris: 12,
            sketch_entity_count: 0,
            wireframe_supported: false,
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        self.draw_top_toolbar(ctx);
        self.draw_bottom_bar(ctx);
        self.draw_left_panel(ctx);
        if self.show_chat {
            self.draw_chat_panel(ctx);
        }
    }

    fn draw_top_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .exact_height(30.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;

                    ui.label(egui::RichText::new("GraniteX").strong().size(13.0));
                    ui.separator();

                    if ui.add(egui::Button::new(egui::RichText::new("Import").size(11.0))).clicked() {
                        self.import_request = true;
                    }

                    ui.separator();

                    // Operations
                    let ops = [
                        (Tool::Select,  "Select"),
                        (Tool::Extrude, "Extrude"),
                        (Tool::Cut,     "Cut"),
                        (Tool::Inset,   "Inset"),
                        (Tool::Fillet,  "Fillet"),
                    ];
                    for (tool, label) in &ops {
                        let btn = egui::Button::new(egui::RichText::new(*label).size(11.0))
                            .selected(self.active_tool == *tool);
                        if ui.add(btn).clicked() {
                            self.active_tool = if self.active_tool == *tool { Tool::Select } else { *tool };
                        }
                    }

                    ui.separator();

                    // Drawing tools
                    ui.label(egui::RichText::new("Draw:").weak().size(10.0));
                    let draw_tools = [
                        (Tool::Line,   "Line"),
                        (Tool::Rect,   "Rect"),
                        (Tool::Circle, "Circle"),
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
                        for plane in ["XY Plane", "XZ Plane", "YZ Plane"] {
                            let is_sel = self.selected_feature.as_deref() == Some(plane);
                            let label = if is_sel {
                                egui::RichText::new(format!("  {}", plane)).size(11.0).strong().color(egui::Color32::from_rgb(100, 160, 255))
                            } else {
                                egui::RichText::new(format!("  {}", plane)).size(11.0)
                            };
                            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                                self.selected_feature = Some(plane.to_string());
                            }
                        }
                    });

                ui.separator();

                egui::CollapsingHeader::new(egui::RichText::new("Bodies").size(11.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        let is_sel = self.selected_feature.as_deref() == Some("Cube");
                        let label = if is_sel {
                            egui::RichText::new("  Cube").size(11.0).strong().color(egui::Color32::from_rgb(100, 160, 255))
                        } else {
                            egui::RichText::new("  Cube").size(11.0)
                        };
                        if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                            self.selected_feature = Some("Cube".to_string());
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
                    }
                    Tool::Line | Tool::Rect | Tool::Circle => {
                        let name = match self.active_tool {
                            Tool::Line => "Line",
                            Tool::Rect => "Rectangle",
                            Tool::Circle => "Circle",
                            _ => "",
                        };
                        ui.label(egui::RichText::new(name).strong().size(11.0).color(egui::Color32::from_rgb(100, 200, 100)));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Click on a face to draw").weak().size(9.0));
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
                    ui.label(egui::RichText::new("Ready").weak().size(10.0));
                    ui.separator();
                    ui.label(egui::RichText::new(format!("F:{} V:{} T:{}", self.mesh_faces, self.mesh_verts, self.mesh_tris)).weak().size(10.0));
                    if let Some(ref feat) = self.selected_feature {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Selected: {}", feat)).size(10.0).color(egui::Color32::from_rgb(100, 160, 255)));
                    }
                });
            });
    }
}
