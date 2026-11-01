use crate::config::AppConfig;
use crate::crypto::generate_entropy;
use crate::input::mapper::{egui_key_to_vk, CoordinateMapper};
use crate::net::{HostSession, SignalingClient, SignalingEvent, ViewerSession};
use crate::protocol::{InputPayload, MouseButton};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Home,
    Settings,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    LocalNetwork,
    RenderCloud,
}

struct IncomingRequestState {
    session_id: String,
    from_id: String,
    client_entropy: Vec<u8>,
}

pub struct RemoteLiteApp {
    config: AppConfig,
    signaling: SignalingClient,
    events_rx: mpsc::UnboundedReceiver<SignalingEvent>,

    // Connection state
    is_online: bool,
    copied_time: Option<Instant>,
    full_copied_time: Option<Instant>,
    remote_id_input: String,
    connect_status: Option<String>,
    pending_connect: Option<(String, String, [u8; 32])>,

    // Incoming request on host
    incoming_request: Option<IncomingRequestState>,

    // Active sessions
    host_session: Option<HostSession>,
    viewer_session: Option<ViewerSession>,

    // Viewer texture & display
    viewer_texture: Option<egui::TextureHandle>,
    remote_resolution: (u32, u32),
    latency_ms: u64,
    last_mouse_norm: Option<(f32, f32)>,

    // UI Navigation & Configuration
    active_tab: ActiveTab,
    network_mode: NetworkMode,
    server_url_input: String,
}

impl RemoteLiteApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        let server_url_input = config.signaling_server_url.clone();
        let (signaling, events_rx) = SignalingClient::connect(
            config.signaling_server_url.clone(),
            config.device_id.as_str().to_string(),
        );

        Self {
            config,
            signaling,
            events_rx,
            is_online: false,
            copied_time: None,
            full_copied_time: None,
            remote_id_input: String::new(),
            connect_status: None,
            pending_connect: None,
            incoming_request: None,
            host_session: None,
            viewer_session: None,
            viewer_texture: None,
            remote_resolution: (1920, 1080),
            latency_ms: 0,
            last_mouse_norm: None,
            active_tab: ActiveTab::Home,
            network_mode: NetworkMode::LocalNetwork,
            server_url_input,
        }
    }

    /// Process asynchronous network events.
    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                SignalingEvent::Connected | SignalingEvent::Registered { .. } => {
                    self.is_online = true;
                    ctx.request_repaint();
                }
                SignalingEvent::Disconnected => {
                    self.is_online = false;
                    ctx.request_repaint();
                }
                SignalingEvent::IncomingRequest {
                    session_id,
                    from_id,
                    key_exchange,
                } => {
                    self.incoming_request = Some(IncomingRequestState {
                        session_id,
                        from_id,
                        client_entropy: key_exchange,
                    });
                    ctx.request_repaint();
                }
                SignalingEvent::ConnectResponse {
                    session_id,
                    from_id,
                    accepted,
                    reason,
                    key_exchange,
                } => {
                    if let Some((pend_sess, target_id, client_entropy)) = self.pending_connect.take() {
                        if pend_sess == session_id && target_id == from_id {
                            if accepted {
                                if let Some(host_entropy) = key_exchange {
                                    self.connect_status = Some("Connected! Streaming desktop...".to_string());
                                    let session = ViewerSession::new(
                                        session_id,
                                        from_id,
                                        self.signaling.clone(),
                                        &client_entropy,
                                        &host_entropy,
                                    );
                                    self.viewer_session = Some(session);
                                } else {
                                    self.connect_status = Some("Connection failed: Missing host cryptographic key".to_string());
                                }
                            } else {
                                self.connect_status = Some(format!(
                                    "Connection rejected: {}",
                                    reason.unwrap_or_else(|| "User declined".into())
                                ));
                            }
                        }
                    }
                    ctx.request_repaint();
                }
                SignalingEvent::RelayedData {
                    session_id,
                    payload,
                    ..
                } => {
                    if let Some(ref mut viewer) = self.viewer_session {
                        if viewer.session_id == session_id {
                            if let Ok(Some(frame_data)) = viewer.handle_incoming_payload(&payload) {
                                self.remote_resolution = (frame_data.width, frame_data.height);
                                self.latency_ms = frame_data.latency_ms;

                                if let Some(ref mut tex) = self.viewer_texture {
                                    tex.set(frame_data.image, egui::TextureOptions::LINEAR);
                                } else {
                                    self.viewer_texture = Some(ctx.load_texture(
                                        "remote_desktop",
                                        frame_data.image,
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                ctx.request_repaint();
                            }
                        }
                    }

                    if let Some(ref host) = self.host_session {
                        if host.session_id == session_id {
                            let _ = host.handle_incoming_payload(&payload);
                        }
                    }
                }
                SignalingEvent::PeerDisconnected { session_id, reason } => {
                    if let Some(ref mut viewer) = self.viewer_session {
                        if viewer.session_id == session_id {
                            self.viewer_session = None;
                            self.viewer_texture = None;
                            self.connect_status = Some(format!("Session ended: {}", reason));
                        }
                    }
                    if let Some(ref mut host) = self.host_session {
                        if host.session_id == session_id {
                            self.host_session = None;
                        }
                    }
                    ctx.request_repaint();
                }
                SignalingEvent::Error(err) => {
                    self.connect_status = Some(format!("Error: {}", err));
                    ctx.request_repaint();
                }
            }
        }
    }

    /// Initiate connection to remote device ID.
    fn start_connection(&mut self) {
        let clean_id: String = self.remote_id_input.chars().filter(|c| c.is_ascii_digit()).collect();
        if clean_id.len() != 9 {
            self.connect_status = Some("Error: Device ID must be 9 digits".to_string());
            return;
        }

        if clean_id == self.config.device_id.as_str() {
            self.connect_status = Some("Cannot connect to your own device ID".to_string());
            return;
        }

        let session_id = format!("sess_{}", rand::random::<u32>());
        let client_entropy = generate_entropy();

        self.connect_status = Some("Connecting: Waiting for Approval...".to_string());
        self.pending_connect = Some((session_id.clone(), clean_id.clone(), client_entropy));

        if let Err(e) = self.signaling.send_connect_request(session_id, clean_id, client_entropy.to_vec()) {
            self.connect_status = Some(format!("Failed to send connection request: {}", e));
            self.pending_connect = None;
        }
    }

    /// Accept incoming connection request.
    fn accept_incoming_request(&mut self) {
        if let Some(req) = self.incoming_request.take() {
            match HostSession::start(
                req.session_id.clone(),
                req.from_id.clone(),
                &req.client_entropy,
                self.signaling.clone(),
                self.config.target_fps,
                self.config.jpeg_quality,
            ) {
                Ok((session, host_entropy)) => {
                    self.host_session = Some(session);
                    let _ = self.signaling.send_connect_response(
                        req.session_id,
                        true,
                        None,
                        Some(host_entropy),
                    );
                }
                Err(e) => {
                    let _ = self.signaling.send_connect_response(
                        req.session_id,
                        false,
                        Some(format!("Host failed to start screen capture: {}", e)),
                        None,
                    );
                }
            }
        }
    }

    /// Reject incoming connection request.
    fn reject_incoming_request(&mut self) {
        if let Some(req) = self.incoming_request.take() {
            let _ = self.signaling.send_connect_response(
                req.session_id,
                false,
                Some("Connection rejected by host user".to_string()),
                None,
            );
        }
    }

    /// Formats a long deterministic connection token for manual / offline pairing.
    fn full_connection_code(&self) -> String {
        let id = self.config.device_id.as_str();
        format!(
            "{}103985739303186591558845001296204869487541426861302160002916059741785102585757697",
            id
        )
    }
}

impl eframe::App for RemoteLiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);

        if self.viewer_session.is_some() {
            ctx.request_repaint();
            self.render_viewer(ctx);
        } else {
            self.render_main_window(ctx);
        }
    }
}

// ----------------------------------------------------------------------------
// VECTOR ICON DRAWING HELPERS
// ----------------------------------------------------------------------------

fn draw_lightning(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size * 0.5;
    let points = vec![
        Pos2::new(center.x + s * 0.15, center.y - s),
        Pos2::new(center.x - s * 0.55, center.y + s * 0.1),
        Pos2::new(center.x - s * 0.05, center.y + s * 0.1),
        Pos2::new(center.x - s * 0.25, center.y + s),
        Pos2::new(center.x + s * 0.55, center.y - s * 0.15),
        Pos2::new(center.x + s * 0.05, center.y - s * 0.15),
    ];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

fn draw_monitor(painter: &egui::Painter, rect: Rect, stroke_color: Color32, fill_color: Color32) {
    let screen_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.min.y),
        Pos2::new(rect.max.x, rect.max.y - rect.height() * 0.28),
    );
    painter.rect(screen_rect, 3.0, fill_color, Stroke::new(1.8_f32, stroke_color));

    let stand_cx = rect.center().x;
    let stand_ty = screen_rect.max.y;
    let stand_by = rect.max.y - 1.0;
    painter.line_segment([Pos2::new(stand_cx, stand_ty), Pos2::new(stand_cx, stand_by)], Stroke::new(2.0_f32, stroke_color));

    let base_w = rect.width() * 0.55;
    painter.line_segment(
        [
            Pos2::new(stand_cx - base_w * 0.5, stand_by),
            Pos2::new(stand_cx + base_w * 0.5, stand_by),
        ],
        Stroke::new(2.0_f32, stroke_color),
    );
}

fn draw_monitor_arrow(painter: &egui::Painter, rect: Rect, color: Color32) {
    let screen_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.min.y),
        Pos2::new(rect.max.x - rect.width() * 0.22, rect.max.y - rect.height() * 0.28),
    );
    painter.rect_stroke(screen_rect, 3.0, Stroke::new(1.8_f32, color));

    let stand_cx = screen_rect.center().x;
    let stand_ty = screen_rect.max.y;
    let stand_by = rect.max.y - 1.0;
    painter.line_segment([Pos2::new(stand_cx, stand_ty), Pos2::new(stand_cx, stand_by)], Stroke::new(2.0_f32, color));
    painter.line_segment(
        [Pos2::new(stand_cx - 6.0, stand_by), Pos2::new(stand_cx + 6.0, stand_by)],
        Stroke::new(2.0_f32, color),
    );

    // Arrow out
    let cy = screen_rect.center().y;
    let arrow_end = Pos2::new(rect.max.x + 1.0, cy);
    painter.line_segment([Pos2::new(stand_cx, cy), arrow_end], Stroke::new(2.0_f32, color));
    painter.line_segment([Pos2::new(arrow_end.x - 4.5, cy - 4.5), arrow_end], Stroke::new(2.0_f32, color));
    painter.line_segment([Pos2::new(arrow_end.x - 4.5, cy + 4.5), arrow_end], Stroke::new(2.0_f32, color));
}

fn draw_shield_check(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size * 0.5;
    let p0 = Pos2::new(center.x, center.y - s);
    let p1 = Pos2::new(center.x + s * 0.8, center.y - s * 0.55);
    let p2 = Pos2::new(center.x + s * 0.8, center.y + s * 0.1);
    let p3 = Pos2::new(center.x, center.y + s);
    let p4 = Pos2::new(center.x - s * 0.8, center.y + s * 0.1);
    let p5 = Pos2::new(center.x - s * 0.8, center.y - s * 0.55);
    painter.add(egui::Shape::closed_line(vec![p0, p1, p2, p3, p4, p5], Stroke::new(2.0_f32, color)));

    let c0 = Pos2::new(center.x - s * 0.35, center.y + s * 0.05);
    let c1 = Pos2::new(center.x - s * 0.05, center.y + s * 0.35);
    let c2 = Pos2::new(center.x + s * 0.4, center.y - s * 0.25);
    painter.line_segment([c0, c1], Stroke::new(2.2_f32, color));
    painter.line_segment([c1, c2], Stroke::new(2.2_f32, color));
}

fn draw_lock(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size * 0.5;
    let body = Rect::from_center_size(Pos2::new(center.x, center.y + s * 0.3), Vec2::new(s * 1.3, s * 0.95));
    painter.rect_filled(body, 2.5, color);

    let shackle = Rect::from_center_size(Pos2::new(center.x, center.y - s * 0.32), Vec2::new(s * 0.85, s * 0.75));
    painter.rect_stroke(shackle, 4.0, Stroke::new(2.0_f32, color));
}

fn draw_user_check(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size * 0.5;
    painter.circle_stroke(Pos2::new(center.x - s * 0.25, center.y - s * 0.35), s * 0.32, Stroke::new(1.8_f32, color));
    painter.circle_stroke(Pos2::new(center.x - s * 0.25, center.y + s * 0.45), s * 0.52, Stroke::new(1.8_f32, color));

    let c0 = Pos2::new(center.x + s * 0.25, center.y + s * 0.05);
    let c1 = Pos2::new(center.x + s * 0.42, center.y + s * 0.28);
    let c2 = Pos2::new(center.x + s * 0.78, center.y - s * 0.18);
    painter.line_segment([c0, c1], Stroke::new(2.0_f32, color));
    painter.line_segment([c1, c2], Stroke::new(2.0_f32, color));
}

fn draw_wifi(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size * 0.5;
    let cy = center.y + s * 0.45;
    painter.circle_stroke(Pos2::new(center.x, cy), s * 0.9, Stroke::new(1.8_f32, color));
    painter.circle_stroke(Pos2::new(center.x, cy), s * 0.55, Stroke::new(1.8_f32, color));
    painter.circle_filled(Pos2::new(center.x, cy), 2.2, color);
}

// ----------------------------------------------------------------------------
// MAIN WINDOW IMPLEMENTATION
// ----------------------------------------------------------------------------

impl RemoteLiteApp {
    fn render_main_window(&mut self, ctx: &egui::Context) {
        // Left Sidebar (Branding, Navigation, Version Badge)
        egui::SidePanel::left("instantlink_sidebar")
            .exact_width(185.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(11, 35, 38))
                    .inner_margin(egui::Margin::symmetric(14.0, 20.0)),
            )
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // Main Center Panel (Dynamic Header, Home / Settings / About, Modal Overlays)
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();

                // Smooth background gradient (Dark teal at top -> soft mint-slate backdrop at bottom)
                let top_color = Color32::from_rgb(14, 38, 42);
                let bottom_color = Color32::from_rgb(175, 206, 208);
                let mut mesh = egui::Mesh::default();
                mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_top(), uv: Pos2::ZERO, color: top_color });
                mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_top(), uv: Pos2::ZERO, color: top_color });
                mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_bottom(), uv: Pos2::ZERO, color: bottom_color });
                mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_bottom(), uv: Pos2::ZERO, color: bottom_color });
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                ui.painter().add(egui::Shape::mesh(mesh));

                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 16.0);

                    // Padding for content
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(24.0, 20.0))
                        .show(ui, |ui| {
                            // Active Remote Control Host Banner
                            self.render_active_host_banner(ui);

                            // Incoming Connection Request Modal
                            self.render_incoming_modal(ui);

                            match self.active_tab {
                                ActiveTab::Home => self.render_home_tab(ctx, ui),
                                ActiveTab::Settings => self.render_settings_tab(ui),
                                ActiveTab::About => self.render_about_tab(ui),
                            }
                        });
                });
            });
    }

    /// Renders the Left Navigation Sidebar.
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 10.0);

        // Subtle wavy curves at the bottom
        let rect = ui.max_rect();
        let painter = ui.painter();
        painter.circle_stroke(
            Pos2::new(rect.min.x - 30.0, rect.max.y - 30.0),
            170.0,
            Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(0, 210, 160, 20)),
        );
        painter.circle_stroke(
            Pos2::new(rect.min.x - 10.0, rect.max.y),
            230.0,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 210, 160, 15)),
        );

        // Top Branding: Circle + Bolt + InstantLink
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
            ui.painter().circle_filled(badge_rect.center(), 23.0, Color32::from_rgb(0, 229, 163));
            draw_lightning(ui.painter(), badge_rect.center(), 22.0, Color32::from_rgb(11, 35, 38));

            ui.add_space(8.0);
            ui.label(
                RichText::new("InstantLink")
                    .size(18.0)
                    .strong()
                    .color(Color32::WHITE),
            );
        });

        ui.add_space(24.0);

        // Tab: Home
        let is_home = self.active_tab == ActiveTab::Home;
        let home_btn = ui.add_sized(
            [ui.available_width(), 40.0],
            egui::Button::new(
                RichText::new(" ⌂   Home")
                    .size(14.0)
                    .color(if is_home { Color32::WHITE } else { Color32::from_rgb(160, 190, 190) })
                    .strong(),
            )
            .rounding(8.0)
            .fill(if is_home { Color32::from_rgb(19, 61, 63) } else { Color32::TRANSPARENT })
            .stroke(if is_home { Stroke::new(1.0_f32, Color32::from_rgb(33, 90, 93)) } else { Stroke::NONE }),
        );
        if home_btn.clicked() {
            self.active_tab = ActiveTab::Home;
        }

        // Tab: Settings
        let is_settings = self.active_tab == ActiveTab::Settings;
        let settings_btn = ui.add_sized(
            [ui.available_width(), 40.0],
            egui::Button::new(
                RichText::new(" ⚙   Settings")
                    .size(14.0)
                    .color(if is_settings { Color32::WHITE } else { Color32::from_rgb(160, 190, 190) })
                    .strong(),
            )
            .rounding(8.0)
            .fill(if is_settings { Color32::from_rgb(19, 61, 63) } else { Color32::TRANSPARENT })
            .stroke(if is_settings { Stroke::new(1.0_f32, Color32::from_rgb(33, 90, 93)) } else { Stroke::NONE }),
        );
        if settings_btn.clicked() {
            self.active_tab = ActiveTab::Settings;
        }

        // Tab: About
        let is_about = self.active_tab == ActiveTab::About;
        let about_btn = ui.add_sized(
            [ui.available_width(), 40.0],
            egui::Button::new(
                RichText::new(" ℹ   About")
                    .size(14.0)
                    .color(if is_about { Color32::WHITE } else { Color32::from_rgb(160, 190, 190) })
                    .strong(),
            )
            .rounding(8.0)
            .fill(if is_about { Color32::from_rgb(19, 61, 63) } else { Color32::TRANSPARENT })
            .stroke(if is_about { Stroke::new(1.0_f32, Color32::from_rgb(33, 90, 93)) } else { Stroke::NONE }),
        );
        if about_btn.clicked() {
            self.active_tab = ActiveTab::About;
        }

        // Bottom Badge: BASIC v0.1
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(14, 42, 44))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(29, 78, 81)))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(14.0, 5.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("BASIC")
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new("v0.1")
                                .size(11.0)
                                .color(Color32::from_rgb(120, 210, 195)),
                        );
                    });
                });
        });
    }

    /// Renders the Home Dashboard matching the exact visual layout.
    fn render_home_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // App Header: Lightning bolt + "Instant" + "Link" + Subtitle
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
            draw_lightning(ui.painter(), icon_rect.center(), 32.0, Color32::from_rgb(0, 230, 153));

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Instant")
                            .size(26.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        RichText::new("Link")
                            .size(26.0)
                            .strong()
                            .color(Color32::from_rgb(0, 230, 153)),
                    );
                });
                ui.label(
                    RichText::new("Remote access made simple and lightweight.")
                        .size(13.0)
                        .color(Color32::from_rgb(150, 185, 185)),
                );
            });

            // Online status badge on top right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.is_online {
                    ui.label(RichText::new("● Ready").color(Color32::from_rgb(0, 230, 153)).strong());
                } else {
                    ui.label(RichText::new("● Connecting to relay...").color(Color32::from_rgb(240, 180, 50)));
                }
            });
        });

        ui.add_space(10.0);

        // Two Cards Side-by-Side: "Share This PC" and "Connect to Another PC"
        ui.columns(2, |columns| {
            // CARD 1: Share This PC
            egui::Frame::none()
                .fill(Color32::from_rgb(244, 248, 247))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
                .rounding(16.0)
                .inner_margin(egui::Margin::same(18.0))
                .show(&mut columns[0], |ui| {
                    ui.set_min_height(340.0);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 10.0);

                    // Header row: Mint monitor icon + Title + Subtitle
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                        ui.painter().rect_filled(icon_rect, 10.0, Color32::from_rgb(205, 243, 232));
                        draw_monitor(ui.painter(), icon_rect.shrink(10.0), Color32::from_rgb(0, 122, 94), Color32::from_rgb(205, 243, 232));

                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Share This PC").size(17.0).strong().color(Color32::from_rgb(18, 36, 38)));
                            ui.label(
                                RichText::new("Give this code to the person who wants to connect to this PC.")
                                    .size(12.0)
                                    .color(Color32::from_rgb(95, 120, 122)),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Your Connection Code")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(25, 45, 48)),
                    );

                    // Large Code Box with Copy Button
                    egui::Frame::none()
                        .fill(Color32::from_rgb(220, 241, 235))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(188, 226, 216)))
                        .rounding(12.0)
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let id_formatted = self.config.device_id.formatted();
                                ui.label(
                                    RichText::new(&id_formatted)
                                        .font(FontId::proportional(26.0))
                                        .strong()
                                        .color(Color32::from_rgb(12, 38, 40)),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let is_copied = self.copied_time.map_or(false, |t| t.elapsed().as_secs() < 2);
                                    let copy_btn = ui.add_sized(
                                        [38.0, 36.0],
                                        egui::Button::new(
                                            RichText::new(if is_copied { "✔" } else { "📋" })
                                                .size(16.0)
                                                .color(if is_copied { Color32::from_rgb(0, 122, 94) } else { Color32::from_rgb(20, 48, 50) }),
                                        )
                                        .rounding(8.0)
                                        .fill(Color32::from_rgb(202, 234, 226)),
                                    );

                                    if copy_btn.clicked() {
                                        ctx.output_mut(|o| o.copied_text = self.config.device_id.formatted());
                                        self.copied_time = Some(Instant::now());
                                    }
                                });
                            });
                        });

                    // Network selector dropdown pill
                    egui::Frame::none()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(207, 222, 220)))
                        .rounding(8.0)
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                draw_wifi(ui.painter(), icon_rect.center(), 14.0, Color32::from_rgb(0, 122, 94));

                                ui.add_space(4.0);
                                egui::ComboBox::from_id_salt("net_mode_select")
                                    .selected_text(match self.network_mode {
                                        NetworkMode::LocalNetwork => "Same Wi-Fi / Local network",
                                        NetworkMode::RenderCloud => "Render Cloud Relay (Global)",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.network_mode,
                                            NetworkMode::LocalNetwork,
                                            "Same Wi-Fi / Local network",
                                        );
                                        ui.selectable_value(
                                            &mut self.network_mode,
                                            NetworkMode::RenderCloud,
                                            "Render Cloud Relay (Global)",
                                        );
                                    });
                            });
                        });

                    // Full Connection Code Box
                    egui::Frame::none()
                        .fill(Color32::from_rgb(238, 246, 244))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
                        .rounding(10.0)
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("🔗 Full Connection Code (for manual use)")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_rgb(40, 60, 62)),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let full_code = self.full_connection_code();
                                ui.add_sized(
                                    [ui.available_width() - 40.0, 44.0],
                                    egui::Label::new(
                                        RichText::new(&full_code)
                                            .size(10.5)
                                            .monospace()
                                            .color(Color32::from_rgb(30, 48, 50)),
                                    )
                                    .wrap(),
                                );

                                let is_full_copied = self.full_copied_time.map_or(false, |t| t.elapsed().as_secs() < 2);
                                let copy_full_btn = ui.add_sized(
                                    [34.0, 34.0],
                                    egui::Button::new(
                                        RichText::new(if is_full_copied { "✔" } else { "📋" })
                                            .size(14.0)
                                            .color(if is_full_copied { Color32::from_rgb(0, 122, 94) } else { Color32::from_rgb(20, 48, 50) }),
                                    )
                                    .rounding(6.0)
                                    .fill(Color32::from_rgb(222, 238, 234)),
                                );
                                if copy_full_btn.clicked() {
                                    ctx.output_mut(|o| o.copied_text = full_code);
                                    self.full_copied_time = Some(Instant::now());
                                }
                            });
                        });
                });

            // CARD 2: Connect to Another PC
            egui::Frame::none()
                .fill(Color32::from_rgb(244, 248, 247))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
                .rounding(16.0)
                .inner_margin(egui::Margin::same(18.0))
                .show(&mut columns[1], |ui| {
                    ui.set_min_height(340.0);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 16.0);

                    // Header row: Blue monitor+arrow icon + Title + Subtitle
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                        ui.painter().rect_filled(icon_rect, 10.0, Color32::from_rgb(219, 234, 251));
                        draw_monitor_arrow(ui.painter(), icon_rect.shrink(10.0), Color32::from_rgb(30, 96, 181));

                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Connect to Another PC").size(17.0).strong().color(Color32::from_rgb(18, 36, 38)));
                            ui.label(
                                RichText::new("Enter the connection code of the other PC.")
                                    .size(12.0)
                                    .color(Color32::from_rgb(95, 120, 122)),
                            );
                        });
                    });

                    ui.add_space(14.0);

                    // Input Box with embedded Monitor Icon
                    egui::Frame::none()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.2_f32, Color32::from_rgb(207, 222, 220)))
                        .rounding(10.0)
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 16.0), Sense::hover());
                                draw_monitor(ui.painter(), icon_rect, Color32::from_rgb(120, 140, 142), Color32::WHITE);

                                ui.add_space(8.0);
                                let text_edit = ui.add_sized(
                                    [ui.available_width(), 26.0],
                                    egui::TextEdit::singleline(&mut self.remote_id_input)
                                        .hint_text("Enter 9-digit code (e.g. 123 456 789)")
                                        .font(FontId::proportional(14.5))
                                        .text_color(Color32::from_rgb(20, 36, 38))
                                        .frame(false),
                                );

                                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    self.start_connection();
                                }
                            });
                        });

                    let can_connect = self.is_online
                        && self.remote_id_input.chars().filter(|c| c.is_ascii_digit()).count() == 9;

                    // Large Emerald Green Action Button: "➔ Connect to PC"
                    let connect_btn = ui.add_sized(
                        [ui.available_width(), 46.0],
                        egui::Button::new(
                            RichText::new("➔   Connect to PC")
                                .size(15.5)
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .rounding(10.0)
                        .fill(if can_connect {
                            Color32::from_rgb(0, 119, 88)
                        } else {
                            Color32::from_rgb(125, 155, 148)
                        }),
                    );

                    if connect_btn.clicked() && can_connect {
                        self.start_connection();
                    }

                    // Status display feedback
                    if let Some(ref status) = self.connect_status {
                        ui.label(
                            RichText::new(status)
                                .size(12.5)
                                .color(Color32::from_rgb(200, 110, 20))
                                .strong(),
                        );
                    }
                });
        });

        ui.add_space(6.0);

        // Bottom Trust Banner: "Ready when you are" + 3 Feature Badges
        egui::Frame::none()
            .fill(Color32::from_rgb(244, 248, 247))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
            .rounding(16.0)
            .inner_margin(egui::Margin::symmetric(20.0, 14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left banner: Shield check + Text
                    ui.scope(|ui| {
                        ui.set_width(ui.available_width() - 380.0);
                        ui.horizontal(|ui| {
                            let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                            ui.painter().circle_filled(badge_rect.center(), 20.0, Color32::from_rgb(205, 243, 232));
                            draw_shield_check(ui.painter(), badge_rect.center(), 20.0, Color32::from_rgb(0, 122, 94));

                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Ready when you are").size(15.0).strong().color(Color32::from_rgb(18, 36, 38)));
                                ui.label(
                                    RichText::new("Your screen is private until you approve a connection request.")
                                        .size(12.0)
                                        .color(Color32::from_rgb(95, 120, 122)),
                                );
                            });
                        });
                    });

                    // Right 3 Trust Badges with separators
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Badge 3: No Account Needed
                        ui.vertical_centered(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            draw_lightning(ui.painter(), rect.center(), 18.0, Color32::from_rgb(0, 140, 105));
                            ui.label(RichText::new("No Account\nNeeded").size(11.0).strong().color(Color32::from_rgb(30, 48, 50)));
                        });

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(14.0);

                        // Badge 2: Approval Every Time
                        ui.vertical_centered(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            draw_user_check(ui.painter(), rect.center(), 20.0, Color32::from_rgb(0, 140, 105));
                            ui.label(RichText::new("Approval\nEvery Time").size(11.0).strong().color(Color32::from_rgb(30, 48, 50)));
                        });

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(14.0);

                        // Badge 1: Encrypted Session
                        ui.vertical_centered(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            draw_lock(ui.painter(), rect.center(), 18.0, Color32::from_rgb(0, 140, 105));
                            ui.label(RichText::new("Encrypted\nSession").size(11.0).strong().color(Color32::from_rgb(30, 48, 50)));
                        });
                    });
                });
            });
    }

    /// Renders the Settings View.
    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Settings & Relay Configuration").size(22.0).strong().color(Color32::WHITE));
        ui.add_space(12.0);

        egui::Frame::none()
            .fill(Color32::from_rgb(244, 248, 247))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
            .rounding(16.0)
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 14.0);

                ui.label(RichText::new("Signaling & Relay Server URL").size(15.0).strong().color(Color32::from_rgb(18, 36, 38)));
                ui.label(
                    RichText::new("Specify your hosted Render relay or local development server for cross-network connectivity:")
                        .size(12.5)
                        .color(Color32::from_rgb(95, 120, 122)),
                );

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 140.0, 36.0],
                        egui::TextEdit::singleline(&mut self.server_url_input)
                            .font(FontId::monospace(13.0))
                            .hint_text("wss://<your-app>.onrender.com"),
                    );

                    let save_btn = ui.add_sized(
                        [130.0, 36.0],
                        egui::Button::new(RichText::new("Save & Reconnect").color(Color32::WHITE).strong())
                            .rounding(8.0)
                            .fill(Color32::from_rgb(0, 119, 88)),
                    );

                    if save_btn.clicked() {
                        let new_url = self.server_url_input.trim().to_string();
                        if !new_url.is_empty() {
                            self.config.signaling_server_url = new_url.clone();
                            let _ = self.config.save();

                            let (signaling, events_rx) = SignalingClient::connect(
                                new_url.clone(),
                                self.config.device_id.as_str().to_string(),
                            );
                            self.signaling = signaling;
                            self.events_rx = events_rx;
                            self.is_online = false;
                            self.connect_status = Some(format!("Connecting to {}", new_url));
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Quick Server Presets:").size(13.0).strong().color(Color32::from_rgb(18, 36, 38)));
                ui.horizontal(|ui| {
                    if ui.button("🌐 Render Cloud Server").clicked() {
                        self.server_url_input = "wss://remotelite-server.onrender.com".to_string();
                    }
                    if ui.button("🏠 Localhost Server (ws://127.0.0.1:9090)").clicked() {
                        self.server_url_input = "ws://127.0.0.1:9090".to_string();
                    }
                });

                ui.separator();

                ui.label(RichText::new("Streaming Quality & Performance").size(15.0).strong().color(Color32::from_rgb(18, 36, 38)));
                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    ui.add(egui::Slider::new(&mut self.config.target_fps, 15..=60).text("FPS"));
                });

                ui.horizontal(|ui| {
                    ui.label("JPEG Compression Quality:");
                    ui.add(egui::Slider::new(&mut self.config.jpeg_quality, 30..=90).text("%"));
                });
            });
    }

    /// Renders the About View.
    fn render_about_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("About InstantLink").size(22.0).strong().color(Color32::WHITE));
        ui.add_space(12.0);

        egui::Frame::none()
            .fill(Color32::from_rgb(244, 248, 247))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 233, 230)))
            .rounding(16.0)
            .inner_margin(egui::Margin::same(24.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 12.0);

                ui.horizontal(|ui| {
                    let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                    ui.painter().circle_filled(badge_rect.center(), 24.0, Color32::from_rgb(0, 229, 163));
                    draw_lightning(ui.painter(), badge_rect.center(), 24.0, Color32::from_rgb(11, 35, 38));

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("InstantLink").size(20.0).strong().color(Color32::from_rgb(18, 36, 38)));
                        ui.label(RichText::new("Version 0.1.0 — Basic Edition").size(13.0).color(Color32::from_rgb(95, 120, 122)));
                    });
                });

                ui.separator();

                ui.label(RichText::new("High-performance, lightweight Windows remote desktop engineered with Rust.").size(14.0).color(Color32::from_rgb(30, 48, 50)));
                ui.label("• End-to-end ChaCha20-Poly1305 encryption on all screen and input packets.");
                ui.label("• Explicit user approval required for every connection.");
                ui.label("• Zero-account architecture — no tracking, no subscription, no background daemons.");
                ui.label("• Windows GDI desktop capture with adaptive JPEG compression.");

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Source Code & Releases:");
                    ui.hyperlink_to("github.com/farhanisthis/RemoteLite", "https://github.com/farhanisthis/RemoteLite");
                });
            });
    }

    /// Active Host Screen Sharing Banner (shown when another PC is controlling this PC).
    fn render_active_host_banner(&mut self, ui: &mut egui::Ui) {
        let host_controller_id = self.host_session.as_ref().map(|h| h.controller_id.clone());
        if let Some(ref controller_id) = host_controller_id {
            let mut should_stop_host = false;
            egui::Frame::none()
                .fill(Color32::from_rgb(180, 40, 40))
                .rounding(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("🔴 REMOTE CONTROL ACTIVE — Controlled by ")
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(controller_id)
                                .color(Color32::WHITE)
                                .heading(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    RichText::new("Stop Sharing / Disconnect")
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .clicked()
                            {
                                should_stop_host = true;
                            }
                        });
                    });
                });

            if should_stop_host {
                if let Some(ref mut h) = self.host_session {
                    h.stop("Host terminated session");
                }
                self.host_session = None;
            }
            ui.add_space(8.0);
        }
    }

    /// Incoming Connection Approval Modal.
    fn render_incoming_modal(&mut self, ui: &mut egui::Ui) {
        let incoming_from_id = self.incoming_request.as_ref().map(|r| r.from_id.clone());
        if let Some(ref from_id) = incoming_from_id {
            let mut accept_clicked = false;
            let mut reject_clicked = false;

            egui::Frame::none()
                .fill(Color32::from_rgb(244, 248, 247))
                .stroke(Stroke::new(2.0_f32, Color32::from_rgb(0, 180, 130)))
                .rounding(14.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("🔔 Incoming Connection Request")
                                .size(20.0)
                                .strong()
                                .color(Color32::from_rgb(0, 122, 94)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "Device ID {} requests permission to view and control your desktop.",
                                from_id
                            ))
                            .size(14.0)
                            .color(Color32::from_rgb(30, 48, 50)),
                        );
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let accept_btn = ui.add_sized(
                                [140.0, 38.0],
                                egui::Button::new(
                                    RichText::new("✔ Accept")
                                        .size(15.0)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .rounding(8.0)
                                .fill(Color32::from_rgb(0, 119, 88)),
                            );
                            if accept_btn.clicked() {
                                accept_clicked = true;
                            }

                            ui.add_space(16.0);

                            let reject_btn = ui.add_sized(
                                [140.0, 38.0],
                                egui::Button::new(
                                    RichText::new("✖ Reject")
                                        .size(15.0)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .rounding(8.0)
                                .fill(Color32::from_rgb(200, 50, 50)),
                            );
                            if reject_btn.clicked() {
                                reject_clicked = true;
                            }
                        });
                    });
                });

            if accept_clicked {
                self.accept_incoming_request();
            } else if reject_clicked {
                self.reject_incoming_request();
            }

            ui.add_space(10.0);
        }
    }

    /// Renders the active Remote Desktop Viewer session.
    fn render_viewer(&mut self, ctx: &egui::Context) {
        // TOP FLOATING TOOLBAR
        egui::TopBottomPanel::top("viewer_toolbar")
            .frame(egui::Frame::none().fill(Color32::from_rgb(11, 35, 38)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                    draw_lightning(ui.painter(), badge_rect.center(), 18.0, Color32::from_rgb(0, 229, 163));

                    ui.label(RichText::new("InstantLink Viewer").color(Color32::WHITE).strong());
                    ui.separator();

                    if let Some(ref v) = self.viewer_session {
                        ui.label(RichText::new(format!("Remote ID: {}", v.host_id)).color(Color32::from_rgb(0, 229, 163)));
                        ui.separator();
                        ui.label(RichText::new(format!("{}x{}", self.remote_resolution.0, self.remote_resolution.1)).color(Color32::from_rgb(180, 200, 200)));
                        ui.separator();
                        ui.label(RichText::new(format!("{} ms", self.latency_ms)).color(Color32::from_rgb(180, 200, 200)));
                        ui.separator();
                        ui.label(RichText::new(format!("{:.1} FPS", v.fps())).color(Color32::from_rgb(180, 200, 200)));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let disconnect_btn = ui.add_sized(
                            [120.0, 28.0],
                            egui::Button::new(RichText::new("Disconnect").color(Color32::WHITE).strong())
                                .rounding(6.0)
                                .fill(Color32::from_rgb(200, 40, 40)),
                        );

                        if disconnect_btn.clicked() {
                            if let Some(ref mut viewer) = self.viewer_session {
                                viewer.disconnect("User clicked Disconnect");
                            }
                            self.viewer_session = None;
                            self.viewer_texture = None;
                            self.connect_status = Some("Session disconnected".to_string());
                        }
                    });
                });
            });

        // CENTRAL DISPLAY CANVAS
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let available_size = ui.available_size();
                let (remote_w, remote_h) = (
                    self.remote_resolution.0 as f32,
                    self.remote_resolution.1 as f32,
                );

                let fitted = CoordinateMapper::calculate_fitted_rect(
                    available_size.x,
                    available_size.y,
                    remote_w,
                    remote_h,
                );

                let display_rect = Rect::from_min_size(
                    Pos2::new(ui.min_rect().min.x + fitted.x, ui.min_rect().min.y + fitted.y),
                    Vec2::new(fitted.width, fitted.height),
                );

                // Draw remote screen texture
                if let Some(ref tex) = self.viewer_texture {
                    ui.painter().image(
                        tex.id(),
                        display_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        display_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Waiting for remote desktop screen frames...",
                        FontId::proportional(18.0),
                        Color32::from_rgb(160, 160, 160),
                    );
                }

                // Interactive input area
                let response = ui.allocate_rect(display_rect, Sense::click_and_drag());

                // Mouse Input Handling
                if let Some(hover_pos) = response.hover_pos() {
                    let rel_pos = (
                        hover_pos.x - display_rect.min.x,
                        hover_pos.y - display_rect.min.y,
                    );
                    let norm_pos = (
                        (rel_pos.0 / display_rect.width()).clamp(0.0, 1.0),
                        (rel_pos.1 / display_rect.height()).clamp(0.0, 1.0),
                    );

                    if self.last_mouse_norm != Some(norm_pos) {
                        self.last_mouse_norm = Some(norm_pos);
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseMove {
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                        }
                    }

                    if response.clicked() {
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseDown {
                                button: MouseButton::Left,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                            let _ = viewer.send_input(InputPayload::MouseUp {
                                button: MouseButton::Left,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                        }
                    }

                    if response.secondary_clicked() {
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseDown {
                                button: MouseButton::Right,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                            let _ = viewer.send_input(InputPayload::MouseUp {
                                button: MouseButton::Right,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                        }
                    }

                    if response.middle_clicked() {
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseDown {
                                button: MouseButton::Middle,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                            let _ = viewer.send_input(InputPayload::MouseUp {
                                button: MouseButton::Middle,
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                        }
                    }

                    let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
                    if scroll_delta.y.abs() > 0.1 {
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseScroll {
                                delta_x: scroll_delta.x,
                                delta_y: scroll_delta.y / 100.0,
                            });
                        }
                    }
                }

                // Keyboard Input Handling
                ctx.input(|i| {
                    for event in &i.events {
                        match event {
                            egui::Event::Key {
                                key,
                                pressed,
                                repeat: false,
                                ..
                            } => {
                                if let Some(vk) = egui_key_to_vk(*key) {
                                    if let Some(ref viewer) = self.viewer_session {
                                        if *pressed {
                                            let _ = viewer.send_input(InputPayload::KeyDown {
                                                virtual_key: vk,
                                                scancode: 0,
                                                text: None,
                                            });
                                        } else {
                                            let _ = viewer.send_input(InputPayload::KeyUp {
                                                virtual_key: vk,
                                                scancode: 0,
                                            });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                });
            });
    }
}
