//! Schiffe versenken zeichnen: Gitter, Randzahlen rechts und unten,
//! vorgegebene Schiffsteile und Wasser, darunter die Flotte.

use super::zeichnen::{Ausrichtung, Rechteck, Schrift, Seite, MM_PRO_PT};
use crate::raetsel::schiffe::{Richtung, Schiffe, Teil};
use printpdf::path::PaintMode;

pub fn zeichne_aufgabe(s: &Seite, sch: &Schiffe, feld: Rechteck) {
    let n = sch.n;
    let spalten = n as f32 + 1.0; // + Randzahlen
    let zelle = (feld.b / spalten).min((feld.h - 20.0) / spalten).min(8.0);
    let g = zelle * n as f32;
    let x0 = feld.x + (feld.b - g - zelle) / 2.0;
    let y0 = feld.y;

    // Gitter
    s.linienfarbe(0.0);
    for k in 1..n {
        let d = k as f32 * zelle;
        s.linie(x0 + d, y0, x0 + d, y0 + g, 0.45);
        s.linie(x0, y0 + d, x0 + g, y0 + d, 0.45);
    }
    s.rahmen(Rechteck::neu(x0, y0, g, g), 1.6);

    // Randzahlen (fehlende bleiben leer)
    let groesse = zelle / MM_PRO_PT * 0.55;
    s.fuellfarbe(0.0);
    for k in 0..n {
        let mitte = k as f32 * zelle + zelle / 2.0;
        if let Some(z) = sch.zeilen[k] {
            s.text_mittig(&z.to_string(), x0 + g + zelle * 0.55, y0 + mitte, groesse, Schrift::Fett);
        }
        if let Some(z) = sch.spalten[k] {
            s.text_mittig(&z.to_string(), x0 + mitte, y0 + g + zelle * 0.55, groesse, Schrift::Fett);
        }
    }

    // Hinweise – Schiffsteile nur als Kontur (spart Tinte, wie in der Legende)
    s.linienfarbe(0.0);
    for &(i, t) in &sch.hinweise {
        let cx = x0 + (i % n) as f32 * zelle + zelle / 2.0;
        let cy = y0 + (i / n) as f32 * zelle + zelle / 2.0;
        let r = zelle * 0.34;
        teil(s, t, cx, cy, r, r, true);
    }

    // Flotte: „1×“ + Schiff, nach Länge gruppiert
    let mut gruppen: Vec<(usize, usize)> = Vec::new(); // (Länge, Anzahl)
    for &l in &sch.flotte {
        match gruppen.iter_mut().find(|g| g.0 == l) {
            Some(g) => g.1 += 1,
            None => gruppen.push((l, 1)),
        }
    }
    // Eine Zeile, wenn die Schiffe darin groß genug bleiben – sonst zwei
    let y = y0 + g + zelle + 4.5;
    if flotten_feld(s, &gruppen, feld.b) >= 3.0 {
        flotte_zeichnen(s, &gruppen, feld, y);
    } else {
        let mitte = gruppen.len().div_ceil(2);
        let u = flotten_feld(s, &gruppen[..mitte], feld.b).min(flotten_feld(s, &gruppen[mitte..], feld.b));
        flotte_zeichnen(s, &gruppen[..mitte], feld, y);
        flotte_zeichnen(s, &gruppen[mitte..], feld, y + u + 2.5);
    }
    s.fuellfarbe(0.0);
}

const FLOTTE_PT: f32 = 8.0;
const FLOTTE_LUECKE: f32 = 3.5;

/// Größe eines Schiffsfelds, wenn diese Gruppen (Länge, Anzahl) in eine Zeile der Breite `b` sollen.
fn flotten_feld(s: &Seite, gruppen: &[(usize, usize)], b: f32) -> f32 {
    let textbreite: f32 = gruppen
        .iter()
        .map(|&(_, a)| s.schriften.breite(Schrift::Normal, &format!("{a}×"), FLOTTE_PT) + 1.2)
        .sum();
    let felder: usize = gruppen.iter().map(|g| g.0).sum();
    let platz = b - textbreite - FLOTTE_LUECKE * (gruppen.len() - 1) as f32;
    (platz / felder as f32).min(4.2)
}

/// Eine Zeile der Flotte („2×“ + Schiff …), waagrecht mittig in `feld`, Mittellinie bei `y`.
fn flotte_zeichnen(s: &Seite, gruppen: &[(usize, usize)], feld: Rechteck, y: f32) {
    let pt = FLOTTE_PT;
    let luecke = FLOTTE_LUECKE;
    let u = flotten_feld(s, gruppen, feld.b);
    let texte: Vec<String> = gruppen.iter().map(|&(_, a)| format!("{a}×")).collect();
    let textbreite: f32 = texte.iter().map(|t| s.schriften.breite(Schrift::Normal, t, pt) + 1.2).sum();
    let felder: usize = gruppen.iter().map(|g| g.0).sum();
    let breite = textbreite + luecke * (gruppen.len() - 1) as f32 + u * felder as f32;
    let mut x = feld.x + (feld.b - breite) / 2.0;
    for (k, &(laenge, _)) in gruppen.iter().enumerate() {
        s.fuellfarbe(0.3);
        s.text(&texte[k], x, y + u * 0.36, pt, Schrift::Normal, Ausrichtung::Links);
        x += s.schriften.breite(Schrift::Normal, &texte[k], pt) + 1.2;
        // Vorgegebene Flotte nur als Kontur – spart Tinte
        s.linienfarbe(0.2);
        for j in 0..laenge {
            let t = match (laenge, j) {
                (1, _) => Teil::Einzel,
                (_, 0) => Teil::Ende(Richtung::Rechts),
                (l, j) if j == l - 1 => Teil::Ende(Richtung::Links),
                _ => Teil::Mitte,
            };
            // kleine Fugen zwischen den Teilen, damit man die Länge abzählen kann
            teil(s, t, x + (j as f32 + 0.5) * u, y, u * 0.4, u * 0.4, true);
        }
        s.linienfarbe(0.0);
        x += laenge as f32 * u + luecke;
    }
}

/// Ein Schiffsteil oder Wasser um (cx, cy). `r` = halbe Dicke,
/// `aus` = wie weit die flache Seite reicht (in der Flotte bis an den Nachbarn).
/// `kontur` = nur Umriss zeichnen (spart Tinte; für die vorgegebene Flotte).
fn teil(s: &Seite, t: Teil, cx: f32, cy: f32, r: f32, aus: f32, kontur: bool) {
    let st = (r * 0.32).clamp(0.5, 0.9); // Strichstärke der Konturen in pt (größere Teile kräftiger)
    match t {
        Teil::Wasser => {
            s.linienfarbe(0.1);
            for dy in [-0.4, 0.4] {
                let punkte: Vec<(f32, f32)> = (0..=16)
                    .map(|k| {
                        let f = k as f32 / 16.0;
                        let w = f * 2.0 * std::f32::consts::TAU;
                        (cx - r + 2.0 * r * f, cy + dy * r + 0.22 * r * w.sin())
                    })
                    .collect();
                s.linienzug(&punkte, 0.9);
            }
            s.linienfarbe(0.0);
        }
        Teil::Einzel => {
            if kontur {
                s.kreis(cx, cy, r, st);
            } else {
                s.scheibe(cx, cy, r);
            }
        }
        Teil::Mitte => {
            let feld = Rechteck::neu(cx - aus, cy - r, 2.0 * aus, 2.0 * r);
            if kontur {
                s.rahmen(feld, st);
            } else {
                s.flaeche(feld);
            }
        }
        Teil::Ende(richtung) => {
            // Lokale Achsen: u in Fahrtrichtung, v quer dazu
            let (dr, dc) = richtung.schritt();
            let (ux, uy) = (dc as f32, dr as f32);
            let (vx, vy) = (-uy, ux);
            let bild = |u: f32, v: f32| (cx + u * ux + v * vx, cy + u * uy + v * vy);
            let mut ecken: Vec<(f32, f32)> = (0..=24)
                .map(|k| {
                    let w = std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 24.0;
                    bild(r * w.cos(), r * w.sin())
                })
                .collect();
            ecken.push(bild(aus, -r));
            ecken.push(bild(aus, r));
            if kontur {
                s.umriss(&ecken, st);
            } else {
                s.vieleck(&ecken, PaintMode::Fill);
            }
        }
    }
}
