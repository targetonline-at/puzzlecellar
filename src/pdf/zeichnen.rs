//! Dünne Zeichenschicht über `printpdf`: Millimeter, Ursprung oben links.

use printpdf::path::{PaintMode, WindingOrder};
use printpdf::*;
use std::io::Cursor;
use std::path::Path;

/// Ein Rechteck in Millimetern (x/y = linke obere Ecke).
#[derive(Clone, Copy, Debug)]
pub struct Rechteck {
    pub x: f32,
    pub y: f32,
    pub b: f32,
    pub h: f32,
}

impl Rechteck {
    pub fn neu(x: f32, y: f32, b: f32, h: f32) -> Self {
        Rechteck { x, y, b, h }
    }
    /// Schneidet oben `dy` Millimeter ab.
    pub fn ohne_oben(self, dy: f32) -> Self {
        Rechteck::neu(self.x, self.y + dy, self.b, (self.h - dy).max(0.0))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Schrift {
    /// Ziffern und Fließtext
    Normal = 0,
    Fett = 1,
    /// Überschriften
    Titel = 2,
}

#[derive(Clone, Copy, Debug)]
pub enum Ausrichtung {
    Links,
    Mitte,
    Rechts,
}

/// Die eingebetteten Schriften samt Metrik (für das Ausmessen von Text).
pub struct Schriften {
    refs: [IndirectFontRef; 3],
    daten: [Vec<u8>; 3],
}

impl Schriften {
    pub fn laden(doc: &PdfDocumentReference) -> Result<Self, String> {
        let normal = lade_systemschrift(&["arial.ttf", "segoeui.ttf", "calibri.ttf"])?;
        let fett = lade_systemschrift(&["arialbd.ttf", "segoeuib.ttf", "calibrib.ttf"])?;
        let titel = lade_systemschrift(&["georgiab.ttf", "constanb.ttf", "arialbd.ttf"])?;
        let einbetten = |d: &Vec<u8>| {
            doc.add_external_font(Cursor::new(d.clone()))
                .map_err(|e| format!("Schrift konnte nicht eingebettet werden: {e}"))
        };
        Ok(Schriften {
            refs: [einbetten(&normal)?, einbetten(&fett)?, einbetten(&titel)?],
            daten: [normal, fett, titel],
        })
    }

    fn gesicht(&self, s: Schrift) -> Option<ttf_parser::Face<'_>> {
        ttf_parser::Face::parse(&self.daten[s as usize], 0).ok()
    }

    /// Textbreite in mm.
    pub fn breite(&self, s: Schrift, text: &str, groesse_pt: f32) -> f32 {
        let Some(f) = self.gesicht(s) else {
            return text.chars().count() as f32 * groesse_pt * 0.5 * MM_PRO_PT;
        };
        let em = f.units_per_em() as f32;
        let einheiten: f32 = text
            .chars()
            .map(|c| {
                f.glyph_index(c)
                    .and_then(|g| f.glyph_hor_advance(g))
                    .unwrap_or(0) as f32
            })
            .sum();
        einheiten / em * groesse_pt * MM_PRO_PT
    }

    /// Höhe der Großbuchstaben/Ziffern in mm.
    pub fn versalhoehe(&self, s: Schrift, groesse_pt: f32) -> f32 {
        let anteil = self
            .gesicht(s)
            .and_then(|f| f.capital_height().map(|h| h as f32 / f.units_per_em() as f32))
            .unwrap_or(0.716);
        anteil * groesse_pt * MM_PRO_PT
    }
}

fn lade_systemschrift(kandidaten: &[&str]) -> Result<Vec<u8>, String> {
    let ordner = std::env::var_os("WINDIR")
        .map(|w| Path::new(&w).join("Fonts"))
        .unwrap_or_else(|| Path::new(r"C:\Windows\Fonts").to_path_buf());
    for k in kandidaten {
        if let Ok(d) = std::fs::read(ordner.join(k)) {
            return Ok(d);
        }
    }
    Err(format!("Keine der Schriften {kandidaten:?} in {} gefunden", ordner.display()))
}

pub const MM_PRO_PT: f32 = 25.4 / 72.0;

/// Eine Seite, auf die gezeichnet wird.
pub struct Seite<'a> {
    pub ebene: PdfLayerReference,
    pub schriften: &'a Schriften,
    /// Seitenhöhe in mm (PDF zählt von unten, wir von oben)
    pub hoehe: f32,
}

fn grau(g: f32) -> Color {
    Color::Greyscale(Greyscale::new(g, None))
}

impl<'a> Seite<'a> {
    fn punkt(&self, x: f32, y: f32) -> (Point, bool) {
        (Point::new(Mm(x), Mm(self.hoehe - y)), false)
    }

    pub fn linienfarbe(&self, g: f32) {
        self.ebene.set_outline_color(grau(g));
    }

    pub fn fuellfarbe(&self, g: f32) {
        self.ebene.set_fill_color(grau(g));
    }

    pub fn fuellfarbe_rgb(&self, r: f32, g: f32, b: f32) {
        self.ebene.set_fill_color(Color::Rgb(Rgb::new(r, g, b, None)));
    }

    pub fn linie(&self, x1: f32, y1: f32, x2: f32, y2: f32, staerke_pt: f32) {
        self.ebene.set_outline_thickness(staerke_pt);
        self.ebene.add_line(Line {
            points: vec![self.punkt(x1, y1), self.punkt(x2, y2)],
            is_closed: false,
        });
    }

    pub fn rahmen(&self, r: Rechteck, staerke_pt: f32) {
        self.ebene.set_outline_thickness(staerke_pt);
        self.ebene.add_line(Line {
            points: vec![
                self.punkt(r.x, r.y),
                self.punkt(r.x + r.b, r.y),
                self.punkt(r.x + r.b, r.y + r.h),
                self.punkt(r.x, r.y + r.h),
            ],
            is_closed: true,
        });
    }

    /// Gefülltes Rechteck in der aktuellen Füllfarbe.
    pub fn flaeche(&self, r: Rechteck) {
        self.vieleck(
            &[
                (r.x, r.y),
                (r.x + r.b, r.y),
                (r.x + r.b, r.y + r.h),
                (r.x, r.y + r.h),
            ],
            PaintMode::Fill,
        );
    }

    pub fn vieleck(&self, ecken: &[(f32, f32)], modus: PaintMode) {
        self.ebene.add_polygon(Polygon {
            rings: vec![ecken.iter().map(|&(x, y)| self.punkt(x, y)).collect()],
            mode: modus,
            winding_order: WindingOrder::NonZero,
        });
    }

    /// Text auf einer Grundlinie (y = Grundlinie).
    pub fn text(&self, t: &str, x: f32, y: f32, groesse_pt: f32, s: Schrift, a: Ausrichtung) {
        let b = self.schriften.breite(s, t, groesse_pt);
        let x = match a {
            Ausrichtung::Links => x,
            Ausrichtung::Mitte => x - b / 2.0,
            Ausrichtung::Rechts => x - b,
        };
        self.ebene.use_text(
            t,
            groesse_pt,
            Mm(x),
            Mm(self.hoehe - y),
            &self.schriften.refs[s as usize],
        );
    }

    /// Spiegelverkehrter Text (nur mit Spiegel lesbar), endet rechts bei `x_rechts`.
    pub fn text_gespiegelt(&self, t: &str, x_rechts: f32, y: f32, groesse_pt: f32, s: Schrift) {
        let schrift = &self.schriften.refs[s as usize];
        self.ebene.begin_text_section();
        self.ebene.set_font(schrift, groesse_pt);
        // a = -1: waagrecht gespiegelt, der Text läuft von x_rechts nach links
        self.ebene.set_text_matrix(TextMatrix::Raw([
            -1.0,
            0.0,
            0.0,
            1.0,
            Pt::from(Mm(x_rechts)).0,
            Pt::from(Mm(self.hoehe - y)).0,
        ]));
        self.ebene.write_text(t, schrift);
        self.ebene.end_text_section();
    }

    /// Text waagrecht und senkrecht mittig um (cx, cy).
    pub fn text_mittig(&self, t: &str, cx: f32, cy: f32, groesse_pt: f32, s: Schrift) {
        let h = self.schriften.versalhoehe(s, groesse_pt);
        self.text(t, cx, cy + h / 2.0, groesse_pt, s, Ausrichtung::Mitte);
    }

    /// Kreislinie um (cx, cy) mit Radius r.
    pub fn kreis(&self, cx: f32, cy: f32, r: f32, staerke_pt: f32) {
        self.ebene.set_outline_thickness(staerke_pt);
        self.ebene.add_line(Line {
            points: (0..48)
                .map(|k| {
                    let w = k as f32 * std::f32::consts::TAU / 48.0;
                    self.punkt(cx + r * w.cos(), cy + r * w.sin())
                })
                .collect(),
            is_closed: true,
        });
    }

    /// Offener Linienzug.
    pub fn linienzug(&self, punkte: &[(f32, f32)], staerke_pt: f32) {
        self.ebene.set_outline_thickness(staerke_pt);
        self.ebene.add_line(Line {
            points: punkte.iter().map(|&(x, y)| self.punkt(x, y)).collect(),
            is_closed: false,
        });
    }

    /// Gefüllter Kreis in der aktuellen Füllfarbe.
    pub fn scheibe(&self, cx: f32, cy: f32, r: f32) {
        let ecken: Vec<(f32, f32)> = (0..48)
            .map(|k| {
                let w = k as f32 * std::f32::consts::TAU / 48.0;
                (cx + r * w.cos(), cy + r * w.sin())
            })
            .collect();
        self.vieleck(&ecken, PaintMode::Fill);
    }

    /// Fünfzackiger Stern um (cx, cy) mit Außenradius r.
    pub fn stern(&self, cx: f32, cy: f32, r: f32, gefuellt: bool) {
        let innen = r * 0.42;
        let ecken: Vec<(f32, f32)> = (0..10)
            .map(|k| {
                let winkel = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                let rr = if k % 2 == 0 { r } else { innen };
                (cx + rr * winkel.cos(), cy + rr * winkel.sin())
            })
            .collect();
        if gefuellt {
            self.vieleck(&ecken, PaintMode::Fill);
        } else {
            self.ebene.set_outline_thickness(0.5);
            self.ebene.add_line(Line {
                points: ecken.iter().map(|&(x, y)| self.punkt(x, y)).collect(),
                is_closed: true,
            });
        }
    }
}
