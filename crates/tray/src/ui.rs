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

fn icon() -> Result<Icon, tray_icon::BadIcon> {
    let mut pixels = vec![0_u8; 32 * 32 * 4];
    for y in 0_i32..32 {
        for x in 0_i32..32 {
            let corner_x = if x < 7 {
                7 - x
            } else if x > 24 {
                x - 24
            } else {
                0
            };
            let corner_y = if y < 7 {
                7 - y
            } else if y > 24 {
                y - 24
            } else {
                0
            };
            if corner_x * corner_x + corner_y * corner_y > 49 {
                continue;
            }
            let mark = ((7..=24).contains(&x) && (10..=12).contains(&y))
                || ((7..=24).contains(&x) && (19..=21).contains(&y))
                || ((7..=9).contains(&x) && (10..=21).contains(&y))
                || ((22..=24).contains(&x) && (10..=21).contains(&y));
            let color = if mark {
                [255, 255, 255, 255]
            } else {
                [18, 53, 43, 255]
            };
            let index = ((y * 32 + x) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
    Icon::from_rgba(pixels, 32, 32)
}
