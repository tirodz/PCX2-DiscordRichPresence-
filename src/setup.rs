//! First-run setup wizard and settings window.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;

use crate::autostart;
use crate::config::{self, Config};
use crate::helper;
use crate::pine::PineClient;
use crate::state::RuntimeState;

const WINDOW_SIZE: [f32; 2] = [580.0, 500.0];

pub fn run_wizard() -> Result<()> {
    let config = Config::load().unwrap_or_default();
    run_window("PCSX2 Discord Rich Presence - Setup", Wizard::new(&config))
}

pub fn run_settings() -> Result<()> {
    let config = Config::load().unwrap_or_default();
    run_window(
        "PCSX2 Discord Rich Presence - Settings",
        Settings::new(&config),
    )
}

fn run_window<A: eframe::App + 'static>(title: &str, app: A) -> Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size([520.0, 460.0]);

    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(title, options, Box::new(|_cc| Ok(Box::new(app))))
        .map_err(|error| anyhow::anyhow!("could not open the window: {error}"))
}

fn window_icon() -> Option<egui::IconData> {
    const SIZE: u32 = 64;
    Some(egui::IconData {
        rgba: crate::icon::render_icon(SIZE as usize),
        width: SIZE,
        height: SIZE,
    })
}

/// Editable copies of everything setup can change.
struct FormState {
    pcsx2_path: String,
    pine_host: String,
    pine_port: String,
    client_id: String,
    autostart: bool,
    start_helper: bool,
}

impl FormState {
    fn from_config(config: &Config) -> Self {
        Self {
            pcsx2_path: config.pcsx2.exe_path.clone(),
            pine_host: config.pine.host.clone(),
            pine_port: config.pine.port.to_string(),
            client_id: config.discord.client_id.clone(),
            autostart: autostart::installed().unwrap_or(false),
            start_helper: true,
        }
    }

    fn to_config(&self) -> Result<Config, String> {
        let mut config = Config::load().unwrap_or_default();
        config.pcsx2.exe_path = self.pcsx2_path.trim().to_string();
        config.pine.host = self.pine_host.trim().to_string();
        config.pine.port = config::validate_pine(&self.pine_host, &self.pine_port)?;
        config.discord.client_id = self.client_id.trim().to_string();
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        config::validate_pcsx2_path(Path::new(self.pcsx2_path.trim()))?;
        config::validate_pine(&self.pine_host, &self.pine_port)?;
        config::validate_client_id(&self.client_id)?;
        Ok(())
    }
}

type StepResult = (&'static str, Result<(), String>);

/// Save the configuration, apply the startup choice and (re)start the
/// background helper. Runs off the UI thread; each step reports separately.
fn apply_setup(form: &FormState) -> Vec<StepResult> {
    let mut results = Vec::new();

    let config = match form.to_config() {
        Ok(config) => config,
        Err(error) => {
            results.push(("Save the configuration", Err(error)));
            return results;
        }
    };

    results.push((
        "Save the configuration",
        config.save().map_err(|e| format!("{e:#}")),
    ));
    if results.last().map(|(_, r)| r.is_err()) == Some(true) {
        return results;
    }

    results.push((
        if form.autostart {
            "Enable automatic startup"
        } else {
            "Disable automatic startup"
        },
        (if form.autostart {
            autostart::install()
        } else {
            autostart::uninstall()
        })
        .map_err(|e| format!("{e:#}")),
    ));

    if form.start_helper {
        let step = (|| -> Result<(), String> {
            if helper::running_pid().is_some() {
                helper::request_stop().map_err(|e| format!("{e:#}"))?;
                if !helper::wait_until_stopped(Duration::from_secs(10)) {
                    return Err("the running helper did not stop in time".to_string());
                }
            }
            helper::start_detached().map_err(|e| format!("{e:#}"))
        })();
        results.push(("Start the background helper", step));
    }

    results
}

fn describe_state(state: &RuntimeState) -> String {
    match state {
        RuntimeState::Offline => "PCSX2 was not detected".to_string(),
        RuntimeState::Idle => "PCSX2 is open at the main menu".to_string(),
        RuntimeState::Bios { paused } => {
            if *paused {
                "the PS2 system menu is open (paused)".to_string()
            } else {
                "the PS2 system menu is open".to_string()
            }
        }
        RuntimeState::Game { title, paused, .. } => {
            if *paused {
                format!("{title} is running (paused)")
            } else {
                format!("{title} is running")
            }
        }
    }
}

// ---------------------------------------------------------------- wizard

enum Page {
    Welcome,
    Pcsx2,
    Pine,
    Discord,
    Review,
    Finished,
}

struct Wizard {
    page: Page,
    form: FormState,
    error: Option<String>,
    pine_test: Option<Receiver<Result<String, String>>>,
    pine_test_result: Option<Result<String, String>>,
    finishing: Option<Receiver<Vec<StepResult>>>,
    finish_results: Vec<StepResult>,
}

impl Wizard {
    fn new(config: &Config) -> Self {
        Self {
            page: Page::Welcome,
            form: FormState::from_config(config),
            error: None,
            pine_test: None,
            pine_test_result: None,
            finishing: None,
            finish_results: Vec::new(),
        }
    }

    fn next(&mut self) {
        self.error = None;
        match self.page {
            Page::Welcome => self.page = Page::Pcsx2,
            Page::Pcsx2 => {
                match config::validate_pcsx2_path(Path::new(self.form.pcsx2_path.trim())) {
                    Ok(()) => self.page = Page::Pine,
                    Err(error) => self.error = Some(error),
                }
            }
            Page::Pine => match config::validate_pine(&self.form.pine_host, &self.form.pine_port) {
                Ok(_) => self.page = Page::Discord,
                Err(error) => self.error = Some(error),
            },
            Page::Discord => match config::validate_client_id(&self.form.client_id) {
                Ok(()) => self.page = Page::Review,
                Err(error) => self.error = Some(error),
            },
            Page::Review => self.finish(),
            Page::Finished => {}
        }
    }

    fn back(&mut self) {
        self.error = None;
        self.page = match self.page {
            Page::Welcome => Page::Welcome,
            Page::Pcsx2 => Page::Welcome,
            Page::Pine => Page::Pcsx2,
            Page::Discord => Page::Pine,
            Page::Review => Page::Discord,
            Page::Finished => Page::Finished,
        };
    }

    fn finish(&mut self) {
        if let Err(error) = self.form.validate() {
            self.error = Some(error);
            return;
        }
        let form = FormState {
            pcsx2_path: self.form.pcsx2_path.clone(),
            pine_host: self.form.pine_host.clone(),
            pine_port: self.form.pine_port.clone(),
            client_id: self.form.client_id.clone(),
            autostart: self.form.autostart,
            start_helper: self.form.start_helper,
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(apply_setup(&form));
        });
        self.finishing = Some(receiver);
    }

    fn start_pine_test(&mut self) {
        self.pine_test_result = None;
        let host = self.form.pine_host.trim().to_string();
        let port = match self.form.pine_port.trim().parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                self.pine_test_result = Some(Err("the slot is not a valid number".to_string()));
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = PineClient::connect(&host, port)
                .and_then(|mut client| client.read_state())
                .map(|state| format!("Connected: {}.", describe_state(&state)))
                .map_err(|error| {
                    format!(
                        "Could not connect to {host}:{port}. Is PCSX2 running with PINE enabled? ({error})"
                    )
                });
            let _ = sender.send(result);
        });
        self.pine_test = Some(receiver);
    }
}

impl eframe::App for Wizard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Collect results from background work.
        if let Some(receiver) = &self.pine_test {
            if let Ok(result) = receiver.try_recv() {
                self.pine_test_result = Some(result);
                self.pine_test = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        if let Some(receiver) = &self.finishing {
            if let Ok(results) = receiver.try_recv() {
                self.finish_results = results;
                self.finishing = None;
                self.page = Page::Finished;
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.heading(match self.page {
                    Page::Welcome => "Welcome",
                    Page::Pcsx2 => "PCSX2 location",
                    Page::Pine => "PINE connection",
                    Page::Discord => "Discord application",
                    Page::Review => "Confirm setup",
                    Page::Finished => "All done",
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                match self.page {
                    Page::Welcome => wizard_welcome(ui),
                    Page::Pcsx2 => self.page_pcsx2(ui),
                    Page::Pine => self.page_pine(ui),
                    Page::Discord => self.page_discord(ui),
                    Page::Review => self.page_review(ui),
                    Page::Finished => self.page_finished(ui),
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            if let Some(error) = &self.error {
                ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED));
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                match self.page {
                    Page::Welcome | Page::Finished => {}
                    _ => {
                        if ui.button("< Back").clicked() {
                            self.back();
                        }
                    }
                }

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| match self.page {
                        Page::Welcome => {
                            if ui.button("Get started >").clicked() {
                                self.next();
                            }
                        }
                        Page::Pcsx2 | Page::Pine | Page::Discord => {
                            if ui.button("Next >").clicked() {
                                self.next();
                            }
                            if ui.button("Cancel").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        Page::Review => {
                            let finishing = self.finishing.is_some();
                            ui.add_enabled_ui(!finishing, |ui| {
                                if ui.button("Finish").clicked() {
                                    self.next();
                                }
                            });
                            if finishing {
                                ui.label("Finishing setup...");
                            }
                            if ui.button("Cancel").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        Page::Finished => {
                            if ui.button("Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    },
                );
            });
            ui.add_space(4.0);
        });
    }
}

fn wizard_welcome(ui: &mut egui::Ui) {
    ui.label(
        "This wizard sets up Discord Rich Presence for PCSX2. Once finished, the \
         helper starts with Windows and shows what you are playing without you \
         having to launch anything.",
    );
    ui.add_space(10.0);
    ui.label(egui::RichText::new("You will need:").strong());
    ui.add_space(4.0);
    ui.label("  -  Your PCSX2 installation (the pcsx2-qt.exe file)");
    ui.label("  -  PINE enabled in PCSX2 (the wizard explains how)");
    ui.label("  -  A Discord application ID (the wizard explains where to get one)");
    ui.add_space(10.0);
    ui.label("Everything can be changed later from the settings window.");
}

impl Wizard {
    fn page_pcsx2(&mut self, ui: &mut egui::Ui) {
        ui.label("Select your PCSX2 executable. Both installed and portable versions work.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.form.pcsx2_path)
                    .desired_width(380.0)
                    .hint_text("C:\\Emulators\\PCSX2\\pcsx2-qt.exe"),
            );
            if ui.button("Browse...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select the PCSX2 executable")
                    .add_filter("PCSX2 executable", &["exe"])
                    .pick_file()
                {
                    self.form.pcsx2_path = path.display().to_string();
                    self.error = None;
                }
            }
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "This is usually called pcsx2-qt.exe or pcsx2.exe. The path is only used to \
                 check your setup; the helper talks to PCSX2 through PINE and never modifies \
                 the emulator.",
            )
            .small(),
        );
    }

    fn page_pine(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "The helper reads the emulator state through PINE, the interface built into PCSX2.",
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Enable it once in PCSX2:").strong());
        ui.label("  1.  Open PCSX2.");
        ui.label("  2.  Go to Settings > Advanced.");
        ui.label("  3.  Enable the PINE server.");
        ui.label("  4.  Keep the slot at 28011 unless you changed it yourself.");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.add(egui::TextEdit::singleline(&mut self.form.pine_host).desired_width(120.0));
            ui.label("Slot:");
            ui.add(egui::TextEdit::singleline(&mut self.form.pine_port).desired_width(60.0));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let testing = self.pine_test.is_some();
            ui.add_enabled_ui(!testing, |ui| {
                if ui.button("Test connection").clicked() {
                    self.start_pine_test();
                }
            });
            if testing {
                ui.label("Testing...");
            }
        });

        if let Some(result) = &self.pine_test_result {
            ui.add_space(4.0);
            match result {
                Ok(message) => {
                    ui.label(egui::RichText::new(message).color(egui::Color32::LIGHT_GREEN))
                }
                Err(message) => ui.label(egui::RichText::new(message).color(egui::Color32::GOLD)),
            };
            ui.label(
                egui::RichText::new(
                    "The test only works while PCSX2 is running. You can finish setup either way.",
                )
                .small(),
            );
        }
    }

    fn page_discord(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Discord shows rich presence for an application ID. Create one of your own - \
             it takes a minute:",
        );
        ui.add_space(6.0);
        ui.label("  1.  Open discord.com/developers/applications and sign in.");
        ui.label("  2.  Choose New Application and name it, for example, PCSX2.");
        ui.label("  3.  Copy the Application ID from the General Information page.");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Application ID:");
            ui.add(
                egui::TextEdit::singleline(&mut self.form.client_id)
                    .desired_width(220.0)
                    .hint_text("1234567890123456789"),
            );
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "This is a public application identifier, not your Discord password or token. \
                 It is stored only in the configuration file on this PC.",
            )
            .small(),
        );
    }

    fn page_review(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Setup summary").strong());
        ui.add_space(6.0);
        summary_row(ui, "PCSX2", &self.form.pcsx2_path);
        summary_row(
            ui,
            "PINE",
            &format!("{}:{}", self.form.pine_host, self.form.pine_port),
        );
        summary_row(ui, "Discord application ID", &self.form.client_id);
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.form.autostart,
            "Start automatically when I sign in to Windows",
        );
        ui.checkbox(
            &mut self.form.start_helper,
            "Start the background helper now",
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "The helper runs in the background with no window and waits for PCSX2. \
                 You can change all of this later from the settings window.",
            )
            .small(),
        );
    }

    fn page_finished(&mut self, ui: &mut egui::Ui) {
        let failed = self.finish_results.iter().any(|(_, r)| r.is_err());

        if self.finish_results.is_empty() {
            ui.label("Finishing setup...");
            return;
        }

        for (step, result) in &self.finish_results {
            match result {
                Ok(()) => {
                    ui.label(
                        egui::RichText::new(format!("OK    {step}"))
                            .color(egui::Color32::LIGHT_GREEN),
                    );
                }
                Err(error) => {
                    ui.label(
                        egui::RichText::new(format!("FAIL  {step}: {error}"))
                            .color(egui::Color32::LIGHT_RED),
                    );
                }
            }
        }

        ui.add_space(10.0);
        if failed {
            ui.label(
                "Some steps did not complete. Use < Back to adjust your choices, or open the \
                 settings window later to try again.",
            );
        } else {
            ui.label(
                "Setup is complete. Start PCSX2 and your Discord status will follow it - \
                 at the menu, in game, and paused.",
            );
        }
    }
}

fn summary_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{label}:")).strong());
        ui.label(egui::RichText::new(value).monospace());
    });
}

// -------------------------------------------------------------- settings

struct Settings {
    form: FormState,
    error: Option<String>,
    notice: Option<String>,
    restarting: Option<Instant>,
}

impl Settings {
    fn new(config: &Config) -> Self {
        Self {
            form: FormState::from_config(config),
            error: None,
            notice: None,
            restarting: None,
        }
    }

    fn save(&mut self, restart_helper: bool) {
        self.error = None;
        self.notice = None;

        if let Err(error) = self.form.validate() {
            self.error = Some(error);
            return;
        }

        let config = match self.form.to_config() {
            Ok(config) => config,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };

        if let Err(error) = config.save() {
            self.error = Some(format!("could not save the configuration: {error:#}"));
            return;
        }

        let startup = if self.form.autostart {
            autostart::install()
        } else {
            autostart::uninstall()
        };
        if let Err(error) = startup {
            self.error = Some(format!("could not update automatic startup: {error:#}"));
            return;
        }

        if restart_helper {
            if helper::running_pid().is_some() {
                if let Err(error) = helper::request_stop() {
                    self.error = Some(format!("could not stop the helper: {error:#}"));
                    return;
                }
                self.restarting = Some(Instant::now());
            } else if let Err(error) = helper::start_detached() {
                self.error = Some(format!("could not start the helper: {error:#}"));
            } else {
                self.notice = Some("Saved. The background helper is starting.".to_string());
            }
        } else {
            self.notice = Some("Saved.".to_string());
        }
    }
}

impl eframe::App for Settings {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if let Some(since) = self.restarting {
            if helper::running_pid().is_none() {
                self.restarting = None;
                match helper::start_detached() {
                    Ok(()) => {
                        self.notice =
                            Some("Saved. The background helper was restarted.".to_string())
                    }
                    Err(error) => {
                        self.error = Some(format!("could not restart the helper: {error:#}"))
                    }
                }
            } else if since.elapsed() > Duration::from_secs(12) {
                self.restarting = None;
                self.error = Some("the running helper did not stop in time".to_string());
            } else {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }

        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading("Settings");
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(egui::RichText::new("PCSX2 executable").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form.pcsx2_path).desired_width(360.0),
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Select the PCSX2 executable")
                            .add_filter("PCSX2 executable", &["exe"])
                            .pick_file()
                        {
                            self.form.pcsx2_path = path.display().to_string();
                        }
                    }
                });
                ui.add_space(8.0);

                ui.label(egui::RichText::new("PINE connection").strong());
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form.pine_host).desired_width(120.0),
                    );
                    ui.label("Slot:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form.pine_port).desired_width(60.0),
                    );
                });
                ui.add_space(8.0);

                ui.label(egui::RichText::new("Discord application ID").strong());
                ui.add(egui::TextEdit::singleline(&mut self.form.client_id).desired_width(240.0));
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.form.autostart,
                    "Start automatically when I sign in to Windows",
                );
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(egui::RichText::new("Status").strong());
                ui.label(format!("Configuration file: {}", Config::path().display()));
                ui.label(format!(
                    "Automatic startup: {}",
                    if self.form.autostart {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                ui.label(match helper::running_pid() {
                    Some(pid) => format!("Background helper: running (pid {pid})"),
                    None => "Background helper: not running".to_string(),
                });
                ui.label(format!("Log file: {}", crate::logging::path().display()));
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            if let Some(error) = &self.error {
                ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED));
            }
            if let Some(notice) = &self.notice {
                ui.label(egui::RichText::new(notice).color(egui::Color32::LIGHT_GREEN));
            }
            if self.restarting.is_some() {
                ui.label("Restarting the background helper...");
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save and restart helper").clicked() {
                    self.save(true);
                }
                if ui.button("Close").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.add_space(4.0);
        });
    }
}
