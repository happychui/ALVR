use crate::dashboard::basic_components;
use alvr_packets::{FirewallRulesAction, PathValuePair, ServerRequest};
use eframe::{
    egui::{Button, Label, Layout, RichText, Ui},
    emath::Align,
};

pub enum SetupWizardRequest {
    ServerRequest(ServerRequest),
    Close { finished: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Welcome = 0,
    ResetSettings = 1,
    HardwareRequirements = 2,
    SoftwareRequirements = 3,
    HandGestures = 4,
    Firewall = 5,
    Recommendations = 6,
    Finished = 7,
}

fn index_to_page(index: usize) -> Page {
    match index {
        0 => Page::Welcome,
        1 => Page::ResetSettings,
        2 => Page::HardwareRequirements,
        3 => Page::SoftwareRequirements,
        4 => Page::HandGestures,
        5 => Page::Firewall,
        6 => Page::Recommendations,
        7 => Page::Finished,
        _ => unreachable!(),
    }
}

fn page_content(
    ui: &mut Ui,
    subtitle: &str,
    paragraph: &str,
    interactible_content: impl FnMut(&mut Ui),
) {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        ui.add_space(60.0);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.add_space(60.0);
            ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                ui.add_space(15.0);
                ui.heading(RichText::new(subtitle).size(20.0));
                ui.add(Label::new(RichText::new(paragraph).size(14.0)).wrap());
                ui.add_space(30.0);
                ui.vertical_centered(interactible_content);
            });
        })
    });
}

pub struct SetupWizard {
    page: Page,
    only_touch: bool,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            page: Page::Welcome,
            only_touch: true,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<SetupWizardRequest> {
        let mut request = None;

        ui.horizontal(|ui| {
            ui.add_space(60.0);
            ui.vertical(|ui| {
                ui.add_space(30.0);
                ui.heading(RichText::new("Welcome to ALVR").size(30.0));
                ui.add_space(5.0);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(15.0);
                if ui.button("❌").clicked() {
                    request = Some(SetupWizardRequest::Close { finished: false });
                }
            })
        });
        ui.separator();
        match &self.page {
            Page::Welcome => page_content(
                ui,
                "This setup wizard will help you setup ALVR.",
                "",
                |_| (),
            ),
            Page::ResetSettings => page_content(
                ui,
                "Reset settings",
                "It is recommended to reset your settings everytime you update ALVR.",
                |ui| {
                    if ui.button("Reset settings").clicked() {
                        request = Some(SetupWizardRequest::ServerRequest(
                            ServerRequest::UpdateSession(Box::default()),
                        ));
                    }
                },
            ),
            Page::HardwareRequirements => page_content(
                ui,
                "Hardware requirements",
                r"ALVR requires a dedicated and recent graphics card.
Make sure you have at least one output audio device.",
                |_| (),
            ),
            Page::SoftwareRequirements => page_content(
                ui,
                "Software requirements",
                if cfg!(windows) {
                    r"To stream the headset microphone on Windows you need to install VB-Cable or Voicemeeter."
                } else {
                    r"Unsupported OS"
                },
                #[allow(unused_variables)]
                |ui| {
                    #[cfg(windows)]
                    if ui.button("Download VB-Cable").clicked() {
                        ui.ctx().open_url(crate::dashboard::egui::OpenUrl::same_tab(
                            "https://vb-audio.com/Cable/",
                        ));
                    }
                },
            ),

            Page::HandGestures => page_content(
                ui,
                "Hand Gestures",
                r"ALVR allows you to use Hand Tracking and emulate controller buttons using it.
By default, controller button emulation is set to prevent accidental clicks. You can re-enable gestures by disabling slider bellow.",
                |ui| {
                    ui.label("Only touch");
                    if basic_components::switch(ui, &mut self.only_touch).changed() {
                        request = Some(SetupWizardRequest::ServerRequest(
                            ServerRequest::SetValues(vec![PathValuePair {
                                path: alvr_packets::parse_path(&format!(
                                    "session_settings.headset.controllers.content.{}",
                                    "gestures.content.only_touch"
                                )),
                                value: serde_json::Value::Bool(self.only_touch),
                            }]),
                        ));
                    }
                },
            ),
            Page::Firewall => page_content(
                ui,
                "Firewall",
                r"To communicate with the headset, some firewall rules need to be set.
This requires administrator rights!",
                |ui| {
                    if ui.button("Add firewall rules").clicked() {
                        request = Some(SetupWizardRequest::ServerRequest(
                            ServerRequest::FirewallRules(FirewallRulesAction::Add),
                        ));
                    }
                },
            ),
            Page::Recommendations => page_content(
                ui,
                "Recommendations",
                r"ALVR supports multiple types of PC hardware and headsets but not all might work correctly with default settings. Please try tweaking different settings like encoder, bitrate and others if your ALVR experience is great or not optimal.",
                |_| (),
            ),
            Page::Finished => page_content(
                ui,
                "Finished",
                r#"You can always restart this setup wizard from the "Installation" tab on the left."#,
                |_| (),
            ),
        };

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                if self.page == Page::Finished {
                    if ui.button("Finish").clicked() {
                        request = Some(SetupWizardRequest::Close { finished: true });
                    }
                } else if ui.button("Next").clicked() {
                    self.page = index_to_page(self.page as usize + 1);
                }
                if ui
                    .add_visible(self.page != Page::Welcome, Button::new("Back"))
                    .clicked()
                {
                    self.page = index_to_page(self.page as usize - 1);
                }
            });
            ui.separator();
        });

        request
    }
}
