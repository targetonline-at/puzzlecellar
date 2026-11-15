//! Das Fenster: Einstellungen, großer Knopf, Fortschritt, fertiges PDF öffnen.

use eframe::egui::{self, RichText};
use puzzlecellar::heft::{self, Art, Einstellungen, Mischung};
use puzzlecellar::pdf::{schreibe_heft, Format, HeftKopf};
use puzzlecellar::raetsel::Stufe;
use rand::Rng;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

enum Nachricht {
    Fortschritt(usize, usize),
    Fertig(PathBuf),
    Fehler(String),
}

enum Zustand {
    Bereit,
    Laeuft { fertig: usize, gesamt: usize, start: Instant },
    Fertig { pfad: PathBuf, sekunden: f32 },
    Fehler(String),
}

pub struct RaetselApp {
    einst: Einstellungen,
    gleich_oeffnen: bool,
    zustand: Zustand,
    empfaenger: Option<Receiver<Nachricht>>,
}

impl RaetselApp {
    pub fn neu(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_zoom_factor(1.15);
        RaetselApp {
            einst: Einstellungen::default(),
            gleich_oeffnen: true,
            zustand: Zustand::Bereit,
            empfaenger: None,
        }
    }

    fn starte(&mut self, ctx: &egui::Context) {
        let einst = self.einst.clone();
        let nummer: u32 = rand::thread_rng().gen_range(10_000..100_000);
        let (tx, rx) = channel();
        self.empfaenger = Some(rx);
        self.zustand = Zustand::Laeuft { fertig: 0, gesamt: einst.anzahl, start: Instant::now() };
        let ctx = ctx.clone();
        let oeffnen = self.gleich_oeffnen;

        std::thread::spawn(move || {
            let arbeit = || -> Result<PathBuf, String> {
                let plan = heft::plane(&einst, nummer);
                let raetsel = heft::erzeuge_raetsel(&plan, nummer, &|f, n| {
                    let _ = tx.send(Nachricht::Fortschritt(f, n));
                    ctx.request_repaint();
                });
                let ordner = ausgabe_ordner();
                std::fs::create_dir_all(&ordner)
                    .map_err(|e| format!("Ordner {} nicht anlegbar: {e}", ordner.display()))?;
                let datum = chrono::Local::now().format("%Y-%m-%d");
                let zusatz = if einst.format == Format::Buch { "_170x240" } else { "" };
                let pfad = ordner.join(format!("Raetselheft_Nr{nummer}_{datum}{zusatz}.pdf"));
                let kopf = HeftKopf { nummer, datum: heft::datum_heute() };
                schreibe_heft(&raetsel, &kopf, einst.format, &pfad)?;
                Ok(pfad)
            };
            let nachricht = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(arbeit)) {
                Ok(Ok(pfad)) => {
                    if oeffnen {
                        oeffne_datei(&pfad);
                    }
                    Nachricht::Fertig(pfad)
                }
                Ok(Err(e)) => Nachricht::Fehler(e),
                Err(_) => Nachricht::Fehler("Interner Fehler beim Erzeugen.".into()),
            };
            let _ = tx.send(nachricht);
            ctx.request_repaint();
        });
    }

    fn nachrichten_abholen(&mut self) {
        let Some(rx) = &self.empfaenger else { return };
        while let Ok(m) = rx.try_recv() {
            match m {
                Nachricht::Fortschritt(f, n) => {
                    if let Zustand::Laeuft { fertig, gesamt, .. } = &mut self.zustand {
                        *fertig = f;
                        *gesamt = n;
                    }
                }
                Nachricht::Fertig(pfad) => {
                    let sekunden = match &self.zustand {
                        Zustand::Laeuft { start, .. } => start.elapsed().as_secs_f32(),
                        _ => 0.0,
                    };
                    self.zustand = Zustand::Fertig { pfad, sekunden };
                }
                Nachricht::Fehler(e) => self.zustand = Zustand::Fehler(e),
            }
        }
    }
}

impl eframe::App for RaetselApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.nachrichten_abholen();
        let laeuft = matches!(self.zustand, Zustand::Laeuft { .. });
        if laeuft {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("PuzzleCellar").size(32.0).strong());
                ui.label("Ein Klick – ein neues Heft voller frischer Rätsel.");
            });
            ui.add_space(16.0);

            ui.add_enabled_ui(!laeuft, |ui| {
                egui::Grid::new("einstellungen")
                    .num_columns(2)
                    .spacing([18.0, 12.0])
                    .show(ui, |ui| {
                        ui.label("Anzahl Rätsel");
                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut self.einst.anzahl, 2..=24));
                            ui.label(RichText::new("(auf volle Seiten aufgerundet)").weak());
                        });
                        ui.end_row();

                        ui.label("Schwierigkeit");
                        egui::ComboBox::from_id_salt("mischung")
                            .selected_text(self.einst.mischung.name())
                            .width(240.0)
                            .show_ui(ui, |ui| {
                                let optionen = [Mischung::EherSchwer, Mischung::Ansteigend]
                                    .into_iter()
                                    .chain(Stufe::IM_HEFT.map(Mischung::Nur));
                                for m in optionen {
                                    ui.selectable_value(&mut self.einst.mischung, m, m.name());
                                }
                            });
                        ui.end_row();

                        ui.label("Papierformat");
                        egui::ComboBox::from_id_salt("format")
                            .selected_text(self.einst.format.name())
                            .width(240.0)
                            .show_ui(ui, |ui| {
                                for f in Format::ALLE {
                                    ui.selectable_value(&mut self.einst.format, f, f.name());
                                }
                            });
                        ui.end_row();

                        ui.label("Rätselarten");
                        ui.vertical(|ui| {
                            for art in Art::ALLE {
                                let mut an = self.einst.arten.contains(&art);
                                if ui.checkbox(&mut an, art.name()).changed() {
                                    if an {
                                        self.einst.arten.push(art);
                                    } else {
                                        self.einst.arten.retain(|&a| a != art);
                                    }
                                }
                            }
                        });
                        ui.end_row();

                        ui.label("Danach");
                        ui.checkbox(&mut self.gleich_oeffnen, "PDF gleich öffnen");
                        ui.end_row();
                    });
            });

            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                let knopf = egui::Button::new(RichText::new("Neues Rätselheft erzeugen").size(20.0).strong())
                    .min_size(egui::vec2(320.0, 50.0));
                if ui.add_enabled(!laeuft && !self.einst.arten.is_empty(), knopf).clicked() {
                    self.starte(ctx);
                }
                ui.add_space(14.0);

                match &self.zustand {
                    Zustand::Bereit => {}
                    Zustand::Laeuft { fertig, gesamt, start } => {
                        let anteil = *fertig as f32 / (*gesamt).max(1) as f32;
                        ui.add(
                            egui::ProgressBar::new(anteil)
                                .desired_width(320.0)
                                .text(format!("Rätsel {fertig} von {gesamt} fertig")),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{:.0} s – teuflische Rätsel brauchen etwas Geduld …",
                                start.elapsed().as_secs_f32()
                            ))
                            .weak(),
                        );
                    }
                    Zustand::Fertig { pfad, sekunden } => {
                        let name = pfad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        ui.label(RichText::new(format!("Fertig nach {sekunden:.1} s: {name}")).strong());
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 250.0).max(0.0) / 2.0);
                            if ui.button("PDF öffnen").clicked() {
                                oeffne_datei(pfad);
                            }
                            if ui.button("Ordner zeigen").clicked() {
                                zeige_im_ordner(pfad);
                            }
                        });
                    }
                    Zustand::Fehler(e) => {
                        ui.colored_label(egui::Color32::from_rgb(220, 60, 60), format!("Fehler: {e}"));
                    }
                }
            });
        });
    }
}

/// `Dokumente\Rätselhefte`
fn ausgabe_ordner() -> PathBuf {
    let basis = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|p| p.join("Documents"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("."));
    basis.join("Rätselhefte")
}

fn oeffne_datei(pfad: &Path) {
    let _ = std::process::Command::new("explorer").arg(pfad).spawn();
}

fn zeige_im_ordner(pfad: &Path) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", pfad.display()))
        .spawn();
}
