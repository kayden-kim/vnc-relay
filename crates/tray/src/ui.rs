//! All tray and muda objects are created, changed and dropped on this thread.

use crate::copy::{Language, Text};
use crate::worker::{self, Action, Reply, Request, Snapshot};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

struct Items {
    menu: Menu,
    status: MenuItem,
    config_error: Option<MenuItem>,
    failure: Option<MenuItem>,
    start: MenuItem,
    stop: MenuItem,
    dashboard: MenuItem,
    log: MenuItem,
    config: MenuItem,
    sign_in: CheckMenuItem,
    exit: MenuItem,
}

impl Items {
    fn new(language: Language) -> tray_icon::menu::Result<Self> {
        let items = Self {
            menu: Menu::new(),
            status: MenuItem::new(Snapshot::initial().state.label(language), false, None),
            config_error: None,
            failure: None,
            start: MenuItem::new(language.text(Text::Start), false, None),
            stop: MenuItem::new(language.text(Text::Stop), false, None),
            dashboard: MenuItem::new(language.text(Text::Dashboard), false, None),
            log: MenuItem::new(language.text(Text::Log), false, None),
            config: MenuItem::new(language.text(Text::Config), false, None),
            sign_in: CheckMenuItem::new(language.text(Text::SignIn), true, false, None),
            exit: MenuItem::new(language.text(Text::Exit), true, None),
        };
        items.menu.append_items(&[
            &items.status,
            &PredefinedMenuItem::separator(),
            &items.start,
            &items.stop,
            &PredefinedMenuItem::separator(),
            &items.dashboard,
            &items.log,
            &items.config,
            &PredefinedMenuItem::separator(),
            &items.sign_in,
            &PredefinedMenuItem::separator(),
            &items.exit,
        ])?;
        Ok(items)
    }

    fn apply(&mut self, snapshot: &Snapshot, language: Language) -> tray_icon::menu::Result<()> {
        self.status.set_text(snapshot.state.label(language));
        if let Some(item) = self.config_error.take() {
            self.menu.remove(&item)?;
        }
        if let Some(reason) = snapshot.config_error {
            let item = MenuItem::new(
                format!(
                    "{}{}",
                    language.text(Text::ConfigFailure),
                    reason.label(language)
                ),
                false,
                None,
            );
            self.menu.insert(&item, 1)?;
            self.config_error = Some(item);
        }
        let enabled = snapshot.state.enabled(
            snapshot.config_error.is_none() && snapshot.dashboard.is_some(),
            snapshot.log.is_some(),
            snapshot.config_path.is_some(),
        );
        self.start.set_enabled(enabled.start);
        self.stop.set_enabled(enabled.stop);
        self.dashboard.set_enabled(enabled.dashboard);
        self.log.set_enabled(enabled.log);
        self.config.set_enabled(enabled.config);
        self.sign_in.set_checked(snapshot.sign_in);
        Ok(())
    }

    fn clear_failure(&mut self) -> tray_icon::menu::Result<()> {
        if let Some(item) = self.failure.take() {
            self.menu.remove(&item)?;
        }
        Ok(())
    }

    fn show_failure(
        &mut self,
        action: Action,
        reason: crate::rules::Reason,
        language: Language,
    ) -> tray_icon::menu::Result<()> {
        self.clear_failure()?;
        let item = MenuItem::new(
            format!(
                "{} {}: {}",
                language.text(action.text()),
                language.text(Text::ActionFailure),
                reason.label(language)
            ),
            false,
            None,
        );
        self.menu
            .insert(&item, 1 + usize::from(self.config_error.is_some()))?;
        self.failure = Some(item);
        Ok(())
    }

    fn action(&self, event: &MenuEvent) -> Option<Action> {
        for (id, action) in [
            (self.start.id(), Action::Start),
            (self.stop.id(), Action::Stop),
            (self.dashboard.id(), Action::Dashboard),
            (self.log.id(), Action::Log),
            (self.config.id(), Action::Config),
            (self.sign_in.id(), Action::SignIn),
        ] {
            if id == event.id() {
                return Some(action);
            }
        }
        None
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let language = crate::registry::language();
    let mut items = Items::new(language)?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(items.menu.clone()))
        .with_icon(icon()?)
        // Windows keeps the first tooltip as the icon's name in its taskbar
        // settings, so it is the product name without a state. The first
        // refresh sets `VNC Relay: <state>` right after.
        .with_tooltip("VNC Relay")
        .with_menu_on_left_click(true)
        .with_menu_on_right_click(true)
        .build()?;
    let (requests, replies) = worker::spawn()?;
    requests.send(Request::Refresh)?;
    let mut generation = 0_u64;
    let mut sign_in = false;
    let mut ui_error: Option<Box<dyn std::error::Error>> = None;
    let loop_result = crate::message_loop::run(|| {
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id() == items.exit.id() {
                    return Ok(false);
                }
                if let Some(action) = items.action(&event) {
                    generation = generation.wrapping_add(1);
                    items.clear_failure()?;
                    // Native check items toggle themselves; restore the observed
                    // registry value until the worker reads it after the action.
                    items.sign_in.set_checked(sign_in);
                    requests.send(Request::Action(action, generation))?;
                }
            }
            while let Ok(reply) = replies.try_recv() {
                match reply {
                    Reply::Snapshot(snapshot) => {
                        sign_in = snapshot.sign_in;
                        tray.set_tooltip(Some(snapshot.state.label(language)))?;
                        items.apply(&snapshot, language)?;
                    }
                    Reply::Failure(action, request_generation, reason)
                        if request_generation == generation =>
                    {
                        items.show_failure(action, reason, language)?;
                    }
                    Reply::Failure(..) => {}
                }
            }
            Ok(true)
        })();
        match result {
            Ok(keep_running) => keep_running,
            Err(error) => {
                ui_error = Some(error);
                false
            }
        }
    });
    drop(tray);
    loop_result?;
    if let Some(error) = ui_error {
        return Err(error);
    }
    Ok(())
}

const ICON_SIZE: u32 = 32;

fn icon() -> Result<Icon, tray_icon::BadIcon> {
    Icon::from_rgba(brand_mark_rgba(), ICON_SIZE, ICON_SIZE)
}

/// Draws the dashboard brand mark as the tray icon (DESIGN.md "아이콘 규칙", Q258).
///
/// Like the top bar: a `spacing.brand-size` square with `rounded.md` corners in
/// `colors.navbar-accent`, holding the 16-unit `brand` icon at 1.25× in
/// `colors.navbar-brand` lines of `spacing.icon-stroke`. The tray takes RGBA, not
/// SVG, so each pixel counts 4×4 samples to keep the edges smooth when Windows
/// scales the icon down.
fn brand_mark_rgba() -> Vec<u8> {
    const RADIUS: f32 = 8.0;
    const BACKGROUND: [u8; 3] = [0x1c, 0x27, 0x23];
    const LINE: [u8; 3] = [0xff, 0xff, 0xff];
    const SCALE: f32 = 1.25;
    const OFFSET: f32 = (ICON_SIZE as f32 - 16.0 * SCALE) / 2.0;
    const HALF_WIDTH: f32 = 1.75 * SCALE / 2.0;
    const SAMPLES: u32 = 4;
    // The `brand` icon of web/dashboard/app/components/icons.js as segments.
    const SEGMENTS: [[f32; 4]; 6] = [
        [2.75, 5.5, 12.25, 5.5],
        [9.75, 3.0, 12.25, 5.5],
        [12.25, 5.5, 9.75, 8.0],
        [13.25, 10.5, 3.75, 10.5],
        [6.25, 8.0, 3.75, 10.5],
        [3.75, 10.5, 6.25, 13.0],
    ];
    let size = ICON_SIZE as f32;
    let mut pixels = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let (mut inside, mut line) = (0_u32, 0_u32);
            for sample in 0..SAMPLES * SAMPLES {
                let px = x as f32 + ((sample % SAMPLES) as f32 + 0.5) / SAMPLES as f32;
                let py = y as f32 + ((sample / SAMPLES) as f32 + 0.5) / SAMPLES as f32;
                let dx = (RADIUS - px).max(px - (size - RADIUS)).max(0.0);
                let dy = (RADIUS - py).max(py - (size - RADIUS)).max(0.0);
                if dx * dx + dy * dy > RADIUS * RADIUS {
                    continue;
                }
                inside += 1;
                let on_line = SEGMENTS.iter().any(|&[ax, ay, bx, by]| {
                    let (ax, ay) = (OFFSET + ax * SCALE, OFFSET + ay * SCALE);
                    let (sx, sy) = (OFFSET + bx * SCALE - ax, OFFSET + by * SCALE - ay);
                    let t =
                        (((px - ax) * sx + (py - ay) * sy) / (sx * sx + sy * sy)).clamp(0.0, 1.0);
                    let (cx, cy) = (ax + t * sx - px, ay + t * sy - py);
                    cx * cx + cy * cy <= HALF_WIDTH * HALF_WIDTH
                });
                line += u32::from(on_line);
            }
            if inside == 0 {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Coverage-weighted mix; every value stays within 0..=255.
            for (background, stroke) in BACKGROUND.into_iter().zip(LINE) {
                let mixed = u32::from(background) * (inside - line) + u32::from(stroke) * line;
                pixels.push((mixed / inside) as u8);
            }
            pixels.push((inside * 255 / (SAMPLES * SAMPLES)) as u8);
        }
    }
    pixels
}
