//! Nonogramm zeichnen: Zahlen links und oben (grau hinterlegt), Gitter mit dickeren
//! Linien alle fünf Felder. Farbige Rätsel: Zahlen in Farbkästchen, jede Farbe zusätzlich
//! mit eigenem Muster (damit es auch schwarz-weiß gedruckt lesbar bleibt) und eine Legende
//! mit Muster und Farbnamen.

use super::zeichnen::{Ausrichtung, Rechteck, Schrift, Seite, MM_PRO_PT};
use crate::raetsel::nonogramm::Nonogramm;

fn setze_farbe(s: &Seite, ng: &Nonogramm, farbe: u8) {
    if ng.farbig() {
        let [r, g, b] = ng.farben[farbe as usize - 1];
        s.fuellfarbe_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    } else {
        s.fuellfarbe(0.1);
    }
}

/// Helle Farben bekommen schwarze Zahlen und Muster, dunkle weiße.
fn ist_hell(rgb: [u8; 3]) -> bool {
    0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32 > 150.0
}

/// Farbfläche samt Muster der Farbe (Farbe 1 = einfarbig, dann Schraffuren, Punkte, Streifen …).
fn farbflaeche(s: &Seite, ng: &Nonogramm, farbe: u8, r: Rechteck) {
    setze_farbe(s, ng, farbe);
    s.flaeche(r);
    if !ng.farbig() {
        return;
    }
    let hell = ist_hell(ng.farben[farbe as usize - 1]);
    let kontrast = if hell { 0.1 } else { 1.0 };
    s.linienfarbe(kontrast);
    let a = (r.b.min(r.h) / 3.2).clamp(0.6, 1.4); // Linienabstand
    let st = 0.4; // Strichstärke in pt
    match (farbe - 1) % 7 {
        0 => {}
        1 => schraffur(s, r, a, true, st),
        2 => {
            s.fuellfarbe(kontrast);
            let mut y = r.y + a / 2.0;
            while y < r.y + r.h {
                let mut x = r.x + a / 2.0;
                while x < r.x + r.b {
                    s.scheibe(x, y, a * 0.17);
                    x += a;
                }
                y += a;
            }
        }
        3 => {
            let mut y = r.y + a / 2.0;
            while y < r.y + r.h {
                s.linie(r.x, y, r.x + r.b, y, st);
                y += a;
            }
        }
        4 => {
            schraffur(s, r, a * 1.2, true, st);
            schraffur(s, r, a * 1.2, false, st);
        }
        5 => schraffur(s, r, a, false, st),
        _ => {
            let mut x = r.x + a / 2.0;
            while x < r.x + r.b {
                s.linie(x, r.y, x, r.y + r.h, st);
                x += a;
            }
        }
    }
    s.linienfarbe(0.0);
    setze_farbe(s, ng, farbe);
}

/// Schräge Linien im Abstand `a`, auf das Rechteck beschnitten.
/// `steigend` = „/“, sonst „\“ (y zählt nach unten).
fn schraffur(s: &Seite, r: Rechteck, a: f32, steigend: bool, st: f32) {
    let schritt = a * std::f32::consts::SQRT_2;
    if steigend {
        // Linien x + y = c
        let mut c = r.x + r.y + schritt / 2.0;
        while c < r.x + r.b + r.y + r.h {
            let lo = r.x.max(c - r.y - r.h);
            let hi = (r.x + r.b).min(c - r.y);
            if lo < hi {
                s.linie(lo, c - lo, hi, c - hi, st);
            }
            c += schritt;
        }
    } else {
        // Linien x - y = c
        let mut c = r.x - r.y - r.h + schritt / 2.0;
        while c < r.x + r.b - r.y {
            let lo = r.x.max(c + r.y);
            let hi = (r.x + r.b).min(c + r.y + r.h);
            if lo < hi {
                s.linie(lo, lo - c, hi, hi - c, st);
            }
            c += schritt;
        }
    }
}

fn zahl(s: &Seite, ng: &Nonogramm, laenge: usize, farbe: u8, cx: f32, cy: f32, z: f32) {
    let pt = z / MM_PRO_PT * if laenge >= 10 { 0.47 } else { 0.56 };
    let text = laenge.to_string();
    if ng.farbig() {
        let k = z * 0.86;
        farbflaeche(s, ng, farbe, Rechteck::neu(cx - k / 2.0, cy - k / 2.0, k, k));
        // Die Zahl steht auf einem ruhigen Fleck in der Farbe, damit das Muster nicht stört
        let tb = s.schriften.breite(Schrift::Fett, &text, pt) + 0.5;
        let th = s.schriften.versalhoehe(Schrift::Fett, pt) + 0.7;
        s.flaeche(Rechteck::neu(cx - tb / 2.0, cy - th / 2.0, tb, th));
        s.fuellfarbe(if ist_hell(ng.farben[farbe as usize - 1]) { 0.0 } else { 1.0 });
    } else {
        s.fuellfarbe(0.0);
    }
    s.text_mittig(&text, cx, cy, pt, Schrift::Fett);
}

pub fn zeichne_aufgabe(s: &Seite, ng: &Nonogramm, feld: Rechteck) {
    let (b, h) = (ng.breite, ng.hoehe);
    let mr = ng.zeilen.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let mc = ng.spalten.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let legende = if ng.farbig() { 18.0 } else { 0.0 };
    let z = (feld.b / (mr + b) as f32).min((feld.h - legende) / (mc + h) as f32).min(7.5);
    let gesamt_b = (mr + b) as f32 * z;
    let x0 = feld.x + (feld.b - gesamt_b) / 2.0;
    let (gx, gy) = (x0 + mr as f32 * z, feld.y + mc as f32 * z);
    let (gb, gh) = (b as f32 * z, h as f32 * z);

    // Zahlenbereiche hell hinterlegen, mit feinen Hilfslinien
    s.fuellfarbe(0.94);
    s.flaeche(Rechteck::neu(x0, gy, mr as f32 * z, gh));
    s.flaeche(Rechteck::neu(gx, feld.y, gb, mc as f32 * z));
    s.linienfarbe(0.78);
    for r in 0..=h {
        let y = gy + r as f32 * z;
        s.linie(x0, y, gx, y, 0.3);
    }
    for c in 0..=b {
        let x = gx + c as f32 * z;
        s.linie(x, feld.y, x, gy, 0.3);
    }

    // Vorgegebene Felder
    for &i in &ng.vorgaben {
        let (x, y) = (gx + (i % b) as f32 * z, gy + (i / b) as f32 * z);
        if ng.loesung[i] == 0 {
            s.fuellfarbe(0.0);
            s.scheibe(x + z / 2.0, y + z / 2.0, z * 0.1);
        } else {
            farbflaeche(s, ng, ng.loesung[i], Rechteck::neu(x, y, z, z));
        }
    }

    // Gitter: dickere Linien alle fünf Felder
    s.linienfarbe(0.0);
    for r in 1..h {
        let y = gy + r as f32 * z;
        s.linie(gx, y, gx + gb, y, if r % 5 == 0 { 1.1 } else { 0.35 });
    }
    for c in 1..b {
        let x = gx + c as f32 * z;
        s.linie(x, gy, x, gy + gh, if c % 5 == 0 { 1.1 } else { 0.35 });
    }
    s.rahmen(Rechteck::neu(gx, gy, gb, gh), 1.5);

    // Zahlen: rechtsbündig bzw. unten bündig am Gitter; leere Linien bekommen eine 0
    for (r, bl) in ng.zeilen.iter().enumerate() {
        let cy = gy + (r as f32 + 0.5) * z;
        if bl.is_empty() {
            s.fuellfarbe(0.0);
            s.text_mittig("0", gx - z / 2.0, cy, z / MM_PRO_PT * 0.56, Schrift::Fett);
        }
        for (k, &(l, f)) in bl.iter().enumerate() {
            let slot = mr - bl.len() + k;
            zahl(s, ng, l, f, x0 + (slot as f32 + 0.5) * z, cy, z);
        }
    }
    for (c, bl) in ng.spalten.iter().enumerate() {
        let cx = gx + (c as f32 + 0.5) * z;
        if bl.is_empty() {
            s.fuellfarbe(0.0);
            s.text_mittig("0", cx, gy - z / 2.0, z / MM_PRO_PT * 0.56, Schrift::Fett);
        }
        for (k, &(l, f)) in bl.iter().enumerate() {
            let slot = mc - bl.len() + k;
            zahl(s, ng, l, f, cx, feld.y + (slot as f32 + 0.5) * z, z);
        }
    }

    if ng.farbig() {
        legende_zeichnen(s, ng, feld, gy + gh + 7.0);
    }
    s.fuellfarbe(0.0);
}

/// Legende: Kästchen mit Farbe und Muster, daneben der Farbname; bei Platzmangel zweizeilig.
fn legende_zeichnen(s: &Seite, ng: &Nonogramm, feld: Rechteck, y: f32) {
    let pt = 9.0;
    let k = 5.0;
    let namen = ng.farbnamen();
    let eintrag_b = |name: &str| k + 1.5 + s.schriften.breite(Schrift::Normal, name, pt) + 5.0;
    // Einträge auf Zeilen verteilen
    let mut zeilen: Vec<Vec<usize>> = vec![Vec::new()];
    let mut breite = 0.0;
    for (i, name) in namen.iter().enumerate() {
        let b = eintrag_b(name);
        if breite + b > feld.b && !zeilen.last().unwrap().is_empty() {
            zeilen.push(Vec::new());
            breite = 0.0;
        }
        zeilen.last_mut().unwrap().push(i);
        breite += b;
    }
    s.linienfarbe(0.4);
    for (zeile, eintraege) in zeilen.iter().enumerate() {
        let gesamt: f32 = eintraege.iter().map(|&i| eintrag_b(&namen[i])).sum::<f32>() - 5.0;
        let mut x = feld.x + (feld.b - gesamt) / 2.0;
        let yz = y + zeile as f32 * (k + 2.5);
        for &i in eintraege {
            let r = Rechteck::neu(x, yz - k / 2.0, k, k);
            farbflaeche(s, ng, i as u8 + 1, r);
            s.linienfarbe(0.4);
            s.rahmen(r, 0.3);
            s.fuellfarbe(0.2);
            s.text(&namen[i], x + k + 1.5, yz + 1.2, pt, Schrift::Normal, Ausrichtung::Links);
            x += eintrag_b(&namen[i]);
        }
    }
    s.linienfarbe(0.0);
}
