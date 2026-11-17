//! Nonogramme: Die Zahlen am Rand geben die Längen der Farbblöcke jeder Zeile und
//! Spalte in der richtigen Reihenfolge an. Zwischen zwei Blöcken gleicher Farbe liegt
//! mindestens ein leeres Feld, verschiedenfarbige Blöcke dürfen aneinanderstoßen.
//!
//! Die Bilder stammen aus der Motiv-Sammlung `motive.txt`. Ein Löser prüft jedes Motiv:
//! Lässt es sich mit Logik nicht eindeutig lösen, bekommt es ein paar vorgegebene Felder.
//! Jedes Motiv kann zusätzlich gespiegelt werden.

use super::Stufe;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::VecDeque;
use std::sync::OnceLock;

const MOTIVE: &str = include_str!("motive.txt");

#[derive(Clone, Debug)]
pub struct Motiv {
    pub name: String,
    pub breite: usize,
    pub hoehe: usize,
    /// Farbe 1, 2, … als RGB
    pub farben: Vec<[u8; 3]>,
    /// Zeilenweise; 0 = leer, sonst Farbnummer (1-basiert)
    pub pixel: Vec<u8>,
}

/// Ein Block am Rand: (Länge, Farbnummer)
pub type Block = (usize, u8);

#[derive(Clone, Debug)]
pub struct Nonogramm {
    pub name: String,
    pub breite: usize,
    pub hoehe: usize,
    pub farben: Vec<[u8; 3]>,
    pub loesung: Vec<u8>,
    pub zeilen: Vec<Vec<Block>>,
    pub spalten: Vec<Vec<Block>>,
    /// Felder, die schon eingezeichnet sind (nur wenn das Bild sonst nicht eindeutig wäre)
    pub vorgaben: Vec<usize>,
    pub stufe: Stufe,
    pub schwerste_technik: &'static str,
}

impl Nonogramm {
    pub fn farbig(&self) -> bool {
        self.farben.len() > 1
    }

    /// Deutsche Namen der Farben (für die Legende), automatisch aus dem RGB-Wert.
    pub fn farbnamen(&self) -> Vec<String> {
        const NAMEN: [(&str, [i32; 3]); 16] = [
            ("schwarz", [26, 26, 26]),
            ("grau", [128, 128, 128]),
            ("weiß", [250, 250, 250]),
            ("rot", [210, 40, 50]),
            ("dunkelrot", [130, 20, 30]),
            ("rosa", [230, 80, 140]),
            ("orange", [240, 140, 40]),
            ("gelb", [240, 200, 40]),
            ("beige", [230, 195, 140]),
            ("hellbraun", [200, 160, 110]),
            ("braun", [130, 85, 40]),
            ("grün", [40, 170, 90]),
            ("dunkelgrün", [30, 110, 55]),
            ("hellblau", [100, 180, 230]),
            ("blau", [40, 80, 200]),
            ("lila", [120, 60, 160]),
        ];
        let mut namen: Vec<String> = self
            .farben
            .iter()
            .map(|f| {
                let abstand = |n: &[i32; 3]| (0..3).map(|k| (f[k] as i32 - n[k]).pow(2)).sum::<i32>();
                NAMEN.iter().min_by_key(|(_, n)| abstand(n)).unwrap().0.to_string()
            })
            .collect();
        // Doppelte Namen unterscheidbar machen
        let vorher = namen.clone();
        for i in 0..namen.len() {
            if vorher.iter().filter(|n| **n == vorher[i]).count() > 1 {
                namen[i] = format!("{} {}", namen[i], i + 1);
            }
        }
        namen
    }
}

// ---------------------------------------------------------------------------
// Motiv-Sammlung
// ---------------------------------------------------------------------------

pub fn sammlung() -> &'static [Motiv] {
    static S: OnceLock<Vec<Motiv>> = OnceLock::new();
    S.get_or_init(|| lies_motive(MOTIVE).unwrap_or_else(|e| panic!("motive.txt: {e}")))
}

fn lies_motive(text: &str) -> Result<Vec<Motiv>, String> {
    struct Roh {
        name: String,
        palette: Vec<(char, [u8; 3])>,
        zeilen: Vec<String>,
    }
    fn fertig(r: Roh) -> Result<Motiv, String> {
        let palette = if r.palette.is_empty() { vec![('#', [0x1A, 0x1A, 0x1A])] } else { r.palette };
        let hoehe = r.zeilen.len();
        let breite = r.zeilen.first().map_or(0, |z| z.chars().count());
        if !(15..=25).contains(&breite) || !(15..=25).contains(&hoehe) {
            return Err(format!("{}: Größe {breite}×{hoehe} nicht in 15…25", r.name));
        }
        let mut pixel = Vec::with_capacity(breite * hoehe);
        for (k, z) in r.zeilen.iter().enumerate() {
            if z.chars().count() != breite {
                return Err(format!("{}: Zeile {} hat {} statt {breite} Zeichen", r.name, k + 1, z.chars().count()));
            }
            for ch in z.chars() {
                pixel.push(match ch {
                    '.' => 0,
                    _ => match palette.iter().position(|p| p.0 == ch) {
                        Some(p) => p as u8 + 1,
                        None => return Err(format!("{}: unbekanntes Zeichen '{ch}'", r.name)),
                    },
                });
            }
        }
        Ok(Motiv { name: r.name, breite, hoehe, farben: palette.iter().map(|p| p.1).collect(), pixel })
    }

    let mut motive = Vec::new();
    let mut aktuell: Option<Roh> = None;
    for zeile in text.lines() {
        let z = zeile.trim();
        if z.is_empty() || z.starts_with("//") {
            continue;
        }
        if let Some(name) = z.strip_prefix("===") {
            if let Some(r) = aktuell.take() {
                motive.push(fertig(r)?);
            }
            aktuell = Some(Roh { name: name.trim().to_string(), palette: Vec::new(), zeilen: Vec::new() });
            continue;
        }
        let Some(r) = aktuell.as_mut() else {
            return Err(format!("Zeile außerhalb eines Motivs: {z}"));
        };
        if let Some(f) = z.strip_prefix("farben:") {
            for token in f.split_whitespace() {
                let (c, hex) = token.split_once('=').ok_or(format!("{}: Farbe '{token}'", r.name))?;
                let wert = u32::from_str_radix(hex, 16).map_err(|_| format!("{}: Farbe '{token}'", r.name))?;
                let ch = c.chars().next().ok_or(format!("{}: Farbe '{token}'", r.name))?;
                r.palette.push((ch, [(wert >> 16) as u8, (wert >> 8) as u8, wert as u8]));
            }
            continue;
        }
        r.zeilen.push(z.to_string());
    }
    if let Some(r) = aktuell.take() {
        motive.push(fertig(r)?);
    }
    Ok(motive)
}

/// Blöcke einer Folge von Feldern.
fn bloecke(zellen: impl Iterator<Item = u8>) -> Vec<Block> {
    let mut v: Vec<Block> = Vec::new();
    let mut vorher = 0u8;
    for c in zellen {
        if c != 0 {
            match v.last_mut() {
                Some(b) if vorher == c => b.0 += 1,
                _ => v.push((1, c)),
            }
        }
        vorher = c;
    }
    v
}

fn randzahlen(pixel: &[u8], b: usize, h: usize) -> (Vec<Vec<Block>>, Vec<Vec<Block>>) {
    let zeilen = (0..h).map(|r| bloecke((0..b).map(|c| pixel[r * b + c]))).collect();
    let spalten = (0..b).map(|c| bloecke((0..h).map(|r| pixel[r * b + c]))).collect();
    (zeilen, spalten)
}

// ---------------------------------------------------------------------------
// Löser
// ---------------------------------------------------------------------------

struct Widerspruch;

/// Löst eine einzelne Linie vollständig: Welche Farben (Bit 0 = leer, Bit c = Farbe c)
/// sind in mindestens einer passenden Anordnung der Blöcke möglich?
fn loese_linie(zellen: &[u8], bloecke: &[Block]) -> Option<Vec<u8>> {
    let n = zellen.len();
    let m = bloecke.len();
    let passt = |start: usize, j: usize| -> Option<usize> {
        // Liefert die Position nach dem Block (inkl. Pflichtlücke vor gleichfarbigem Nachfolger)
        let (l, c) = bloecke[j];
        let ende = start + l;
        if ende > n || !zellen[start..ende].iter().all(|&z| z & (1 << c) != 0) {
            return None;
        }
        if j + 1 < m && bloecke[j + 1].1 == c {
            (ende < n && zellen[ende] & 1 != 0).then_some(ende + 1)
        } else {
            Some(ende)
        }
    };
    let idx = |i: usize, j: usize| i * (m + 1) + j;
    // vor[i][j]: die ersten i Felder passen mit den ersten j Blöcken
    let mut vor = vec![false; (n + 2) * (m + 1)];
    vor[idx(0, 0)] = true;
    for i in 0..n {
        for j in 0..=m {
            if !vor[idx(i, j)] {
                continue;
            }
            if zellen[i] & 1 != 0 {
                vor[idx(i + 1, j)] = true;
            }
            if j < m {
                if let Some(nach) = passt(i, j) {
                    vor[idx(nach, j + 1)] = true;
                }
            }
        }
    }
    if !vor[idx(n, m)] {
        return None;
    }
    // nach[i][j]: ab Feld i lassen sich die Blöcke j.. noch unterbringen
    let mut nach = vec![false; (n + 2) * (m + 1)];
    nach[idx(n, m)] = true;
    for i in (0..n).rev() {
        for j in 0..=m {
            let leer = zellen[i] & 1 != 0 && nach[idx(i + 1, j)];
            let block = j < m && passt(i, j).is_some_and(|p| nach[idx(p, j + 1)]);
            nach[idx(i, j)] = leer || block;
        }
    }
    let mut moeglich = vec![0u8; n];
    for i in 0..n {
        for j in 0..=m {
            if !vor[idx(i, j)] {
                continue;
            }
            if zellen[i] & 1 != 0 && nach[idx(i + 1, j)] {
                moeglich[i] |= 1;
            }
            if j < m {
                if let Some(p) = passt(i, j) {
                    if nach[idx(p, j + 1)] {
                        let (l, c) = bloecke[j];
                        for k in i..i + l {
                            moeglich[k] |= 1 << c;
                        }
                        if p > i + l {
                            moeglich[i + l] |= 1;
                        }
                    }
                }
            }
        }
    }
    moeglich.iter().all(|&x| x != 0).then_some(moeglich)
}

#[derive(Clone)]
struct Loeser<'a> {
    b: usize,
    h: usize,
    zeilen: &'a [Vec<Block>],
    spalten: &'a [Vec<Block>],
    /// Je Feld die noch möglichen Farben als Bitmaske
    z: Vec<u8>,
}

impl<'a> Loeser<'a> {
    fn neu(b: usize, h: usize, farben: usize, zeilen: &'a [Vec<Block>], spalten: &'a [Vec<Block>]) -> Self {
        let voll = ((1u16 << (farben + 1)) - 1) as u8;
        Loeser { b, h, zeilen, spalten, z: vec![voll; b * h] }
    }

    fn linie(&self, l: usize) -> Vec<usize> {
        if l < self.h {
            (0..self.b).map(|c| l * self.b + c).collect()
        } else {
            (0..self.h).map(|r| r * self.b + (l - self.h)).collect()
        }
    }

    /// Löst Linien, bis sich nichts mehr ändert (nur betroffene Linien werden neu angesehen).
    fn folgere(&mut self, start: &[usize]) -> Result<(), Widerspruch> {
        let anzahl = self.h + self.b;
        let mut wartet = vec![false; anzahl];
        let mut schlange = VecDeque::new();
        for &l in start {
            if !wartet[l] {
                wartet[l] = true;
                schlange.push_back(l);
            }
        }
        while let Some(l) = schlange.pop_front() {
            wartet[l] = false;
            let felder = self.linie(l);
            let alt: Vec<u8> = felder.iter().map(|&i| self.z[i]).collect();
            let bl = if l < self.h { &self.zeilen[l] } else { &self.spalten[l - self.h] };
            let neu = loese_linie(&alt, bl).ok_or(Widerspruch)?;
            for k in 0..felder.len() {
                if neu[k] != alt[k] {
                    self.z[felder[k]] = neu[k];
                    let quer = if l < self.h { self.h + k } else { k };
                    if !wartet[quer] {
                        wartet[quer] = true;
                        schlange.push_back(quer);
                    }
                }
            }
        }
        Ok(())
    }

    fn linien_von(&self, i: usize) -> [usize; 2] {
        [i / self.b, self.h + i % self.b]
    }

    fn geloest(&self) -> bool {
        self.z.iter().all(|m| m.count_ones() == 1)
    }

    /// Setzt eine Farbe probeweise; führt das zum Widerspruch, fällt sie weg.
    fn annahme(&mut self) -> Result<bool, Widerspruch> {
        for i in 0..self.z.len() {
            let maske = self.z[i];
            if maske.count_ones() < 2 {
                continue;
            }
            for bit in (0..8).map(|k| 1u8 << k).filter(|b| maske & b != 0) {
                let mut probe = self.clone();
                probe.z[i] = bit;
                if probe.folgere(&probe.linien_von(i)).is_err() {
                    self.z[i] &= !bit;
                    self.folgere(&self.linien_von(i))?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

// ---------------------------------------------------------------------------
// Bewertung und Katalog
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Bewertung {
    pub vorgaben: Vec<usize>,
    pub annahme: bool,
    pub stufe: Stufe,
}

/// Löst das Motiv; wo die Logik nicht weiterkommt, wird ein Feld vorgegeben.
pub fn bewerte(m: &Motiv) -> Bewertung {
    let (zeilen, spalten) = randzahlen(&m.pixel, m.breite, m.hoehe);
    let mut l = Loeser::neu(m.breite, m.hoehe, m.farben.len(), &zeilen, &spalten);
    let alle: Vec<usize> = (0..m.breite + m.hoehe).collect();
    if l.folgere(&alle).is_err() {
        panic!("Motiv {} ist widersprüchlich", m.name);
    }
    let mut vorgaben = Vec::new();
    let mut annahme = false;
    while !l.geloest() {
        match l.annahme() {
            Ok(true) => annahme = true,
            Ok(false) => {
                // Festgefahren: ein offenes Feld vorgeben – bevorzugt ein gefülltes
                let offen: Vec<usize> = (0..l.z.len()).filter(|&i| l.z[i].count_ones() > 1).collect();
                let gefuellt: Vec<usize> = offen.iter().copied().filter(|&i| m.pixel[i] != 0).collect();
                let wahl = if gefuellt.is_empty() { offen[offen.len() / 2] } else { gefuellt[gefuellt.len() / 2] };
                l.z[wahl] = 1 << m.pixel[wahl];
                vorgaben.push(wahl);
                if l.folgere(&l.linien_von(wahl)).is_err() {
                    panic!("Motiv {}: Löser widerspricht sich", m.name);
                }
            }
            Err(_) => panic!("Motiv {}: Löser widerspricht sich", m.name),
        }
    }
    debug_assert!((0..l.z.len()).all(|i| l.z[i] == 1 << m.pixel[i]));
    vorgaben.sort();
    // Größe bestimmt die Grundstufe, nötige Annahmen machen es eine Stufe schwerer
    let groesse = m.breite.max(m.hoehe);
    let basis = if groesse <= 15 { 2 } else if groesse <= 20 { 3 } else { 4 };
    let stufe = Stufe::ALLE[(basis + annahme as usize).min(4)];
    Bewertung { vorgaben, annahme, stufe }
}

/// Alle Motive mit Bewertung – beim ersten Gebrauch parallel berechnet.
pub fn katalog() -> &'static [(&'static Motiv, Bewertung)] {
    static K: OnceLock<Vec<(&'static Motiv, Bewertung)>> = OnceLock::new();
    K.get_or_init(|| {
        let motive = sammlung();
        std::thread::scope(|sc| {
            let faeden: Vec<_> = motive.iter().map(|m| sc.spawn(move || (m, bewerte(m)))).collect();
            faeden.into_iter().map(|f| f.join().expect("Bewertung")).collect()
        })
    })
}

/// Wählt zu jeder gewünschten Stufe ein Nonogramm – ohne Motiv-Wiederholung im Heft,
/// solange die Sammlung reicht. Passt keine Stufe genau, wird die nächstliegende genommen.
pub fn waehle<R: Rng>(stufen: &[Stufe], rng: &mut R) -> Vec<Nonogramm> {
    let kat = katalog();
    let mut frei: Vec<usize> = Vec::new();
    stufen
        .iter()
        .map(|&s| {
            if frei.is_empty() {
                frei = (0..kat.len()).collect();
                frei.shuffle(rng);
            }
            // Zuerst die Motive mit der besten Stufen-Nähe bestimmen …
            let dist = |p: usize| (kat[frei[p]].1.stufe as i32 - s as i32).abs();
            let beste = (0..frei.len()).map(dist).min().unwrap();
            let kandidaten: Vec<usize> = (0..frei.len()).filter(|&p| dist(p) == beste).collect();
            // … und darunter die weniger symmetrischen bevorzugen (zu symmetrische Motive sind fad)
            let gewichte: Vec<f32> = kandidaten
                .iter()
                .map(|&p| (1.05 - spiegel_symmetrie(kat[frei[p]].0)).max(0.12))
                .collect();
            let pos = kandidaten[gewichtete_wahl(&gewichte, rng)];
            let (motiv, bewertung) = &kat[frei.remove(pos)];
            baue(motiv, bewertung, rng.gen_bool(0.5))
        })
        .collect()
}

/// Wählt einen Index aus `gewichte` zufällig, proportional zu den Gewichten.
fn gewichtete_wahl<R: Rng>(gewichte: &[f32], rng: &mut R) -> usize {
    let summe: f32 = gewichte.iter().sum();
    let mut ziel = rng.gen::<f32>() * summe;
    for (i, &g) in gewichte.iter().enumerate() {
        ziel -= g;
        if ziel <= 0.0 {
            return i;
        }
    }
    gewichte.len() - 1
}

/// Wie spiegelsymmetrisch ein Motiv ist (0 = gar nicht, 1 = perfekt): das Maximum über
/// waagrechte, senkrechte und 180°-Spiegelung, nur über die gezeichneten Felder gemessen
/// (leere Ränder zählen nicht mit, sonst wirkte jedes Motiv symmetrischer als es ist).
fn spiegel_symmetrie(m: &Motiv) -> f32 {
    let (b, h) = (m.breite, m.hoehe);
    let messe = |idx: &dyn Fn(usize, usize) -> usize| -> f32 {
        let (mut relevant, mut gleich) = (0usize, 0usize);
        for r in 0..h {
            for c in 0..b {
                let a = m.pixel[r * b + c];
                let sp = m.pixel[idx(r, c)];
                if a != 0 || sp != 0 {
                    relevant += 1;
                    if a == sp {
                        gleich += 1;
                    }
                }
            }
        }
        if relevant == 0 { 0.0 } else { gleich as f32 / relevant as f32 }
    };
    let waagrecht = messe(&|r, c| r * b + (b - 1 - c));
    let senkrecht = messe(&|r, c| (h - 1 - r) * b + c);
    let gedreht = messe(&|r, c| (h - 1 - r) * b + (b - 1 - c));
    waagrecht.max(senkrecht).max(gedreht)
}

fn baue(m: &Motiv, bew: &Bewertung, spiegeln: bool) -> Nonogramm {
    let (b, h) = (m.breite, m.hoehe);
    let abbild = |i: usize| if spiegeln { (i / b) * b + (b - 1 - i % b) } else { i };
    let loesung: Vec<u8> = (0..b * h).map(|i| m.pixel[abbild(i)]).collect();
    let mut vorgaben: Vec<usize> = bew.vorgaben.iter().map(|&v| abbild(v)).collect();
    vorgaben.sort();
    let (zeilen, spalten) = randzahlen(&loesung, b, h);
    Nonogramm {
        name: m.name.clone(),
        breite: b,
        hoehe: h,
        farben: m.farben.clone(),
        loesung,
        zeilen,
        spalten,
        vorgaben,
        stufe: bew.stufe,
        schwerste_technik: if bew.annahme { "Annahme mit Widerspruch" } else { "Linien durchdenken" },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linie_einfach() {
        // 10 Felder, ein 8er-Block: die mittleren 6 stehen fest
        let z = vec![0b11u8; 10];
        let m = loese_linie(&z, &[(8, 1)]).unwrap();
        assert_eq!(m.iter().filter(|&&x| x == 0b10).count(), 6);
        // gleichfarbige Blöcke brauchen eine Lücke, verschiedenfarbige nicht
        assert!(loese_linie(&[0b111; 4], &[(2, 1), (2, 1)]).is_none());
        let m = loese_linie(&[0b111; 4], &[(2, 1), (2, 2)]).unwrap();
        assert_eq!(m, vec![0b010, 0b010, 0b100, 0b100]);
    }

    #[test]
    fn alle_motive_eindeutig_loesbar() {
        for (m, bew) in katalog() {
            let (zeilen, spalten) = randzahlen(&m.pixel, m.breite, m.hoehe);
            // Mit den Vorgaben muss die reine Logik (inkl. Annahmen) genau das Bild ergeben
            let mut l = Loeser::neu(m.breite, m.hoehe, m.farben.len(), &zeilen, &spalten);
            for &v in &bew.vorgaben {
                l.z[v] = 1 << m.pixel[v];
            }
            let alle: Vec<usize> = (0..m.breite + m.hoehe).collect();
            assert!(l.folgere(&alle).is_ok());
            while !l.geloest() {
                assert!(matches!(l.annahme(), Ok(true)), "{} bleibt stecken", m.name);
            }
            assert!((0..l.z.len()).all(|i| l.z[i] == 1 << m.pixel[i]), "{}", m.name);
            eprintln!(
                "{:<14} {:>2}×{:<2} {:<12} Vorgaben: {:>2}  Annahme: {}",
                m.name,
                m.breite,
                m.hoehe,
                bew.stufe.name(),
                bew.vorgaben.len(),
                bew.annahme
            );
        }
    }
}
