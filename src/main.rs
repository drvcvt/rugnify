use anyhow::Result;
use image::{Rgba, RgbaImage};
use pixels::{Pixels, SurfaceTexture};
use rayon::prelude::*;
use screenshots::Screen;
use std::collections::HashSet;
use winit::dpi::PhysicalSize;
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, WindowBuilder};

// Plattformspezifische Erweiterung für X11, um den Fenstertyp zu setzen
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use winit::platform::x11::{WindowBuilderExtX11, XWindowType};

const BRUSH_SIZE: i32 = 5;
const FOCUS_RADIUS: f64 = 125.0;

/// Repräsentiert den Anwendungszustand.
struct App {
    original_image: RgbaImage,
    leinwand: RgbaImage,
    drawn_lines: Vec<Vec<(u32, u32)>>,
    current_stroke: HashSet<(u32, u32)>, // Für die aktuelle, unfertige Linie
    zoom: f32,
    offset: (f32, f32),
    target_zoom: f32,
    target_offset: (f32, f32),
    smoothing_factor: f32,
    last_mouse_pos: (f64, f64),
    // Zustände für Modi
    is_panning: bool,
    is_drawing_mode: bool,
    is_drawing: bool,
    is_erasing: bool,
    is_alt_pressed: bool,
    last_paint_pos: Option<(f64, f64)>,
}

impl App {
    fn new(image: RgbaImage) -> Self {
        let leinwand = image.clone();
        Self {
            original_image: image,
            leinwand,
            drawn_lines: Vec::new(),
            current_stroke: HashSet::new(),
            zoom: 1.0,
            offset: (0.0, 0.0),
            target_zoom: 1.0,
            target_offset: (0.0, 0.0),
            smoothing_factor: 0.2, // Faktor für die "Glätte" der Animation
            last_mouse_pos: (0.0, 0.0),
            is_panning: false,
            is_drawing_mode: false,
            is_drawing: false,
            is_erasing: false,
            is_alt_pressed: false,
            last_paint_pos: None,
        }
    }

    /// Behandelt alle Eingaben und aktualisiert die Zielwerte für die Animation.
    fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };

                let old_target_zoom = self.target_zoom;
                self.target_zoom *= 1.0 + scroll * 0.1;
                self.target_zoom = self.target_zoom.clamp(0.1, 10.0);

                let (mx, my) = (self.last_mouse_pos.0 as f32, self.last_mouse_pos.1 as f32);
                // Passe das Ziel-Offset an, sodass der Punkt unter der Maus fixiert bleibt
                self.target_offset.0 += (mx / old_target_zoom) - (mx / self.target_zoom);
                self.target_offset.1 += (my / old_target_zoom) - (my / self.target_zoom);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                if self.is_drawing_mode {
                    match button {
                        MouseButton::Left => self.is_drawing = pressed,
                        MouseButton::Right => self.is_erasing = pressed,
                        _ => (),
                    }
                    if pressed {
                        self.last_paint_pos = Some(self.last_mouse_pos);
                        if self.is_erasing {
                            self.erase_at(self.last_mouse_pos.0, self.last_mouse_pos.1);
                        }
                    } else {
                        // Maustaste losgelassen: Strich auf Leinwand "einbrennen"
                        if !self.current_stroke.is_empty() {
                            for &(x, y) in &self.current_stroke {
                                self.leinwand.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                            }
                            self.drawn_lines.push(self.current_stroke.drain().collect());
                        }
                        self.last_paint_pos = None;
                    }
                } else if *button == MouseButton::Left {
                    self.is_panning = pressed;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (new_x, new_y) = (position.x, position.y);
                if self.is_panning {
                    let (dx, dy) = (new_x - self.last_mouse_pos.0, new_y - self.last_mouse_pos.1);
                    self.target_offset.0 -= (dx / self.zoom as f64) as f32;
                    self.target_offset.1 -= (dy / self.zoom as f64) as f32;
                } else if self.is_drawing {
                    if let Some(last_pos) = self.last_paint_pos {
                        self.paint_line(last_pos, (new_x, new_y));
                    }
                    self.last_paint_pos = Some((new_x, new_y));
                } else if self.is_erasing {
                    self.erase_at(new_x, new_y);
                }
                self.last_mouse_pos = (new_x, new_y);
            }

            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                match keycode {
                    VirtualKeyCode::LControl => {
                        if *state == ElementState::Pressed {
                            self.is_drawing_mode = !self.is_drawing_mode;
                        }
                    }
                    VirtualKeyCode::LAlt => self.is_alt_pressed = *state == ElementState::Pressed,
                    _ => (),
                }
            }
            _ => (),
        }
    }
    
    /// Aktualisiert den Zustand für flüssige Animationen (wird in jedem Frame aufgerufen).
    fn update(&mut self) {
        // Berechne den Unterschied zwischen Ziel und aktuellem Zustand
        let zoom_diff = self.target_zoom - self.zoom;
        let offset_x_diff = self.target_offset.0 - self.offset.0;
        let offset_y_diff = self.target_offset.1 - self.offset.1;

        // Wenn der Unterschied sehr gering ist, nicht weiter interpolieren, um "Driften" zu stoppen
        if zoom_diff.abs() < 0.001
            && offset_x_diff.abs() < 0.001
            && offset_y_diff.abs() < 0.001
        {
            self.zoom = self.target_zoom;
            self.offset = self.target_offset;
            return;
        }

        // Interpoliere Zoom und Offset für eine weiche Bewegung
        self.zoom += zoom_diff * self.smoothing_factor;
        self.offset.0 += offset_x_diff * self.smoothing_factor;
        self.offset.1 += offset_y_diff * self.smoothing_factor;
    }

    /// Löscht eine Linie, die vom Radierer berührt wird, und regeneriert die Leinwand.
    fn erase_at(&mut self, screen_x: f64, screen_y: f64) {
        let brush_radius = (BRUSH_SIZE as f32 / self.zoom).max(1.0);
        let img_center_x = screen_x as f32 / self.zoom + self.offset.0;
        let img_center_y = screen_y as f32 / self.zoom + self.offset.1;

        let initial_line_count = self.drawn_lines.len();

        self.drawn_lines.retain(|line| {
            !line.iter().any(|(px, py)| {
                let dist_sq =
                    (*px as f32 - img_center_x).powi(2) + (*py as f32 - img_center_y).powi(2);
                dist_sq <= brush_radius.powi(2)
            })
        });

        // Wenn Linien entfernt wurden, regeneriere die Leinwand für korrekte Überlappungen.
        if self.drawn_lines.len() < initial_line_count {
            self.leinwand = self.original_image.clone();
            for line in &self.drawn_lines {
                for &(x, y) in line {
                    self.leinwand.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
    }

    /// Zeichnet eine durchgehende Linie zwischen zwei Punkten.
    fn paint_line(&mut self, start: (f64, f64), end: (f64, f64)) {
        let (x0, y0) = start;
        let (x1, y1) = end;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let mut err = dx + dy;

        let mut current_x = x0;
        let mut current_y = y0;

        let sx = if x0 < x1 { 1.0 } else { -1.0 };
        let sy = if y0 < y1 { 1.0 } else { -1.0 };

        loop {
            self.add_brush_points(current_x, current_y);
            if (current_x - x1).abs() < 1.0 && (current_y - y1).abs() < 1.0 {
                break;
            }
            let e2 = 2.0 * err;
            if e2 >= dy {
                err += dy;
                current_x += sx;
            }
            if e2 <= dx {
                err += dx;
                current_y += sy;
            }
        }
    }

    /// Fügt die Punkte eines runden Pinsels zum aktuellen Stroke hinzu.
    fn add_brush_points(&mut self, screen_x: f64, screen_y: f64) {
        let brush_radius = (BRUSH_SIZE as f32 / self.zoom).max(1.0);
        let img_center_x = screen_x as f32 / self.zoom + self.offset.0;
        let img_center_y = screen_y as f32 / self.zoom + self.offset.1;
        
        let start_x = (img_center_x - brush_radius).floor() as i32;
        let end_x = (img_center_x + brush_radius).ceil() as i32;
        let start_y = (img_center_y - brush_radius).floor() as i32;
        let end_y = (img_center_y + brush_radius).ceil() as i32;

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                let dist_sq = (x as f32 - img_center_x).powi(2) + (y as f32 - img_center_y).powi(2);
                if dist_sq <= brush_radius.powi(2) {
                    if x >= 0
                        && y >= 0
                        && x < self.original_image.width() as i32
                        && y < self.original_image.height() as i32
                    {
                        self.current_stroke.insert((x as u32, y as u32));
                    }
                }
            }
        }
    }
    
    /// Zeichnet den Frame. Diese Methode ist jetzt hochperformant.
    fn draw(&self, pixels: &mut Pixels, frame_width: u32, _frame_height: u32) {
        // Erstelle ein temporäres Bild für diese Frame-Anzeige, um den langsamen HashSet-Lookup zu vermeiden.
        let mut display_image = self.leinwand.clone();

        // Zeichne den aktuellen, unfertigen Strich auf das temporäre Bild.
        for &(x, y) in &self.current_stroke {
            display_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }

        let frame = pixels.frame_mut();
        let (img_width, img_height) = display_image.dimensions();

        frame
            .par_chunks_mut(4)
            .enumerate()
            .for_each(|(i, pixel)| {
                let screen_x = (i % frame_width as usize) as u32;
                let screen_y = (i / frame_width as usize) as u32;

                let source_x_f = screen_x as f32 / self.zoom + self.offset.0;
                let source_y_f = screen_y as f32 / self.zoom + self.offset.1;

                let mut color = [0x40, 0x40, 0x40, 0xff];

                if source_x_f >= 0.0
                    && source_x_f < img_width as f32
                    && source_y_f >= 0.0
                    && source_y_f < img_height as f32
                {
                    let source_x = source_x_f as u32;
                    let source_y = source_y_f as u32;

                    // Nimm den Pixel direkt vom vorbereiteten Anzeigebild. Dies ist viel schneller.
                    color.copy_from_slice(&display_image.get_pixel(source_x, source_y).0);
                }

                if self.is_alt_pressed {
                    let dist_sq = (screen_x as f64 - self.last_mouse_pos.0).powi(2)
                        + (screen_y as f64 - self.last_mouse_pos.1).powi(2);
                    if dist_sq > FOCUS_RADIUS.powi(2) {
                        color[0] = (color[0] as f32 * 0.25) as u8;
                        color[1] = (color[1] as f32 * 0.25) as u8;
                        color[2] = (color[2] as f32 * 0.25) as u8;
                    }
                }

                pixel.copy_from_slice(&color);
            });
    }
}

fn main() -> Result<()> {
    let screens = Screen::all()?;
    let primary_screen = screens.get(0).ok_or_else(|| anyhow::anyhow!("Konnte keinen Bildschirm finden"))?;
    let image_buffer = primary_screen.capture()?;
    let (width, height) = image_buffer.dimensions();

    let event_loop = EventLoop::new();
    
    // Erstelle den WindowBuilder veränderbar, um plattformspezifische Optionen hinzuzufügen
    let mut builder = WindowBuilder::new()
        .with_decorations(false)
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_inner_size(PhysicalSize::new(width, height));

    // Setze den X11-Fenstertyp, um "floating" zu erzwingen
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    {
        builder = builder.with_x11_window_type(vec![XWindowType::Dialog]);
    }
    
    let window = builder.build(&event_loop)?;

    window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
    window.set_cursor_visible(true);

    let mut app = App::new(image_buffer);

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll; // Auf Poll umstellen für kontinuierliches Rendern

        match event {
            Event::WindowEvent {
                event: win_event, ..
            } => {
                app.input(&win_event);

                match win_event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        if let Err(e) = pixels.resize_surface(size.width, size.height) {
                            eprintln!("Fehler beim Ändern der Fenstergröße: {}", e);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    _ => (),
                }
            }

            Event::MainEventsCleared => {
                app.update(); // Update-Logik für Animationen aufrufen
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let size = window.inner_size();
                app.draw(&mut pixels, size.width, size.height);
                if let Err(e) = pixels.render() {
                    eprintln!("Fehler beim Rendern: {}", e);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}
