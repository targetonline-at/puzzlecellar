//! Sudoku zeichnen.

use super::zeichnen::{Rechteck, Schrift, Seite, MM_PRO_PT};
use crate::raetsel::sudoku::Sudoku;

const DICK: f32 = 1.8;
const DUENN: f32 = 0.45;

fn gitter(s: &Seite, x0: f32, y0: f32, zelle: f32) {
    let g = zelle * 9.0;
    s.linienfarbe(0.0);
    for k in 1..9 {
        if k % 3 == 0 {
            continue;
        }
        let d = k as f32 * zelle;
        s.linie(x0 + d, y0, x0 + d, y0 + g, DUENN);
        s.linie(x0, y0 + d, x0 + g, y0 + d, DUENN);
    }
    for k in [3, 6] {
        let d = k as f32 * zelle;
        s.linie(x0 + d, y0, x0 + d, y0 + g, DICK);
        s.linie(x0, y0 + d, x0 + g, y0 + d, DICK);
    }
    s.rahmen(Rechteck::neu(x0, y0, g, g), DICK);
}

pub fn zeichne_aufgabe(s: &Seite, sd: &Sudoku, feld: Rechteck) {
    let zelle = (feld.b.min(feld.h) / 9.0).min(12.0);
    let x0 = feld.x + (feld.b - zelle * 9.0) / 2.0;
    let y0 = feld.y;
    gitter(s, x0, y0, zelle);
    let groesse = zelle / MM_PRO_PT * 0.6;
    s.fuellfarbe(0.0);
    for i in 0..81 {
        let d = sd.aufgabe[i];
        if d != 0 {
            let cx = x0 + (i % 9) as f32 * zelle + zelle / 2.0;
            let cy = y0 + (i / 9) as f32 * zelle + zelle / 2.0;
            s.text_mittig(&d.to_string(), cx, cy, groesse, Schrift::Normal);
        }
    }
}
