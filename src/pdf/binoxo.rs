//! Binoxo zeichnen: gleichmäßiges Gitter, Vorgaben als gezeichnete X und O.

use super::zeichnen::{Rechteck, Seite, MM_PRO_PT};
use crate::raetsel::binoxo::{Binoxo, O, X};

pub fn zeichne_aufgabe(s: &Seite, b: &Binoxo, feld: Rechteck) {
    let n = b.n;
    let zelle = (feld.b.min(feld.h) / n as f32).min(9.0);
    let g = zelle * n as f32;
    let x0 = feld.x + (feld.b - g) / 2.0;
    let y0 = feld.y;

    s.linienfarbe(0.0);
    for k in 1..n {
        let d = k as f32 * zelle;
        s.linie(x0 + d, y0, x0 + d, y0 + g, 0.45);
        s.linie(x0, y0 + d, x0 + g, y0 + d, 0.45);
    }
    s.rahmen(Rechteck::neu(x0, y0, g, g), 1.8);

    let r = zelle * 0.26;
    let staerke = zelle * 0.13 / MM_PRO_PT;
    for (i, &v) in b.aufgabe.iter().enumerate() {
        let cx = x0 + (i % n) as f32 * zelle + zelle / 2.0;
        let cy = y0 + (i / n) as f32 * zelle + zelle / 2.0;
        match v {
            X => {
                s.linie(cx - r, cy - r, cx + r, cy + r, staerke);
                s.linie(cx - r, cy + r, cx + r, cy - r, staerke);
            }
            O => s.kreis(cx, cy, r * 1.15, staerke),
            _ => {}
        }
    }
}
