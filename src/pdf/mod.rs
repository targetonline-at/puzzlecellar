//! Das Rätselheft als PDF: Seitenaufteilung, Kopf- und Fußzeilen.
//! Die Nonogramm-Auflösung (nur der Motivname) steht spiegelverkehrt in der Fußzeile der Folgeseite.
//!
//! Eine Seite ist in vier Felder geteilt (2 × 2). Sudoku, Binoxo und Schiffe belegen je ein
//! Viertel, größere Rätsel später eine halbe oder ganze Seite.

mod binoxo;
mod nonogramm;
mod schiffe;
mod sudoku;
pub mod zeichnen;

use crate::raetsel::{Platz, Raetsel};
use printpdf::{Mm, PdfDocument, PdfDocumentReference, PdfLayerReference};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use zeichnen::*;

pub struct HeftKopf {
    pub nummer: u32,
    pub datum: String,
}

/// Papierformat des Hefts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// 210 × 297 mm – zum Ausdrucken zu Hause
    A4,
    /// 170 × 240 mm – Buch-/Magazinformat für die Druckerei
    Buch,
}

impl Format {
    pub const ALLE: [Format; 2] = [Format::A4, Format::Buch];

    pub fn name(self) -> &'static str {
        match self {
            Format::A4 => "A4 (zum Ausdrucken)",
            Format::Buch => "170 × 240 mm (Buchformat, Druckerei)",
        }
    }

    fn masse(self) -> Masse {
        match self {
            Format::A4 => Masse {
                b: 210.0,
                h: 297.0,
                rand: 18.0,
                kopf_y: 19.5,
                linie_y: 22.0,
                oben: 27.0,
                unten: 282.0,
                fuss_y: 289.0,
                spalt: 8.0,
            },
            Format::Buch => Masse {
                b: 170.0,
                h: 240.0,
                rand: 13.0,
                kopf_y: 14.5,
                linie_y: 17.0,
                oben: 21.5,
                unten: 228.0,
                fuss_y: 234.0,
                spalt: 6.0,
            },
        }
    }
}

/// Seitenmaße in mm (y von oben).
struct Masse {
    b: f32,
    h: f32,
    /// Rand links und rechts
    rand: f32,
    /// Grundlinie der Kopfzeile, Linie darunter
    kopf_y: f32,
    linie_y: f32,
    /// Bereich für die Rätsel
    oben: f32,
    unten: f32,
    /// Grundlinie der Fußzeile
    fuss_y: f32,
    /// Abstand zwischen den Feldern einer Seite
    spalt: f32,
}

pub fn schreibe_heft(raetsel: &[Raetsel], kopf: &HeftKopf, format: Format, pfad: &Path) -> Result<(), String> {
    let m = format.masse();
    let (seite_b, seite_h) = (m.b, m.h);
    let (rand, inhalt_oben, inhalt_unten, spalt) = (m.rand, m.oben, m.unten, m.spalt);
    let breite = seite_b - 2.0 * rand;
    let titel = format!("Rätselheft Nr. {}", kopf.nummer);
    let (doc, s1, e1) = PdfDocument::new(&titel, Mm(seite_b), Mm(seite_h), "Inhalt");
    let schriften = Schriften::laden(&doc)?;
    let mut seiten: Vec<PdfLayerReference> = vec![doc.get_page(s1).get_layer(e1)];
    let mut erste_seite_leer = true;

    let neue_seite = |doc: &PdfDocumentReference, seiten: &mut Vec<PdfLayerReference>, erste_leer: &mut bool| {
        if *erste_leer {
            *erste_leer = false;
        } else {
            let (p, l) = doc.add_page(Mm(seite_b), Mm(seite_h), "Inhalt");
            seiten.push(doc.get_page(p).get_layer(l));
        }
    };

    // Felder: 0 = oben links, 1 = oben rechts, 2 = unten links, 3 = unten rechts
    let halb_h = (inhalt_unten - inhalt_oben) / 2.0;
    let viertel_b = (breite - spalt) / 2.0;
    // (y, Höhe) der oberen bzw. unteren Feldzeile
    let zeile = |z: usize| (inhalt_oben + z as f32 * (halb_h + spalt / 2.0), halb_h - spalt / 2.0);
    let mitte_x = rand + breite / 2.0;
    // Nächstes freie Feld auf der aktuellen Seite (4 = Seite voll)
    let mut frei = 4usize;
    // Nonogramme: (Seite, Nummer, Motivname) – der Name kommt gespiegelt auf die nächste Seite
    let mut aufloesungen: Vec<(usize, usize, String)> = Vec::new();

    for (k, r) in raetsel.iter().enumerate() {
        let (feld, pos) = match r.platz() {
            Platz::Viertelseite => {
                if frei >= 4 {
                    neue_seite(&doc, &mut seiten, &mut erste_seite_leer);
                    frei = 0;
                }
                let pos = frei;
                frei += 1;
                let (y, h) = zeile(pos / 2);
                let x = rand + (pos % 2) as f32 * (viertel_b + spalt);
                (Rechteck::neu(x, y, viertel_b, h), pos)
            }
            Platz::Halbseite => {
                if frei == 1 {
                    frei = 2;
                }
                if frei >= 3 {
                    neue_seite(&doc, &mut seiten, &mut erste_seite_leer);
                    frei = 0;
                }
                let pos = frei;
                frei += 2;
                let (y, h) = zeile(pos / 2);
                (Rechteck::neu(rand, y, breite, h), pos)
            }
            Platz::Ganzseite => {
                neue_seite(&doc, &mut seiten, &mut erste_seite_leer);
                frei = 4;
                (Rechteck::neu(rand, inhalt_oben, breite, inhalt_unten - inhalt_oben), 0)
            }
        };
        let s = Seite { ebene: seiten.last().unwrap().clone(), schriften: &schriften, hoehe: seite_h };

        // Feine graue Trennlinien zwischen den Feldern
        s.linienfarbe(0.75);
        if pos == 2 {
            let y = inhalt_oben + halb_h;
            s.linie(rand, y, rand + breite, y, 0.3);
        }
        if r.platz() == Platz::Viertelseite && pos % 2 == 1 {
            let (y, h) = zeile(pos / 2);
            s.linie(mitte_x, y, mitte_x, y + h, 0.3);
        }
        s.linienfarbe(0.0);

        zeichne_raetsel(&s, k + 1, r, feld);
        if let Raetsel::Nonogramm(ng) = r {
            aufloesungen.push((seiten.len() - 1, k + 1, ng.name.clone()));
        }
    }

    // --- Kopf- und Fußzeilen ---
    let gesamt = seiten.len();
    for (n, ebene) in seiten.iter().enumerate() {
        let s = Seite { ebene: ebene.clone(), schriften: &schriften, hoehe: seite_h };
        s.fuellfarbe(0.0);
        s.text(&titel, rand, m.kopf_y, 10.0, Schrift::Titel, Ausrichtung::Links);
        s.fuellfarbe(0.35);
        s.text(&kopf.datum, rand + breite, m.kopf_y, 9.0, Schrift::Normal, Ausrichtung::Rechts);
        s.linienfarbe(0.0);
        s.linie(rand, m.linie_y, rand + breite, m.linie_y, 0.6);
        s.text(
            &format!("Seite {} von {}", n + 1, gesamt),
            seite_b / 2.0,
            m.fuss_y,
            8.5,
            Schrift::Normal,
            Ausrichtung::Mitte,
        );
        // Auflösung der Nonogramme der Vorseite: klein, hellgrau, spiegelverkehrt
        let hier: Vec<String> = aufloesungen
            .iter()
            .filter(|(seite, _, _)| (seite + 1) % gesamt == n)
            .map(|(_, nr, name)| format!("Nr. {nr}: {name}"))
            .collect();
        if !hier.is_empty() {
            s.fuellfarbe(0.55);
            s.text_gespiegelt(&hier.join(" · "), rand + breite, m.fuss_y, 7.5, Schrift::Normal);
        }
        s.fuellfarbe(0.0);
    }

    let datei = File::create(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    doc.save(&mut BufWriter::new(datei))
        .map_err(|e| format!("PDF konnte nicht gespeichert werden: {e}"))
}

/// Schlanker Kopf (Nummer links, Sterne rechts) und darunter das Rätsel.
fn zeichne_raetsel(s: &Seite, nr: usize, r: &Raetsel, feld: Rechteck) {
    s.fuellfarbe(0.0);
    s.text(&nr.to_string(), feld.x, feld.y + 6.0, 15.0, Schrift::Titel, Ausrichtung::Links);

    // Sterne rechtsbündig, davor die Stufe in Worten
    let radius = 1.8;
    let abstand = 2.0 * radius + 1.1;
    let x_start = feld.x + feld.b - 4.0 * abstand - radius;
    let sterne = r.stufe().sterne();
    s.fuellfarbe(0.1);
    s.linienfarbe(0.1);
    for k in 0..5u8 {
        s.stern(x_start + k as f32 * abstand, feld.y + 3.9, radius, k < sterne);
    }
    s.linienfarbe(0.0);
    s.fuellfarbe(0.35);
    s.text(r.stufe().name(), x_start - radius - 2.5, feld.y + 5.5, 8.5, Schrift::Normal, Ausrichtung::Rechts);
    s.fuellfarbe(0.0);

    let inhalt = feld.ohne_oben(9.5);
    match r {
        Raetsel::Sudoku(sd) => sudoku::zeichne_aufgabe(s, sd, inhalt),
        Raetsel::Binoxo(b) => binoxo::zeichne_aufgabe(s, b, inhalt),
        Raetsel::Schiffe(sch) => schiffe::zeichne_aufgabe(s, sch, inhalt),
        Raetsel::Nonogramm(ng) => nonogramm::zeichne_aufgabe(s, ng, inhalt),
    }
}
