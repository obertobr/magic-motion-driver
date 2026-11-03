//! egui/eframe GUI control panel with live interactive remote feedback.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use eframe::egui::{self, Color32, Panel, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::domain::action::{Action, KeyShortcut, MediaKey, VirtualKey};
use crate::domain::button::Button;
use crate::domain::config::Config;
use crate::infra::autostart::{is_autostart_enabled, set_autostart};
use crate::infra::driver::{DriverCommand, DriverTelemetry};
use crate::presentation::hotspots::{find_hotspot_at, BUTTON_HOTSPOTS, REMOTE_IMG_HEIGHT, REMOTE_IMG_WIDTH};
use crate::presentation::tray::{SystemTray, TrayAction};

#[derive(Clone, Debug)]
struct SensorRecordSample {
    sequence: u8,
    time_s: f64,
    rate_hz: f32,
    gyro_x: u16,
    gyro_y: u16,
    gyro_dx: i32,
    gyro_dy: i32,
    accel_x: u16,
    accel_y: u16,
    accel_z: u16,
    g_x: f32,
    g_y: f32,
    g_z: f32,
    roll_deg: f32,
    yaw_deg: f32,
    pitch_deg: f32,
    button_raw: u8,
    buttons_active: String,
}

pub struct DriverApp {
    config: Config,
    driver_cmd_tx: Sender<DriverCommand>,
    telem_rx: Option<Receiver<DriverTelemetry>>,
    latest_telemetry: DriverTelemetry,
    selected_button: Button,
    texture: Option<egui::TextureHandle>,
    tray: Option<SystemTray>,
    save_status: Option<(String, std::time::Instant)>,
    calibration_feedback: Option<(String, std::time::Instant)>,
    recording_shortcut: bool,
    active_tab: Tab,
    pub should_exit: bool,
    gyro_history_x: VecDeque<f32>,
    gyro_history_y: VecDeque<f32>,
    accel_history_x: VecDeque<f32>,
    accel_history_y: VecDeque<f32>,
    accel_history_z: VecDeque<f32>,
    osc_gyro_x: bool,
    osc_gyro_y: bool,
    osc_accel_x: bool,
    osc_accel_y: bool,
    osc_accel_z: bool,
    remote_yaw: f32,
    remote_pitch: f32,
    remote_roll: f32,
    attitude: crate::domain::attitude::AttitudeEstimator,
    mesh_3d: crate::presentation::mesh3d::Mesh3D,
    is_recording_sensors: bool,
    recording_start_instant: Option<std::time::Instant>,
    recorded_samples: Vec<SensorRecordSample>,
    recording_status_msg: Option<(String, std::time::Instant)>,
}

impl std::fmt::Debug for DriverApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverApp")
            .field("config", &self.config)
            .field("selected_button", &self.selected_button)
            .field("should_exit", &self.should_exit)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Buttons,
    Motion,
    Settings,
    Sensors,
}

impl DriverApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        driver_cmd_tx: Sender<DriverCommand>,
        telem_rx: Option<Receiver<DriverTelemetry>>,
    ) -> Self {
        let tray = SystemTray::new().ok();

        Self {
            config,
            driver_cmd_tx,
            telem_rx,
            latest_telemetry: DriverTelemetry::default(),
            selected_button: Button::Ok,
            texture: None,
            tray,
            save_status: None,
            calibration_feedback: None,
            recording_shortcut: false,
            active_tab: Tab::Buttons,
            should_exit: false,
            gyro_history_x: VecDeque::with_capacity(120),
            gyro_history_y: VecDeque::with_capacity(120),
            accel_history_x: VecDeque::with_capacity(120),
            accel_history_y: VecDeque::with_capacity(120),
            accel_history_z: VecDeque::with_capacity(120),
            osc_gyro_x: true,
            osc_gyro_y: true,
            osc_accel_x: true,
            osc_accel_y: true,
            osc_accel_z: true,
            remote_yaw: 0.0,
            remote_pitch: 0.0,
            remote_roll: 0.0,
            attitude: crate::domain::attitude::AttitudeEstimator::default(),
            mesh_3d: crate::presentation::mesh3d::Mesh3D::load_or_default(),
            is_recording_sensors: false,
            recording_start_instant: None,
            recorded_samples: Vec::new(),
            recording_status_msg: None,
        }
    }

    pub fn poll_telemetry(&mut self) {
        let mut batch = Vec::new();
        if let Some(ref rx) = self.telem_rx {
            while let Ok(telem) = rx.try_recv() {
                batch.push(telem);
            }
        }
        for telem in batch {
            self.update_telemetry(telem);
        }
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.texture.is_none() {
            let img_bytes = include_bytes!("../../assets/remote.png");
            if let Ok(dynamic_img) = image::load_from_memory(img_bytes) {
                let rgba = dynamic_img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.texture = Some(ctx.load_texture("remote_body", color_image, Default::default()));
            }
        }
    }

    pub fn update_telemetry(&mut self, telem: DriverTelemetry) {
        // Auto-select physically pressed button in UI
        for b in telem.active_buttons.iter() {
            if b.is_mappable() {
                self.selected_button = b;
            }
        }

        const MAX_HISTORY: usize = 100;
        self.gyro_history_x.push_back(telem.gyro_x as f32);
        if self.gyro_history_x.len() > MAX_HISTORY {
            self.gyro_history_x.pop_front();
        }
        self.gyro_history_y.push_back(telem.gyro_y as f32);
        if self.gyro_history_y.len() > MAX_HISTORY {
            self.gyro_history_y.pop_front();
        }
        self.accel_history_x.push_back(telem.accel_x as f32);
        if self.accel_history_x.len() > MAX_HISTORY {
            self.accel_history_x.pop_front();
        }
        self.accel_history_y.push_back(telem.accel_y as f32);
        if self.accel_history_y.len() > MAX_HISTORY {
            self.accel_history_y.pop_front();
        }
        self.accel_history_z.push_back(telem.accel_z as f32);
        if self.accel_history_z.len() > MAX_HISTORY {
            self.accel_history_z.pop_front();
        }

        // 3D Attitude Estimation via DCM (no Euler gimbal lock / no axis flipping / anti-jitter)
        self.attitude.update(
            telem.gyro_x,
            telem.gyro_y,
            telem.center_x,
            telem.center_y,
            [telem.accel_x, telem.accel_y, telem.accel_z],
            0.010,
        );

        let (yaw, pitch, roll) = self.attitude.angles_deg();
        self.remote_yaw = yaw;
        self.remote_pitch = pitch;
        self.remote_roll = roll;

        if self.is_recording_sensors {
            let start = *self.recording_start_instant.get_or_insert(telem.timestamp);
            let time_s = telem.timestamp.duration_since(start).as_secs_f64();
            let [g_x, g_y, g_z] = crate::domain::gravity::AccelCalibration::LG_AN_MR200
                .to_g([telem.accel_x, telem.accel_y, telem.accel_z]);

            let active_btns: Vec<String> = telem.active_buttons.iter().map(|b| format!("{b:?}")).collect();
            let buttons_str = active_btns.join(";");

            if self.recorded_samples.len() < 200_000 {
                self.recorded_samples.push(SensorRecordSample {
                    sequence: telem.sequence,
                    time_s,
                    rate_hz: telem.rate_hz,
                    gyro_x: telem.gyro_x,
                    gyro_y: telem.gyro_y,
                    gyro_dx: telem.gyro_dx,
                    gyro_dy: telem.gyro_dy,
                    accel_x: telem.accel_x,
                    accel_y: telem.accel_y,
                    accel_z: telem.accel_z,
                    g_x,
                    g_y,
                    g_z,
                    roll_deg: telem.roll_deg,
                    yaw_deg: yaw,
                    pitch_deg: pitch,
                    button_raw: telem.button_raw,
                    buttons_active: buttons_str,
                });
            }
        }

        self.latest_telemetry = telem;
    }

    fn start_recording_sensors(&mut self) {
        self.is_recording_sensors = true;
        self.recording_start_instant = None;
        self.recorded_samples.clear();
        self.recording_status_msg = Some(("Gravação iniciada...".to_string(), std::time::Instant::now()));
    }

    fn stop_recording_sensors(&mut self) {
        self.is_recording_sensors = false;
        let count = self.recorded_samples.len();
        if count == 0 {
            self.recording_status_msg = Some(("Nenhuma amostra gravada.".to_string(), std::time::Instant::now()));
            return;
        }

        let now = std::time::SystemTime::now();
        let secs = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let ts_str = format_timestamp(secs);
        let filename = format!("sensores_{ts_str}.csv");

        let mut csv = String::with_capacity(count * 90 + 200);
        csv.push_str("seq,time_s,rate_hz,gyro_x,gyro_y,gyro_dx,gyro_dy,accel_x,accel_y,accel_z,accel_g_x,accel_g_y,accel_g_z,roll_deg,yaw_deg,pitch_deg,buttons_hex,buttons_active\n");
        for s in &self.recorded_samples {
            use std::fmt::Write;
            let _ = writeln!(
                csv,
                "{},{:.4},{:.1},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.1},{:.1},{:.1},0x{:02X},\"{}\"",
                s.sequence,
                s.time_s,
                s.rate_hz,
                s.gyro_x,
                s.gyro_y,
                s.gyro_dx,
                s.gyro_dy,
                s.accel_x,
                s.accel_y,
                s.accel_z,
                s.g_x,
                s.g_y,
                s.g_z,
                s.roll_deg,
                s.yaw_deg,
                s.pitch_deg,
                s.button_raw,
                s.buttons_active
            );
        }

        match std::fs::write(&filename, csv.as_bytes()) {
            Ok(()) => {
                self.recording_status_msg = Some((
                    format!("Salvo: {filename} ({count} amostras)"),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                self.recording_status_msg = Some((
                    format!("Erro ao salvar: {e}"),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.save_status = Some((format!("Erro ao salvar: {e}"), std::time::Instant::now()));
        } else {
            let _ = self.driver_cmd_tx.send(DriverCommand::UpdateConfig(self.config.clone()));
            self.save_status = Some(("Configurações salvas!".to_string(), std::time::Instant::now()));
        }
    }

    pub fn update_background(&mut self, ctx: &egui::Context) {
        if self.should_exit {
            let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
            std::process::exit(0);
        }

        self.poll_telemetry();

        // Process System Tray events & live connection status
        if let Some(ref mut tray) = self.tray {
            tray.update_status(self.latest_telemetry.connected, &self.latest_telemetry.port_name);
            match tray.process_events(&self.driver_cmd_tx) {
                Some(TrayAction::OpenPanel) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Some(TrayAction::Quit) => {
                    self.should_exit = true;
                    let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
                    std::process::exit(0);
                }
                None => {}
            }
        }

        // Intercept close button to hide to tray (unless user clicked Sair)
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.should_exit {
                let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
                std::process::exit(0);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        if self.active_tab == Tab::Sensors || self.is_recording_sensors {
            ctx.request_repaint();
        } else if self.latest_telemetry.connected || !self.latest_telemetry.active_buttons.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.ensure_texture(&ctx);

        // Top Header Panel
        Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("LG Magic Motion Painel");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Lateral direita superior: bolinha de status (verde conectado / vermelha desconectado)
                    let dot_color = if self.latest_telemetry.connected {
                        Color32::from_rgb(40, 220, 100)
                    } else {
                        Color32::from_rgb(230, 60, 60)
                    };
                    let (dot_rect, dot_response) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 5.5, dot_color);
                    ui.painter().circle_stroke(
                        dot_rect.center(),
                        5.5,
                        Stroke::new(1.0, if self.latest_telemetry.connected { Color32::from_rgb(20, 160, 60) } else { Color32::from_rgb(160, 30, 30) }),
                    );

                    let tooltip = if self.latest_telemetry.connected {
                        format!("Conectado ({}) - {:.0} Hz", self.latest_telemetry.port_name, self.latest_telemetry.rate_hz)
                    } else {
                        "Desconectado (Aguardando receptor CP2102...)".to_string()
                    };
                    dot_response.on_hover_text(tooltip);

                    ui.add_space(10.0);

                    let (cursor_txt, cursor_color) = if self.latest_telemetry.cursor_enabled {
                        ("Mira: ATIVA", Color32::from_rgb(40, 200, 100))
                    } else {
                        ("Mira: PAUSADA", Color32::from_rgb(220, 100, 80))
                    };
                    if ui.button(egui::RichText::new(cursor_txt).color(cursor_color).strong()).clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::ToggleCursor);
                    }
                });
            });
            ui.add_space(6.0);
        });

        // Bottom Footer Panel
        Panel::bottom("footer").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Salvar Configurações").clicked() {
                    self.save_config();
                }

                if let Some((ref msg, time)) = self.save_status {
                    if time.elapsed() < std::time::Duration::from_secs(4) {
                        ui.label(egui::RichText::new(msg).color(Color32::from_rgb(100, 220, 140)));
                    }
                }
            });
            ui.add_space(4.0);
        });

        // Central Area
        let central_frame = egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 8,
            bottom: 0,
        });
        egui::CentralPanel::default().frame(central_frame).show(ui, |ui| {
            let total_avail_h = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_min_height(total_avail_h);
                ui.set_max_height(total_avail_h);

                // Left: Remote Image with interactive hotspots
                ui.vertical(|ui| {
                    ui.set_width(200.0);
                    ui.set_max_height(total_avail_h);
                    ui.vertical_centered(|ui| {
                        if let Some(ref tex) = self.texture {
                            let display_h = (total_avail_h - 8.0).min(560.0);
                            let display_w = display_h * (REMOTE_IMG_WIDTH / REMOTE_IMG_HEIGHT);
                            let avail_w = ui.available_width();
                            let offset_x = ((avail_w - display_w) / 2.0).max(0.0);

                            let (full_rect, resp) = ui.allocate_exact_size(
                                Vec2::new(avail_w, display_h),
                                egui::Sense::click_and_drag(),
                            );

                            let img_rect = Rect::from_min_size(
                                Pos2::new(full_rect.min.x + offset_x, full_rect.min.y),
                                Vec2::new(display_w, display_h),
                            );

                            let painter = ui.painter_at(full_rect);
                            painter.image(
                                tex.id(),
                                img_rect,
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );

                            let scale_x = display_w / REMOTE_IMG_WIDTH;
                            let scale_y = display_h / REMOTE_IMG_HEIGHT;

                            let hover_pos = resp.hover_pos();
                            let hovered_button = hover_pos.and_then(|pos| {
                                if img_rect.contains(pos) {
                                    let norm_x = (pos.x - img_rect.min.x) / display_w;
                                    let norm_y = (pos.y - img_rect.min.y) / display_h;
                                    find_hotspot_at(norm_x, norm_y)
                                } else {
                                    None
                                }
                            });

                            if hovered_button.is_some() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if resp.clicked() {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    if img_rect.contains(pos) {
                                        let norm_x = (pos.x - img_rect.min.x) / display_w;
                                        let norm_y = (pos.y - img_rect.min.y) / display_h;
                                        if let Some(btn) = find_hotspot_at(norm_x, norm_y) {
                                            if btn.is_mappable() {
                                                self.selected_button = btn;
                                                self.active_tab = Tab::Buttons;
                                            }
                                        }
                                    }
                                }
                            }

                            for spot in &BUTTON_HOTSPOTS {
                                let center = Pos2::new(
                                    img_rect.min.x + spot.cx * scale_x,
                                    img_rect.min.y + spot.cy * scale_y,
                                );
                                let radius = spot.radius * scale_x;

                                let is_active = self.latest_telemetry.active_buttons.contains(spot.button);
                                let is_selected = self.selected_button == spot.button;
                                let is_hovered = hovered_button == Some(spot.button);

                                if is_active {
                                    // Live animated pulse when pressed physically on the remote!
                                    painter.circle_filled(
                                        center,
                                        radius + 6.0,
                                        Color32::from_rgba_unmultiplied(0, 240, 180, 160),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 8.0,
                                        Stroke::new(2.5, Color32::from_rgb(0, 255, 200)),
                                    );
                                } else if is_hovered {
                                    // Contorno amarelo ao passar o mouse em cima do botão
                                    painter.circle_filled(
                                        center,
                                        radius + 4.0,
                                        Color32::from_rgba_unmultiplied(250, 215, 20, 50),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 4.0,
                                        Stroke::new(2.5, Color32::from_rgb(250, 215, 20)),
                                    );
                                } else if is_selected {
                                    // Destaque do botão selecionado atualmente para configuração
                                    painter.circle_filled(
                                        center,
                                        radius + 4.0,
                                        Color32::from_rgba_unmultiplied(60, 160, 255, 45),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 4.0,
                                        Stroke::new(2.0, Color32::from_rgb(80, 180, 255)),
                                    );
                                }
                            }
                        }
                    });
                });

                ui.separator();

                // Right: Tabs
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(total_avail_h);
                    ui.set_max_height(total_avail_h);

                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Buttons, "Botões");
                        ui.selectable_value(&mut self.active_tab, Tab::Motion, "Movimento");
                        ui.selectable_value(&mut self.active_tab, Tab::Settings, "Configurações");
                        ui.selectable_value(&mut self.active_tab, Tab::Sensors, "Dados Brutos");
                    });
                    ui.separator();

                    match self.active_tab {
                        Tab::Buttons => self.render_buttons_tab(ui),
                        Tab::Motion => self.render_motion_tab(ui),
                        Tab::Settings => self.render_settings_tab(ui),
                        Tab::Sensors => self.render_sensors_tab(ui),
                    }
                });
            });
        });
    }
}

fn render_menu_option<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    current: &mut T,
    val: T,
    label: &str,
) -> bool {
    let row_height = 24.0;
    let row_size = egui::vec2(ui.available_width(), row_height);
    let (rect, resp) = ui.allocate_exact_size(row_size, egui::Sense::click());

    let is_selected = *current == val;
    let clicked = resp.clicked();
    if clicked && !is_selected {
        *current = val;
    }

    let painter = ui.painter().with_clip_rect(rect);

    let bg_color = if is_selected {
        ui.visuals().selection.bg_fill
    } else if resp.hovered() {
        Color32::from_rgb(52, 56, 64)
    } else {
        Color32::from_rgb(36, 38, 43)
    };

    painter.rect_filled(rect, 4.0, bg_color);

    let text_color = if is_selected || resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_gray(215)
    };

    painter.text(
        egui::pos2(rect.min.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        text_color,
    );

    ui.add_space(2.0);

    clicked
}

impl DriverApp {
    fn render_buttons_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurar Ação do Botão");
        ui.add_space(4.0);

        let total_w = ui.available_width();
        let avail_h = ui.available_height();
        let left_w = (total_w * 0.46).clamp(210.0, 260.0);

        ui.horizontal(|ui| {
            ui.set_min_height(avail_h);
            ui.set_max_height(avail_h);

            // Esquerda: Mapeamento dos botões (linha inteira selecionável)
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.set_max_height(avail_h);

                let header_h = 22.0;
                let (header_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), header_h),
                    egui::Sense::hover(),
                );
                let hp = ui.painter().with_clip_rect(header_rect);
                hp.text(
                    egui::pos2(header_rect.min.x + 8.0, header_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "Botão",
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
                hp.text(
                    egui::pos2(header_rect.min.x + 95.0, header_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "Ação Atual",
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );

                ui.separator();

                let btn_h = ui.spacing().interact_size.y + ui.spacing().button_padding.y * 2.0;
                let spacing_y = ui.spacing().item_spacing.y;
                let table_max_h = (ui.available_height() - btn_h - spacing_y - 2.0).max(100.0);
                egui::ScrollArea::vertical()
                    .id_salt("buttons_mapping_scroll")
                    .auto_shrink([false; 2])
                    .max_height(table_max_h)
                    .show(ui, |ui| {
                        for (idx, b) in Button::mappable().enumerate() {
                            let row_h = 24.0;
                            let row_size = egui::vec2(ui.available_width(), row_h);
                            let (row_rect, row_resp) = ui.allocate_exact_size(row_size, egui::Sense::click());

                            if row_resp.clicked() {
                                self.selected_button = b;
                                self.recording_shortcut = false;
                            }

                            let is_selected = self.selected_button == b;
                            let painter = ui.painter().with_clip_rect(row_rect);

                            if is_selected {
                                painter.rect_filled(row_rect, 3.0, ui.visuals().selection.bg_fill);
                            } else if row_resp.hovered() {
                                painter.rect_filled(row_rect, 3.0, ui.visuals().faint_bg_color);
                            } else if idx % 2 == 1 {
                                painter.rect_filled(
                                    row_rect,
                                    3.0,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 4),
                                );
                            }

                            let (name_color, action_color) = if is_selected {
                                (Color32::WHITE, Color32::WHITE)
                            } else {
                                (ui.visuals().text_color(), Color32::from_gray(180))
                            };

                            let action = self.config.get_action(b);

                            painter.text(
                                egui::pos2(row_rect.min.x + 8.0, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                format!("{b:?}"),
                                egui::FontId::proportional(13.0),
                                name_color,
                            );

                            painter.text(
                                egui::pos2(row_rect.min.x + 95.0, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                action.label(),
                                egui::FontId::proportional(13.0),
                                action_color,
                            );
                        }
                    });

                if ui.button("Restaurar Botões Padrão").clicked() {
                    self.config.button_mappings = Config::default().button_mappings;
                    self.save_config();
                }
            });

            ui.separator();

            // Direita: Menu de seleção da ação
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.set_max_height(avail_h);
                let mut new_action = self.config.get_action(self.selected_button);

                // Opção separada: Gravar Tecla ou Atalho Personalizado (mesmo design de card cinza)
                let is_shortcut_active = matches!(new_action, Action::Shortcut(_));
                let shortcut_row_h = 26.0;
                let shortcut_row_size = egui::vec2(ui.available_width(), shortcut_row_h);

                if self.recording_shortcut {
                    let mut captured = None;
                    ui.input(|i| {
                        for ev in &i.events {
                            if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = ev {
                                if let Some(vk) = egui_key_to_vk(*key) {
                                    captured = Some(KeyShortcut::with_modifiers(
                                        vk,
                                        modifiers.ctrl || modifiers.command,
                                        modifiers.shift,
                                        modifiers.alt,
                                        modifiers.mac_cmd,
                                    ));
                                    break;
                                }
                            }
                        }
                    });

                    if let Some(shortcut) = captured {
                        new_action = Action::Shortcut(shortcut);
                        self.recording_shortcut = false;
                    }

                    let (rect, resp) = ui.allocate_exact_size(shortcut_row_size, egui::Sense::click());
                    if resp.clicked() {
                        self.recording_shortcut = false;
                    }

                    let painter = ui.painter().with_clip_rect(rect);
                    painter.rect_filled(rect, 4.0, Color32::from_rgb(48, 44, 30));
                    painter.rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.0, Color32::from_rgb(230, 180, 50)),
                        egui::StrokeKind::Inside,
                    );

                    painter.text(
                        egui::pos2(rect.min.x + 10.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        "⌨ Pressione a tecla ou atalho... (Clique para cancelar)",
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(255, 215, 70),
                    );
                } else {
                    let (rect, resp) = ui.allocate_exact_size(shortcut_row_size, egui::Sense::click());
                    if resp.clicked() {
                        self.recording_shortcut = true;
                    }

                    let painter = ui.painter().with_clip_rect(rect);

                    let bg_color = if is_shortcut_active {
                        ui.visuals().selection.bg_fill
                    } else if resp.hovered() {
                        Color32::from_rgb(52, 56, 64)
                    } else {
                        Color32::from_rgb(36, 38, 43)
                    };

                    painter.rect_filled(rect, 4.0, bg_color);

                    let label_text = if let Action::Shortcut(s) = new_action {
                        format!("⌨ Atalho Personalizado: {}  (Clique para alterar)", s.label())
                    } else {
                        "⌨ Gravar Tecla ou Atalho Personalizado...".to_string()
                    };

                    let text_color = if is_shortcut_active || resp.hovered() {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(215)
                    };

                    painter.text(
                        egui::pos2(rect.min.x + 10.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label_text,
                        egui::FontId::proportional(13.0),
                        text_color,
                    );
                }
                ui.add_space(6.0);

                // Indicativos dinâmicos para cada categoria mesmo se fechada
                let mouse_has_action = matches!(
                    new_action,
                    Action::MouseLeft
                        | Action::MouseRight
                        | Action::MouseMiddle
                        | Action::MouseWheelUp
                        | Action::MouseWheelDown
                );
                let mouse_title = if mouse_has_action {
                    egui::RichText::new("Mouse")
                        .strong()
                        .color(Color32::from_rgb(100, 190, 255))
                } else {
                    egui::RichText::new("Mouse").strong()
                };

                let keyboard_has_action = matches!(new_action, Action::Key(_));
                let keyboard_title = if keyboard_has_action {
                    egui::RichText::new("Teclado")
                        .strong()
                        .color(Color32::from_rgb(100, 190, 255))
                } else {
                    egui::RichText::new("Teclado").strong()
                };

                let media_has_action = matches!(new_action, Action::Media(_));
                let media_title = if media_has_action {
                    egui::RichText::new("Mídia")
                        .strong()
                        .color(Color32::from_rgb(100, 190, 255))
                } else {
                    egui::RichText::new("Mídia").strong()
                };

                let system_has_action = matches!(
                    new_action,
                    Action::ToggleCursor | Action::Recenter | Action::None
                );
                let system_title = if system_has_action {
                    egui::RichText::new("Sistema")
                        .strong()
                        .color(Color32::from_rgb(100, 190, 255))
                } else {
                    egui::RichText::new("Sistema").strong()
                };

                let actions_max_h = ui.available_height().max(100.0);
                egui::ScrollArea::vertical()
                    .id_salt("actions_collapsing_scroll")
                    .auto_shrink([false; 2])
                    .max_height(actions_max_h)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        egui::CollapsingHeader::new(mouse_title)
                            .id_salt("cat_mouse_v2")
                            .default_open(false)
                            .show(ui, |ui| {
                                render_menu_option(ui, &mut new_action, Action::MouseLeft, "Clique Esquerdo");
                                render_menu_option(ui, &mut new_action, Action::MouseRight, "Clique Direito");
                                render_menu_option(ui, &mut new_action, Action::MouseMiddle, "Clique do Meio");
                                render_menu_option(ui, &mut new_action, Action::MouseWheelUp, "Rolar para Cima");
                                render_menu_option(ui, &mut new_action, Action::MouseWheelDown, "Rolar para Baixo");
                                ui.add_space(4.0);
                            });

                        egui::CollapsingHeader::new(keyboard_title)
                            .id_salt("cat_keyboard_v2")
                            .default_open(false)
                            .show(ui, |ui| {
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::LeftWin), "Tecla Windows (Win)");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Return), "Enter");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Escape), "Esc");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Space), "Espaço");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Backspace), "Backspace");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Tab), "Tab");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Up), "Seta Cima");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Down), "Seta Baixo");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Left), "Seta Esquerda");
                                render_menu_option(ui, &mut new_action, Action::Key(VirtualKey::Right), "Seta Direita");
                                ui.add_space(4.0);
                            });

                        egui::CollapsingHeader::new(media_title)
                            .id_salt("cat_media_v2")
                            .default_open(false)
                            .show(ui, |ui| {
                                for mk in MediaKey::ALL {
                                    render_menu_option(ui, &mut new_action, Action::Media(mk), mk.label());
                                }
                                ui.add_space(4.0);
                            });

                        egui::CollapsingHeader::new(system_title)
                            .id_salt("cat_system_v2")
                            .default_open(false)
                            .show(ui, |ui| {
                                render_menu_option(ui, &mut new_action, Action::ToggleCursor, "Ativar / Pausar Mira");
                                render_menu_option(ui, &mut new_action, Action::Recenter, "Recalibrar Centro");
                                render_menu_option(ui, &mut new_action, Action::None, "Desativar Botão");
                                ui.add_space(4.0);
                            });
                    });

                if new_action != self.config.get_action(self.selected_button) {
                    self.config.set_action(self.selected_button, new_action);
                    self.save_config();
                }
            });
        });
    }

    fn render_motion_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ajustes de Movimento e Sensibilidade");
        ui.add_space(4.0);

        let mut changed = false;

        let avail_h = ui.available_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(avail_h)
            .show(ui, |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Sensibilidade dos Eixos:").strong());
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity, 0.05..=1.5).text("Sensibilidade Geral")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_x, 0.2..=2.5).text("Multiplicador X (Yaw / Horizontal)")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_y, 0.2..=2.5).text("Multiplicador Y (Pitch / Vertical)")).changed();
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Precisão & Filtros:").strong());
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.deadzone, 0.0..=6.0).text("Zona Morta (Deadzone)").suffix(" pts")).changed();
                ui.label(egui::RichText::new("Elimina micro-tremores e ruído do sensor com a mão parada.").small().color(Color32::GRAY));

                ui.add_space(4.0);
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.acceleration, 1.0..=1.8).text("Curva de Aceleração")).changed();
                ui.label(egui::RichText::new("1.0 = Movimento linear (1:1). Acima de 1.1 dá mais agilidade a movimentos rápidos de pulso.").small().color(Color32::GRAY));

                ui.add_space(4.0);
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.smoothing, 0.0..=0.75).text("Suavização (Low-Pass Filter)")).changed();
                ui.label(egui::RichText::new("Filtro exponencial que suaviza os passos do mouse sem adicionar atraso perceptível.").small().color(Color32::GRAY));
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Inversão de Eixos:").strong());
                changed |= ui.checkbox(&mut self.config.motion.invert_x, "Inverter Eixo X (Horizontal)").changed();
                changed |= ui.checkbox(&mut self.config.motion.invert_y, "Inverter Eixo Y (Vertical)").changed();
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Orientação & Compensação de Rotação:").strong());
                changed |= ui.checkbox(
                    &mut self.config.motion.roll_compensation,
                    "Compensar Rotação do Controle (Roll / Gravidade)",
                ).changed();
                ui.label(
                    egui::RichText::new("Garante que apontar para cima sempre mova o mouse para cima, mesmo segurando o controle inclinado ou deitado de lado.")
                        .small()
                        .color(Color32::GRAY),
                );
            });
        });

        if changed {
            let _ = self.driver_cmd_tx.send(DriverCommand::UpdateConfig(self.config.clone()));
        }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurações do Sistema e Dispositivo");
        ui.add_space(4.0);

        let mut config_changed = false;

        let avail_h = ui.available_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(avail_h)
            .show(ui, |ui| {
            // 1. Monitores & Exibição
            ui.group(|ui| {
                ui.label(egui::RichText::new("Monitores & Área de Controle:").strong());
                ui.label(
                    egui::RichText::new("Permite confinar os movimentos do Magic Remote a um monitor específico em computadores com múltiplas telas.")
                        .small()
                        .color(Color32::GRAY),
                );

                let monitors = crate::infra::input::list_monitors();
                let current_monitor_label = match self.config.target_monitor {
                    Some(id) => {
                        if let Some(m) = monitors.iter().find(|m| m.id == id) {
                            format!("{} ({}x{})", m.name, m.rect[2] - m.rect[0], m.rect[3] - m.rect[1])
                        } else {
                            format!("Monitor {id}")
                        }
                    }
                    None => "Todos os Monitores (Livre)".to_string(),
                };

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Restringir a:");
                    egui::ComboBox::from_id_salt("settings_monitor_selector")
                        .selected_text(&current_monitor_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.config.target_monitor.is_none(), "Todos os Monitores (Livre)").clicked() && self.config.target_monitor.is_some() {
                                self.config.target_monitor = None;
                                config_changed = true;
                            }
                            for m in &monitors {
                                let is_selected = self.config.target_monitor == Some(m.id);
                                let label = format!("{} ({}x{})", m.name, m.rect[2] - m.rect[0], m.rect[3] - m.rect[1]);
                                if ui.selectable_label(is_selected, label).clicked() && !is_selected {
                                    self.config.target_monitor = Some(m.id);
                                    config_changed = true;
                                }
                            }
                        });
                });
            });

            ui.add_space(8.0);

            // 2. Sistema & Inicialização
            ui.group(|ui| {
                ui.label(egui::RichText::new("Sistema & Inicialização:").strong());
                let mut autostart = is_autostart_enabled();
                if ui.checkbox(&mut autostart, "Iniciar automaticamente junto com o Windows").changed() {
                    let _ = set_autostart(autostart);
                    self.config.auto_start = autostart;
                    self.save_config();
                }
                ui.label(
                    egui::RichText::new("Ao fechar esta janela no 'X', o driver continuará em segundo plano na bandeja do sistema.")
                        .small()
                        .color(Color32::GRAY),
                );
            });

            ui.add_space(8.0);

            // 3. Conexão & Porta Serial
            ui.group(|ui| {
                ui.label(egui::RichText::new("Conexão e Porta Serial (COM):").strong());
                ui.label(
                    egui::RichText::new("Por padrão, o driver detecta automaticamente a porta do receptor CP2102 da LG.")
                        .small()
                        .color(Color32::GRAY),
                );

                let mut available_ports = crate::infra::serial::list_available_ports();
                available_ports.sort();

                let current_port_label = match &self.config.port_override {
                    Some(p) if !p.trim().is_empty() => format!("Manual: {p}"),
                    _ => "Automático (Auto-detectar receptor CP2102)".to_string(),
                };

                let mut port_changed = false;
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Modo da Porta:");
                    egui::ComboBox::from_id_salt("settings_port_selector")
                        .selected_text(&current_port_label)
                        .show_ui(ui, |ui| {
                            let is_auto = self.config.port_override.is_none();
                            if ui.selectable_label(is_auto, "Automático (Auto-detectar receptor CP2102)").clicked() && !is_auto {
                                self.config.port_override = None;
                                port_changed = true;
                            }

                            if !available_ports.is_empty() {
                                ui.separator();
                                for port in &available_ports {
                                    let is_selected = self.config.port_override.as_deref() == Some(port.as_str());
                                    if ui.selectable_label(is_selected, format!("Forçar {port}")).clicked() && !is_selected {
                                        self.config.port_override = Some(port.clone());
                                        port_changed = true;
                                    }
                                }
                            }
                        });
                });

                if port_changed {
                    self.save_config();
                }
            });

            ui.add_space(8.0);

            // 4. Calibração de Ponto Neutro (Drift do Giroscópio)
            ui.group(|ui| {
                ui.label(egui::RichText::new("Calibração de Ponto Neutro (Zero do Giroscópio):").strong());
                ui.label(
                    egui::RichText::new(
                        "O giroscópio mede a velocidade angular da sua mão. Se com o controle parado sobre a mesa \
                        o cursor do mouse estiver se deslocando vagarosamente sozinho (drift), coloque o controle \
                        imóvel sobre uma superfície plana e clique em 'Calibrar Ponto Neutro Atual'.",
                    )
                    .small()
                    .color(Color32::GRAY),
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Centro calibrado atual: X={:.1}, Y={:.1} (Padrão de fábrica: 537.0)",
                        self.latest_telemetry.center_x, self.latest_telemetry.center_y
                    ));
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Calibrar Ponto Neutro Atual").clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::Recenter);
                        self.attitude.recenter();
                        self.remote_yaw = 0.0;
                        self.remote_pitch = 0.0;
                        self.remote_roll = 0.0;
                        self.calibration_feedback = Some((
                            format!(
                                "Calibrado com a posição atual (X={}, Y={})!",
                                self.latest_telemetry.gyro_x, self.latest_telemetry.gyro_y
                            ),
                            std::time::Instant::now(),
                        ));
                    }

                    if ui.button("Restaurar Centro Padrão (537)").clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::ResetCenter);
                        self.calibration_feedback = Some((
                            "Centro restaurado para o padrão de hardware (537.0).".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                });

                if let Some((ref msg, time)) = self.calibration_feedback {
                    if time.elapsed() < std::time::Duration::from_secs(4) {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(msg).color(Color32::from_rgb(100, 220, 140)));
                    }
                }
            });

            ui.add_space(8.0);

            // 5. Restauração de Fábrica
            ui.group(|ui| {
                ui.label(egui::RichText::new("Restaurar Padrões:").strong());
                ui.label(
                    egui::RichText::new("Restaura todos os mapeamentos de botões, sensibilidade e opções para as configurações de fábrica.")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(4.0);
                if ui.button("Restaurar Todas as Configurações de Fábrica").clicked() {
                    self.config = Config::default();
                    self.save_config();
                    let _ = self.driver_cmd_tx.send(DriverCommand::ResetCenter);
                }
            });
        });

        if config_changed {
            self.save_config();
        }
    }

    fn render_sensors_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dados Brutos do Controle em Tempo Real");
        ui.add_space(4.0);

        let t = self.latest_telemetry.clone();

        let avail_h = ui.available_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(avail_h)
            .show(ui, |ui| {
            // 1. Conexão & Telemetria
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Status da Conexão:").strong());
                    if t.connected {
                        ui.label(egui::RichText::new("CONECTADO").color(Color32::from_rgb(40, 220, 100)).strong());
                    } else {
                        ui.label(egui::RichText::new("DESCONECTADO").color(Color32::from_rgb(230, 60, 60)).strong());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.1} Hz (~{:.1} ms)", t.rate_hz, if t.rate_hz > 0.0 { 1000.0 / t.rate_hz } else { 0.0 })).strong());
                    });
                });

                ui.horizontal(|ui| {
                    let port_txt = if t.port_name.is_empty() { "Nenhuma" } else { &t.port_name };
                    ui.label(format!("Porta COM: {port_txt}"));
                    ui.separator();
                    ui.label(format!("Inclinação (Roll): {:.1}°", t.roll_deg));
                });
            });

            ui.add_space(6.0);

            // 2. Giroscópio e Acelerômetro Lado a Lado (2 Quadrados)
            ui.columns(2, |cols| {
                // Quadrado 1: Giroscópio
                cols[0].group(|ui| {
                    ui.set_min_height(165.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Giroscópio").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let gyro_mag = ((t.gyro_dx * t.gyro_dx + t.gyro_dy * t.gyro_dy) as f32).sqrt();
                            ui.label(
                                egui::RichText::new(format!("Δ: {:.0}", gyro_mag))
                                    .color(Color32::from_rgb(0, 210, 255))
                                    .small(),
                            );
                        });
                    });
                    ui.add_space(6.0);

                    let delta_x_str = format!("Delta: {:+3}", t.gyro_dx);
                    draw_bipolar_slide(
                        ui,
                        "Eixo X (Yaw):",
                        t.gyro_x as f32,
                        t.center_x,
                        350.0,
                        750.0,
                        &delta_x_str,
                        Color32::from_rgb(0, 210, 255),
                    );

                    let delta_y_val = format!("Delta: {:+3}", t.gyro_dy);
                    draw_bipolar_slide(
                        ui,
                        "Eixo Y (Pitch):",
                        t.gyro_y as f32,
                        t.center_y,
                        350.0,
                        750.0,
                        &delta_y_val,
                        Color32::from_rgb(255, 195, 30),
                    );

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Centro X: {:.0}  |  Centro Y: {:.0}", t.center_x, t.center_y))
                            .small()
                            .color(Color32::from_rgb(120, 135, 155)),
                    );
                });

                // Quadrado 2: Acelerômetro
                cols[1].group(|ui| {
                    ui.set_min_height(165.0);
                    let [g_x, g_y, g_z] = crate::domain::gravity::AccelCalibration::LG_AN_MR200
                        .to_g([t.accel_x, t.accel_y, t.accel_z]);
                    let total_g = (g_x * g_x + g_y * g_y + g_z * g_z).sqrt();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Acelerômetro").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let g_color = if (total_g - 1.0).abs() < 0.15 {
                                Color32::from_rgb(40, 220, 100)
                            } else {
                                Color32::from_rgb(255, 195, 30)
                            };
                            ui.label(egui::RichText::new(format!("{:.2}g", total_g)).color(g_color).strong());
                        });
                    });
                    ui.add_space(4.0);

                    draw_bipolar_slide(
                        ui,
                        "Acel X (Frente):",
                        t.accel_x as f32,
                        2055.0,
                        1000.0,
                        3100.0,
                        &format!("({:+.2}g)", g_x),
                        Color32::from_rgb(244, 63, 94),
                    );

                    draw_bipolar_slide(
                        ui,
                        "Acel Y (Lateral):",
                        t.accel_y as f32,
                        2012.0,
                        1000.0,
                        3100.0,
                        &format!("({:+.2}g)", g_y),
                        Color32::from_rgb(34, 197, 94),
                    );

                    draw_bipolar_slide(
                        ui,
                        "Acel Z (Botões):",
                        t.accel_z as f32,
                        2180.0,
                        1000.0,
                        3100.0,
                        &format!("({:+.2}g)", g_z),
                        Color32::from_rgb(14, 165, 233),
                    );
                });
            });

            ui.add_space(6.0);

            // 3. Gráfico de Sensores com Toggles Individuais (em cima do 3D)
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Gráfico de Sensores").strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 1. Botão de Gravação (extrema direita)
                        if self.is_recording_sensors {
                            if ui
                                .button(
                                    egui::RichText::new("⏹ Parar e Salvar")
                                        .color(Color32::from_rgb(255, 110, 110))
                                        .small(),
                                )
                                .clicked()
                            {
                                self.stop_recording_sensors();
                            }
                        } else {
                            if ui
                                .button(egui::RichText::new("⏺ Gravar Dados").small())
                                .on_hover_text("Inicia a gravação de todas as leituras a ~111 Hz em arquivo CSV")
                                .clicked()
                            {
                                self.start_recording_sensors();
                            }
                        }

                        // 2. Botão com ícone de pasta sem texto logo ao lado do botão de gravar
                        if ui
                            .button(egui::RichText::new("📂").size(13.0))
                            .on_hover_text("Abrir pasta onde as gravações CSV são salvas")
                            .clicked()
                        {
                            let _ = std::process::Command::new("explorer").arg(".").spawn();
                        }

                        // 3. Indicador de gravação ou feedback
                        if self.is_recording_sensors {
                            let count = self.recorded_samples.len();
                            let elapsed = self
                                .recording_start_instant
                                .map(|t| t.elapsed().as_secs_f32())
                                .unwrap_or(0.0);

                            ui.label(
                                egui::RichText::new(format!("● Gravando ({count} amostras, {elapsed:.1}s)"))
                                    .color(Color32::from_rgb(255, 80, 80))
                                    .strong()
                                    .small(),
                            );
                        } else if let Some((ref msg, time)) = self.recording_status_msg {
                            if time.elapsed() < std::time::Duration::from_secs(8) {
                                ui.label(
                                    egui::RichText::new(msg)
                                        .color(Color32::from_rgb(100, 220, 140))
                                        .small(),
                                );
                            }
                        }
                    });
                });
                ui.add_space(3.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Canais:").small().color(Color32::from_rgb(160, 175, 200)));
                    ui.checkbox(&mut self.osc_gyro_x, egui::RichText::new("Giro X (Yaw)").color(Color32::from_rgb(0, 210, 255)).small());
                    ui.checkbox(&mut self.osc_gyro_y, egui::RichText::new("Giro Y (Pitch)").color(Color32::from_rgb(255, 195, 30)).small());
                    ui.checkbox(&mut self.osc_accel_x, egui::RichText::new("Acel X (Frente)").color(Color32::from_rgb(244, 63, 94)).small());
                    ui.checkbox(&mut self.osc_accel_y, egui::RichText::new("Acel Y (Lateral)").color(Color32::from_rgb(34, 197, 94)).small());
                    ui.checkbox(&mut self.osc_accel_z, egui::RichText::new("Acel Z (Botões)").color(Color32::from_rgb(14, 165, 233)).small());
                });

                ui.add_space(4.0);

                draw_multichannel_oscilloscope(
                    ui,
                    &self.gyro_history_x,
                    &self.gyro_history_y,
                    &self.accel_history_x,
                    &self.accel_history_y,
                    &self.accel_history_z,
                    t.center_x,
                    t.center_y,
                    self.osc_gyro_x,
                    self.osc_gyro_y,
                    self.osc_accel_x,
                    self.osc_accel_y,
                    self.osc_accel_z,
                );
            });

            ui.add_space(6.0);

            // 4. Objeto 3D da Orientação do Controle (embaixo do osciloscópio)
            draw_3d_orientation_viewer(
                ui,
                &mut self.mesh_3d,
                &mut self.attitude,
                &mut self.remote_yaw,
                &mut self.remote_pitch,
                &mut self.remote_roll,
            );

            ui.add_space(6.0);

            // 5. Botão Pressionado com Hexadecimal Bruto (Simplificado)
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Botão Pressionado:").strong());

                    let hex_color = if t.button_raw != 0 {
                        Color32::from_rgb(0, 255, 180)
                    } else {
                        Color32::from_rgb(140, 155, 175)
                    };

                    ui.label(
                        egui::RichText::new(format!("Hex: 0x{:02X}", t.button_raw))
                            .monospace()
                            .size(15.0)
                            .color(hex_color)
                            .strong(),
                    );

                    ui.separator();

                    if t.active_buttons.is_empty() {
                        ui.label(egui::RichText::new("Nenhum botão pressionado").color(Color32::GRAY));
                    } else {
                        for b in t.active_buttons.iter() {
                            ui.label(
                                egui::RichText::new(format!("● {:?}", b))
                                    .color(Color32::from_rgb(0, 255, 180))
                                    .strong(),
                            );
                        }
                    }
                });
            });
        });
    }
}

fn draw_3d_orientation_viewer(
    ui: &mut egui::Ui,
    mesh_3d: &mut crate::presentation::mesh3d::Mesh3D,
    attitude: &mut crate::domain::attitude::AttitudeEstimator,
    remote_yaw: &mut f32,
    remote_pitch: &mut f32,
    remote_roll: &mut f32,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Visualização 3D").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("⟲ Centralizar").small())
                    .on_hover_text("Define a posição física atual do controle como centro neutro de referência")
                    .clicked()
                {
                    attitude.recenter();
                    *remote_yaw = 0.0;
                    *remote_pitch = 0.0;
                    *remote_roll = 0.0;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "Yaw: {:+.1}°  |  Pitch: {:+.1}°  |  Roll: {:+.1}°",
                        *remote_yaw, *remote_pitch, *remote_roll
                    ))
                    .color(Color32::from_rgb(180, 210, 255))
                    .strong(),
                );
            });
        });
        ui.add_space(4.0);

        let height = 230.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Canvas background
        painter.rect_filled(rect, 6.0, Color32::from_rgb(14, 17, 24));
        painter.rect_stroke(rect, 6.0, Stroke::new(1.0, Color32::from_rgb(38, 46, 64)), StrokeKind::Inside);

        let center = Pos2::new(rect.min.x + rect.width() * 0.5, rect.min.y + rect.height() * 0.52);

        // Render real 3D mesh with continuous 3D rotation matrix (no Euler singularities / flips)
        let rel_rot = attitude.relative_matrix();
        mesh_3d.render(&painter, center, &rel_rot, 1.0);
    });
}

fn draw_bipolar_slide(
    ui: &mut egui::Ui,
    label: &str,
    val: f32,
    center: f32,
    min_range: f32,
    max_range: f32,
    extra_info: &str,
    bar_color: Color32,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong());
        if !extra_info.is_empty() {
            ui.label(egui::RichText::new(extra_info).color(Color32::from_rgb(160, 170, 190)).small());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{:.0}", val)).strong().monospace());
        });
    });

    let height = 15.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Track background
    painter.rect_filled(rect, 3.0, Color32::from_rgb(24, 27, 34));
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_rgb(42, 48, 60)), StrokeKind::Inside);

    // Center marker
    let norm_center = ((center - min_range) / (max_range - min_range)).clamp(0.0, 1.0);
    let center_x = rect.min.x + rect.width() * norm_center;
    painter.line_segment(
        [Pos2::new(center_x, rect.min.y), Pos2::new(center_x, rect.max.y)],
        Stroke::new(1.5, Color32::from_rgb(90, 105, 125)),
    );

    // Value bar
    let clamped_val = val.clamp(min_range, max_range);
    let norm_val = (clamped_val - min_range) / (max_range - min_range);
    let val_x = rect.min.x + rect.width() * norm_val;

    if (val_x - center_x).abs() > 1.0 {
        let (x_start, x_end) = if val_x < center_x { (val_x, center_x) } else { (center_x, val_x) };
        let bar_rect = Rect::from_min_max(Pos2::new(x_start, rect.min.y + 2.0), Pos2::new(x_end, rect.max.y - 2.0));
        painter.rect_filled(bar_rect, 2.0, bar_color);
    }

    // Pin indicator
    painter.line_segment(
        [Pos2::new(val_x, rect.min.y - 1.0), Pos2::new(val_x, rect.max.y + 1.0)],
        Stroke::new(2.5, Color32::WHITE),
    );
    ui.add_space(3.0);
}


fn draw_multichannel_oscilloscope(
    ui: &mut egui::Ui,
    gyro_x: &VecDeque<f32>,
    gyro_y: &VecDeque<f32>,
    accel_x: &VecDeque<f32>,
    accel_y: &VecDeque<f32>,
    accel_z: &VecDeque<f32>,
    center_gx: f32,
    center_gy: f32,
    show_gyro_x: bool,
    show_gyro_y: bool,
    show_accel_x: bool,
    show_accel_y: bool,
    show_accel_z: bool,
) {
    let height = 110.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Frame styling
    painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 15, 22));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(34, 42, 58)), StrokeKind::Inside);

    let mid_y = rect.min.y + rect.height() * 0.5;

    // Grid lines: center (0.0)
    painter.line_segment(
        [Pos2::new(rect.min.x, mid_y), Pos2::new(rect.max.x, mid_y)],
        Stroke::new(1.0, Color32::from_rgb(45, 55, 75)),
    );

    // Grid lines: +1.0 and -1.0
    let span = 1.6f32; // normalized units (+/- 1.6)
    let unit_scale = (rect.height() * 0.42) / span;

    let pos_1_y = mid_y - 1.0 * unit_scale;
    let neg_1_y = mid_y + 1.0 * unit_scale;

    painter.line_segment(
        [Pos2::new(rect.min.x, pos_1_y), Pos2::new(rect.max.x, pos_1_y)],
        Stroke::new(0.8, Color32::from_rgb(26, 32, 45)),
    );
    painter.line_segment(
        [Pos2::new(rect.min.x, neg_1_y), Pos2::new(rect.max.x, neg_1_y)],
        Stroke::new(0.8, Color32::from_rgb(26, 32, 45)),
    );

    // Scale labels
    painter.text(
        Pos2::new(rect.min.x + 4.0, pos_1_y - 1.0),
        egui::Align2::LEFT_CENTER,
        "+1.0",
        egui::FontId::monospace(9.0),
        Color32::from_rgb(80, 95, 120),
    );
    painter.text(
        Pos2::new(rect.min.x + 4.0, mid_y - 1.0),
        egui::Align2::LEFT_CENTER,
        " 0.0",
        egui::FontId::monospace(9.0),
        Color32::from_rgb(80, 95, 120),
    );
    painter.text(
        Pos2::new(rect.min.x + 4.0, neg_1_y - 1.0),
        egui::Align2::LEFT_CENTER,
        "-1.0",
        egui::FontId::monospace(9.0),
        Color32::from_rgb(80, 95, 120),
    );

    // Helper to plot a normalized channel
    let plot_channel = |history: &VecDeque<f32>, normalize_fn: &dyn Fn(f32) -> f32, color: Color32| {
        let n = history.len();
        if n >= 2 {
            let step_x = rect.width() / (n - 1) as f32;
            let points: Vec<Pos2> = history
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let norm = normalize_fn(v);
                    let px = rect.min.x + i as f32 * step_x;
                    let py = mid_y - norm * unit_scale;
                    Pos2::new(px, py.clamp(rect.min.y + 2.0, rect.max.y - 2.0))
                })
                .collect();

            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.7, color));
            }
        }
    };

    if show_gyro_x {
        plot_channel(gyro_x, &|v| (v - center_gx) / 100.0, Color32::from_rgb(0, 210, 255));
    }
    if show_gyro_y {
        plot_channel(gyro_y, &|v| (v - center_gy) / 100.0, Color32::from_rgb(255, 195, 30));
    }
    if show_accel_x {
        plot_channel(accel_x, &|v| (v - 2055.0) / 1018.0, Color32::from_rgb(244, 63, 94));
    }
    if show_accel_y {
        plot_channel(accel_y, &|v| (v - 2012.0) / 1037.0, Color32::from_rgb(34, 197, 94));
    }
    if show_accel_z {
        plot_channel(accel_z, &|v| (v - 2180.0) / 1037.0, Color32::from_rgb(14, 165, 233));
    }
}

fn egui_key_to_vk(key: egui::Key) -> Option<u16> {
    let vk = match key {
        egui::Key::A => 0x41,
        egui::Key::B => 0x42,
        egui::Key::C => 0x43,
        egui::Key::D => 0x44,
        egui::Key::E => 0x45,
        egui::Key::F => 0x46,
        egui::Key::G => 0x47,
        egui::Key::H => 0x48,
        egui::Key::I => 0x49,
        egui::Key::J => 0x4A,
        egui::Key::K => 0x4B,
        egui::Key::L => 0x4C,
        egui::Key::M => 0x4D,
        egui::Key::N => 0x4E,
        egui::Key::O => 0x4F,
        egui::Key::P => 0x50,
        egui::Key::Q => 0x51,
        egui::Key::R => 0x52,
        egui::Key::S => 0x53,
        egui::Key::T => 0x54,
        egui::Key::U => 0x55,
        egui::Key::V => 0x56,
        egui::Key::W => 0x57,
        egui::Key::X => 0x58,
        egui::Key::Y => 0x59,
        egui::Key::Z => 0x5A,
        egui::Key::Num0 => 0x30,
        egui::Key::Num1 => 0x31,
        egui::Key::Num2 => 0x32,
        egui::Key::Num3 => 0x33,
        egui::Key::Num4 => 0x34,
        egui::Key::Num5 => 0x35,
        egui::Key::Num6 => 0x36,
        egui::Key::Num7 => 0x37,
        egui::Key::Num8 => 0x38,
        egui::Key::Num9 => 0x39,
        egui::Key::F1 => 0x70,
        egui::Key::F2 => 0x71,
        egui::Key::F3 => 0x72,
        egui::Key::F4 => 0x73,
        egui::Key::F5 => 0x74,
        egui::Key::F6 => 0x75,
        egui::Key::F7 => 0x76,
        egui::Key::F8 => 0x77,
        egui::Key::F9 => 0x78,
        egui::Key::F10 => 0x79,
        egui::Key::F11 => 0x7A,
        egui::Key::F12 => 0x7B,
        egui::Key::ArrowDown => 0x28,
        egui::Key::ArrowLeft => 0x25,
        egui::Key::ArrowRight => 0x27,
        egui::Key::ArrowUp => 0x26,
        egui::Key::PageUp => 0x21,
        egui::Key::PageDown => 0x22,
        egui::Key::Home => 0x24,
        egui::Key::End => 0x23,
        egui::Key::Escape => 0x1B,
        egui::Key::Tab => 0x09,
        egui::Key::Backspace => 0x08,
        egui::Key::Enter => 0x0D,
        egui::Key::Space => 0x20,
        egui::Key::Insert => 0x2D,
        egui::Key::Delete => 0x2E,
        egui::Key::Minus => 0xBD,
        egui::Key::Equals => 0xBB,
        egui::Key::OpenBracket => 0xDB,
        egui::Key::CloseBracket => 0xDD,
        egui::Key::Backslash => 0xDC,
        egui::Key::Semicolon => 0xBA,
        egui::Key::Quote => 0xDE,
        egui::Key::Comma => 0xBC,
        egui::Key::Period => 0xBE,
        egui::Key::Slash => 0xBF,
        _ => return None,
    };
    Some(vk)
}

fn format_timestamp(secs: u64) -> String {
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let mut days = secs / 86400;
    let mut year = 1970;
    loop {
        let leap = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 1 } else { 0 };
        let days_in_year = 365 + leap;
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let leap = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 1 } else { 0 };
    let month_days = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    for &d in &month_days {
        if days >= d {
            days -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = days + 1;
    format!("{year:04}{month:02}{day:02}_{h:02}{m:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "19700101_000000");
        let ts = format_timestamp(1762129500);
        assert!(ts.starts_with("2025"));
    }
}


