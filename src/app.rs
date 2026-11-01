use crate::config::AppConfig;
use crate::crypto::generate_entropy;
use crate::input::mapper::{egui_key_to_vk, CoordinateMapper};
use crate::net::{HostSession, SignalingClient, SignalingEvent, ViewerSession};
use crate::protocol::{InputPayload, MouseButton};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::time::Instant;
use tokio::sync::mpsc;

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
    remote_id_input: String,
    connect_status: Option<String>,
    pending_connect: Option<(String, String, [u8; 32])>, // (session_id, target_id, entropy)

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

    // Server configuration panel
    show_settings: bool,
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
            show_settings: false,
            server_url_input,
        }
    }

    /// Process asynchronous network events.
    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                SignalingEvent::Connected => {
                    self.is_online = true;
                    ctx.request_repaint();
                }
                SignalingEvent::Registered { .. } => {
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
}

impl eframe::App for RemoteLiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);

        if self.viewer_session.is_some() {
            ctx.request_repaint();
            self.render_viewer(ctx);
        } else {
            self.render_dashboard(ctx);
        }
    }
}

impl RemoteLiteApp {
    /// Render the main dashboard (Your Device, Remote Device, Incoming Request, Active Host Banner).
    fn render_dashboard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 14.0);

            // Active Host Session Banner (visible when remote control of this PC is active)
            let host_controller_id = self.host_session.as_ref().map(|h| h.controller_id.clone());
            if let Some(ref controller_id) = host_controller_id {
                let mut should_stop_host = false;
                ui.scope(|ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(180, 40, 40))
                        .rounding(8.0)
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
                });
                if should_stop_host {
                    if let Some(ref mut h) = self.host_session {
                        h.stop("Host terminated session");
                    }
                    self.host_session = None;
                }
                ui.add_space(8.0);
            }

            // Incoming Connection Request Modal / Card
            let incoming_from_id = self.incoming_request.as_ref().map(|r| r.from_id.clone());
            if let Some(ref from_id) = incoming_from_id {
                let mut accept_clicked = false;
                let mut reject_clicked = false;

                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 45, 60))
                    .stroke(Stroke::new(2.0_f32, Color32::from_rgb(80, 150, 240)))
                    .rounding(10.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("🔔 Incoming Connection Request")
                                    .size(20.0)
                                    .strong()
                                    .color(Color32::from_rgb(100, 200, 255)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!(
                                    "Device ID {} requests permission to view and control your desktop.",
                                    from_id
                                ))
                                .size(14.0),
                            );
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                let accept_btn = ui.add_sized(
                                    [140.0, 36.0],
                                    egui::Button::new(
                                        RichText::new("✔ Accept")
                                            .size(15.0)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(34, 150, 75)),
                                );
                                if accept_btn.clicked() {
                                    accept_clicked = true;
                                }

                                ui.add_space(16.0);

                                let reject_btn = ui.add_sized(
                                    [140.0, 36.0],
                                    egui::Button::new(
                                        RichText::new("✖ Reject")
                                            .size(15.0)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
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

                ui.add_space(12.0);
            }

            // App Header
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("RemoteLite")
                        .size(26.0)
                        .strong()
                        .color(Color32::from_rgb(230, 230, 240)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.show_settings { "✖ Close" } else { "⚙ Settings" }).clicked() {
                        self.show_settings = !self.show_settings;
                    }

                    ui.add_space(8.0);

                    if self.is_online {
                        ui.label(RichText::new("● Online").color(Color32::from_rgb(60, 200, 90)).strong());
                    } else {
                        ui.label(RichText::new("● Connecting to relay...").color(Color32::from_rgb(230, 160, 40)));
                    }
                });
            });

            // Collapsible Settings Panel
            if self.show_settings {
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(Color32::from_rgb(28, 34, 44))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 80, 110)))
                    .rounding(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Relay & Signaling Server Configuration").strong());
                        ui.label(
                            RichText::new("Connect across different networks by entering your public server (e.g. Render, Railway, or VPS):")
                                .small()
                                .color(Color32::from_rgb(160, 170, 180)),
                        );
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("URL:");
                            ui.add_sized(
                                [ui.available_width() - 140.0, 28.0],
                                egui::TextEdit::singleline(&mut self.server_url_input)
                                    .hint_text("wss://your-service.onrender.com"),
                            );

                            if ui.add_sized([120.0, 28.0], egui::Button::new(RichText::new("Save & Connect").strong())).clicked() {
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
                                    self.show_settings = false;
                                }
                            }
                        });
                    });
            }

            ui.separator();
            ui.add_space(10.0);

            // Two main columns: "Your Device" and "Remote Device"
            ui.columns(2, |columns| {
                // COLUMN 1: Your Device
                columns[0].group(|ui| {
                    ui.set_min_height(240.0);
                    ui.label(RichText::new("Your Device").size(16.0).strong().color(Color32::from_rgb(180, 180, 200)));
                    ui.add_space(8.0);

                    ui.label("Your ID:");
                    let id_formatted = self.config.device_id.formatted();
                    ui.label(
                        RichText::new(&id_formatted)
                            .font(FontId::monospace(28.0))
                            .strong()
                            .color(Color32::from_rgb(90, 180, 255)),
                    );

                    ui.add_space(10.0);
                    let copy_label = if let Some(t) = self.copied_time {
                        if t.elapsed().as_secs() < 2 {
                            "✔ Copied!"
                        } else {
                            "Copy ID"
                        }
                    } else {
                        "Copy ID"
                    };

                    if ui.add_sized([120.0, 32.0], egui::Button::new(copy_label)).clicked() {
                        ctx.output_mut(|o| o.copied_text = self.config.device_id.formatted());
                        self.copied_time = Some(Instant::now());
                    }

                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("Status: Ready for Connections")
                            .color(Color32::from_rgb(120, 200, 130))
                            .small(),
                    );
                });

                // COLUMN 2: Remote Device
                columns[1].group(|ui| {
                    ui.set_min_height(240.0);
                    ui.label(RichText::new("Remote Device").size(16.0).strong().color(Color32::from_rgb(180, 180, 200)));
                    ui.add_space(8.0);

                    ui.label("Enter Remote ID (9 digits):");
                    let text_edit = ui.add_sized(
                        [ui.available_width(), 34.0],
                        egui::TextEdit::singleline(&mut self.remote_id_input)
                            .font(FontId::monospace(18.0))
                            .hint_text("e.g. 123 456 789"),
                    );

                    if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.start_connection();
                    }

                    ui.add_space(12.0);

                    let can_connect = self.is_online
                        && self.remote_id_input.chars().filter(|c| c.is_ascii_digit()).count() == 9;

                    let connect_btn = ui.add_enabled(
                        can_connect,
                        egui::Button::new(
                            RichText::new("Connect")
                                .size(16.0)
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .min_size(Vec2::new(ui.available_width(), 38.0))
                        .fill(if can_connect {
                            Color32::from_rgb(40, 120, 220)
                        } else {
                            Color32::from_rgb(60, 70, 80)
                        }),
                    );

                    if connect_btn.clicked() {
                        self.start_connection();
                    }

                    if let Some(ref status) = self.connect_status {
                        ui.add_space(10.0);
                        ui.label(RichText::new(status).color(Color32::from_rgb(240, 200, 80)));
                    }
                });
            });
        });
    }

    /// Render the active remote desktop viewer.
    fn render_viewer(&mut self, ctx: &egui::Context) {
        // TOP TOOLBAR
        egui::TopBottomPanel::top("viewer_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref v) = self.viewer_session {
                    ui.label(RichText::new(format!("Connected to: {}", v.host_id)).strong());
                    ui.separator();
                    ui.label(format!("Resolution: {}x{}", self.remote_resolution.0, self.remote_resolution.1));
                    ui.separator();
                    ui.label(format!("Latency: {} ms", self.latency_ms));
                    ui.separator();
                    ui.label(format!("FPS: {:.1}", v.fps()));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let disconnect_btn = ui.add_sized(
                        [120.0, 28.0],
                        egui::Button::new(RichText::new("Disconnect").color(Color32::WHITE).strong())
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

                // Draw the remote screen texture if available
                if let Some(ref tex) = self.viewer_texture {
                    ui.painter().image(
                        tex.id(),
                        display_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Loading placeholder
                    ui.painter().text(
                        display_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Waiting for remote desktop screen frames...",
                        FontId::proportional(18.0),
                        Color32::from_rgb(160, 160, 160),
                    );
                }

                // Interactive input area covering display_rect
                let response = ui.allocate_rect(display_rect, Sense::click_and_drag());

                // MOUSE INPUT HANDLING
                if let Some(hover_pos) = response.hover_pos() {
                    let rel_pos = (
                        hover_pos.x - display_rect.min.x,
                        hover_pos.y - display_rect.min.y,
                    );
                    let norm_pos = (
                        (rel_pos.0 / display_rect.width()).clamp(0.0, 1.0),
                        (rel_pos.1 / display_rect.height()).clamp(0.0, 1.0),
                    );

                    // Send mouse move if position changed
                    if self.last_mouse_norm != Some(norm_pos) {
                        self.last_mouse_norm = Some(norm_pos);
                        if let Some(ref viewer) = self.viewer_session {
                            let _ = viewer.send_input(InputPayload::MouseMove {
                                norm_x: norm_pos.0,
                                norm_y: norm_pos.1,
                            });
                        }
                    }

                    // Mouse clicks
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

                    // Scroll wheel
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

                // KEYBOARD INPUT FORWARDING
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
