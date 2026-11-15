//! Binoxo: das Gitter so mit X und O füllen, dass
//! – nie mehr als zwei gleiche Zeichen nebeneinander oder untereinander stehen,
//! – jede Zeile und jede Spalte gleich viele X und O enthält,
//! – keine zwei Zeilen und keine zwei Spalten gleich sind.
//!
//! Erzeugt wird wie beim Sudoku: zuerst ein volles, gültiges Gitter, dann werden
//! Zeichen entfernt, solange ein „menschlicher“ Löser mit den Techniken der
//! gewünschten Stufe noch zum Ziel kommt. Weil jeder Schritt dieses Lösers eine
//! sichere Folgerung ist, hat jedes erzeugte Rätsel genau eine Lösung.

use super::Stufe;
use rand::seq::SliceRandom;
use rand::Rng;
use std::sync::OnceLock;

pub const LEER: u8 = 0;
pub const X: u8 = 1;
pub const O: u8 = 2;

#[derive(Clone, Debug)]
pub struct Binoxo {
    /// Kantenlänge (10 oder 12)
    pub n: usize,
    /// Zeilenweise, `LEER`/`X`/`O`
    pub aufgabe: Vec<u8>,
    pub loesung: Vec<u8>,
    pub stufe: Stufe,
    pub schwerste_technik: &'static str,
}

/// Erzeugt ein Binoxo der gewünschten Stufe, zufällig 10×10 oder 12×12.
pub fn erzeuge<R: Rng>(stufe: Stufe, rng: &mut R) -> Binoxo {
    let n = if rng.gen_bool(0.5) { 10 } else { 12 };
    erzeuge_groesse(n, stufe, rng)
}

pub fn erzeuge_groesse<R: Rng>(n: usize, stufe: Stufe, rng: &mut R) -> Binoxo {
    let ziel = Technik::fuer(stufe);
    loop {
        let loesung = volles_gitter(n, rng);
        let aufgabe = ausduennen(&loesung, n, ziel, rng);
        let mut f = Feld { n, z: aufgabe.clone() };
        if f.loese(ziel) == Some(ziel) {
            debug_assert_eq!(f.z, loesung);
            return Binoxo { n, aufgabe, loesung, stufe, schwerste_technik: ziel.name() };
        }
    }
}

fn gegen(v: u8) -> u8 {
    if v == X {
        O
    } else {
        X
    }
}

// ---------------------------------------------------------------------------
// Gültige Linien
// ---------------------------------------------------------------------------

/// Alle gültigen vollen Linien der Länge `n` als Bitmuster (Bit k gesetzt = X an Stelle k):
/// gleich viele X und O, nie drei gleiche hintereinander. Für n = 12 sind das nur ein paar Hundert.
fn linien(n: usize) -> &'static [u16] {
    static TAB: OnceLock<Vec<Vec<u16>>> = OnceLock::new();
    let tab = TAB.get_or_init(|| {
        (0..=14usize)
            .map(|n| {
                if n % 2 == 1 {
                    return Vec::new();
                }
                (0..(1u32 << n)).map(|m| m as u16).filter(|&m| linie_gueltig(m, n)).collect()
            })
            .collect()
    });
    &tab[n]
}

fn linie_gueltig(m: u16, n: usize) -> bool {
    let voll = ((1u32 << n) - 1) as u16;
    let o = !m & voll;
    m.count_ones() as usize == n / 2
        && (m & (m >> 1) & (m >> 2)) == 0
        && (o & (o >> 1) & (o >> 2)) == 0
}

// ---------------------------------------------------------------------------
// Volles Gitter
// ---------------------------------------------------------------------------

/// Zufälliges gültiges Gitter: Zeile für Zeile aus den gültigen Linien,
/// mit Rücksicht auf die Spalten.
fn volles_gitter<R: Rng>(n: usize, rng: &mut R) -> Vec<u8> {
    loop {
        let mut zeilen = Vec::with_capacity(n);
        let mut budget = 5_000u32;
        if fuelle_zeilen(&mut zeilen, n, rng, &mut budget) {
            let mut z = vec![LEER; n * n];
            for (r, &zeile) in zeilen.iter().enumerate() {
                for c in 0..n {
                    z[r * n + c] = if zeile >> c & 1 == 1 { X } else { O };
                }
            }
            return z;
        }
    }
}

fn fuelle_zeilen<R: Rng>(zeilen: &mut Vec<u16>, n: usize, rng: &mut R, budget: &mut u32) -> bool {
    if zeilen.len() == n {
        return spalten_verschieden(zeilen, n);
    }
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    let mut auswahl = linien(n).to_vec();
    auswahl.shuffle(rng);
    for zeile in auswahl {
        if zeilen.contains(&zeile) {
            continue;
        }
        zeilen.push(zeile);
        if spalten_moeglich(zeilen, n) && fuelle_zeilen(zeilen, n, rng, budget) {
            return true;
        }
        zeilen.pop();
        if *budget == 0 {
            return false;
        }
    }
    false
}

/// Spalten des teilweise gefüllten Gitters: keine Dreier, nicht zu viele X oder O.
fn spalten_moeglich(zeilen: &[u16], n: usize) -> bool {
    let h = zeilen.len();
    (0..n).all(|c| {
        let bit = |z: u16| z >> c & 1;
        let x = zeilen.iter().filter(|&&z| bit(z) == 1).count();
        let dreier = h >= 3 && {
            let a = bit(zeilen[h - 1]);
            a == bit(zeilen[h - 2]) && a == bit(zeilen[h - 3])
        };
        x <= n / 2 && h - x <= n / 2 && !dreier
    })
}

fn spalten_verschieden(zeilen: &[u16], n: usize) -> bool {
    let spalte = |c: usize| zeilen.iter().enumerate().fold(0u16, |m, (r, &z)| m | (z >> c & 1) << r);
    let spalten: Vec<u16> = (0..n).map(spalte).collect();
    (0..n).all(|a| (a + 1..n).all(|b| spalten[a] != spalten[b]))
}

// ---------------------------------------------------------------------------
// Ausdünnen
// ---------------------------------------------------------------------------

fn ausduennen<R: Rng>(loesung: &[u8], n: usize, ziel: Technik, rng: &mut R) -> Vec<u8> {
    let mut aufgabe = loesung.to_vec();
    // Für „teuflisch“ erst mit den schnellen Techniken ausdünnen, dann mit Annahmen
    // weitermachen – das spart viel Rechenzeit.
    let phasen: &[Technik] = if ziel == Technik::Annahme {
        &[Technik::Einzigartig, Technik::Annahme]
    } else {
        std::slice::from_ref(&ziel)
    };
    for &t in phasen {
        let mut reihenfolge: Vec<usize> = (0..n * n).filter(|&i| aufgabe[i] != LEER).collect();
        reihenfolge.shuffle(rng);
        for i in reihenfolge {
            let alt = aufgabe[i];
            aufgabe[i] = LEER;
            if (Feld { n, z: aufgabe.clone() }).loese(t).is_none() {
                aufgabe[i] = alt;
            }
        }
    }
    aufgabe
}

// ---------------------------------------------------------------------------
// Menschlicher Löser
// ---------------------------------------------------------------------------

/// Lösungstechniken, nach Schwierigkeit sortiert.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Technik {
    /// XX_ → O daneben, X_X → O dazwischen
    Nachbarn,
    /// Linie hat schon alle X → Rest O (und umgekehrt)
    Zaehlen,
    /// Alle Möglichkeiten einer Linie durchdenken (Dreier- und Zählregel zusammen)
    Linie,
    /// Wie `Linie`, zusätzlich darf die Linie keiner fertigen gleichen
    Einzigartig,
    /// Zeichen probeweise setzen, bis ein Widerspruch entsteht
    Annahme,
}

impl Technik {
    const ALLE: [Technik; 5] = [
        Technik::Nachbarn,
        Technik::Zaehlen,
        Technik::Linie,
        Technik::Einzigartig,
        Technik::Annahme,
    ];

    fn stufe(self) -> Stufe {
        Stufe::ALLE[self as usize]
    }

    fn fuer(stufe: Stufe) -> Technik {
        Technik::ALLE[stufe as usize]
    }

    fn name(self) -> &'static str {
        match self {
            Technik::Nachbarn => "Paare und Lücken",
            Technik::Zaehlen => "Zählen",
            Technik::Linie => "Linie ganz durchdenken",
            Technik::Einzigartig => "keine doppelten Zeilen",
            Technik::Annahme => "Annahme mit Widerspruch",
        }
    }
}

struct Widerspruch;

#[derive(Clone)]
struct Feld {
    n: usize,
    z: Vec<u8>,
}

impl Feld {
    /// Zellindex der k-ten Zelle von Linie l (0..n Zeilen, n..2n Spalten).
    fn idx(&self, l: usize, k: usize) -> usize {
        if l < self.n {
            l * self.n + k
        } else {
            k * self.n + (l - self.n)
        }
    }

    /// (X-Maske, O-Maske) der Linie l.
    fn masken(&self, l: usize) -> (u16, u16) {
        let (mut x, mut o) = (0u16, 0u16);
        for k in 0..self.n {
            match self.z[self.idx(l, k)] {
                X => x |= 1 << k,
                O => o |= 1 << k,
                _ => {}
            }
        }
        (x, o)
    }

    fn voll(&self) -> bool {
        !self.z.contains(&LEER)
    }

    /// Wendet eine Technik einmal auf alle Linien an. `Ok(true)`, wenn etwas eingetragen wurde.
    fn anwenden(&mut self, t: Technik) -> Result<bool, Widerspruch> {
        match t {
            Technik::Nachbarn => self.nachbarn(),
            Technik::Zaehlen => self.zaehlen(),
            Technik::Linie => self.linien_durchdenken(false),
            Technik::Einzigartig => self.linien_durchdenken(true),
            Technik::Annahme => self.annahme(),
        }
    }

    fn nachbarn(&mut self) -> Result<bool, Widerspruch> {
        let n = self.n;
        let mut neu = false;
        for l in 0..2 * n {
            for k in 0..n - 2 {
                let i = [self.idx(l, k), self.idx(l, k + 1), self.idx(l, k + 2)];
                let w = i.map(|j| self.z[j]);
                match w.iter().filter(|&&v| v == LEER).count() {
                    0 if w[0] == w[1] && w[1] == w[2] => return Err(Widerspruch),
                    1 => {
                        let lueke = w.iter().position(|&v| v == LEER).unwrap();
                        let andere: Vec<u8> = w.iter().copied().filter(|&v| v != LEER).collect();
                        if andere[0] == andere[1] {
                            self.z[i[lueke]] = gegen(andere[0]);
                            neu = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(neu)
    }

    fn zaehlen(&mut self) -> Result<bool, Widerspruch> {
        let n = self.n;
        let mut neu = false;
        for l in 0..2 * n {
            let (x, o) = self.masken(l);
            let (cx, co) = (x.count_ones() as usize, o.count_ones() as usize);
            if cx > n / 2 || co > n / 2 {
                return Err(Widerspruch);
            }
            if cx + co == n {
                continue;
            }
            let fuellen = if cx == n / 2 {
                O
            } else if co == n / 2 {
                X
            } else {
                continue;
            };
            for k in 0..n {
                let i = self.idx(l, k);
                if self.z[i] == LEER {
                    self.z[i] = fuellen;
                }
            }
            neu = true;
        }
        Ok(neu)
    }

    /// Prüft für jede Linie alle gültigen Belegungen, die zu den eingetragenen Zeichen passen.
    /// Was in allen gleich ist, wird eingetragen. Mit `einzig` scheiden Belegungen aus,
    /// die einer fertigen anderen Linie gleicher Richtung gleichen.
    fn linien_durchdenken(&mut self, einzig: bool) -> Result<bool, Widerspruch> {
        let n = self.n;
        let voll = ((1u32 << n) - 1) as u16;
        let mut neu = false;
        for l in 0..2 * n {
            let (x, o) = self.masken(l);
            let fertige: Vec<u16> = if einzig {
                let basis = if l < n { 0 } else { n };
                (basis..basis + n)
                    .filter(|&m| m != l)
                    .map(|m| self.masken(m))
                    .filter(|&(mx, mo)| mx | mo == voll)
                    .map(|(mx, _)| mx)
                    .collect()
            } else {
                Vec::new()
            };
            let (mut alle, mut irgend, mut passend) = (voll, 0u16, false);
            for &c in linien(n) {
                if c & o != 0 || !c & x != 0 || fertige.contains(&c) {
                    continue;
                }
                alle &= c;
                irgend |= c;
                passend = true;
            }
            if !passend {
                return Err(Widerspruch);
            }
            for k in 0..n {
                let i = self.idx(l, k);
                if self.z[i] != LEER {
                    continue;
                }
                if alle >> k & 1 == 1 {
                    self.z[i] = X;
                    neu = true;
                } else if irgend >> k & 1 == 0 {
                    self.z[i] = O;
                    neu = true;
                }
            }
        }
        Ok(neu)
    }

    /// Setzt probeweise X oder O und folgert mit allen anderen Techniken weiter.
    /// Führt das zu einem Widerspruch, muss das andere Zeichen hin.
    fn annahme(&mut self) -> Result<bool, Widerspruch> {
        for i in 0..self.z.len() {
            if self.z[i] != LEER {
                continue;
            }
            for v in [X, O] {
                let mut probe = self.clone();
                probe.z[i] = v;
                if probe.folgere(Technik::Einzigartig).is_err() {
                    self.z[i] = gegen(v);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Wendet die Techniken bis `max` an, bis nichts mehr geht.
    fn folgere(&mut self, max: Technik) -> Result<(), Widerspruch> {
        'aussen: loop {
            for t in Technik::ALLE.into_iter().filter(|&t| t <= max) {
                if self.anwenden(t)? {
                    continue 'aussen;
                }
            }
            return Ok(());
        }
    }

    /// Löst wie ein Mensch – immer mit der einfachsten Technik, die weiterhilft.
    /// Liefert die schwerste nötige Technik, oder `None`, wenn die Techniken bis `max` nicht reichen.
    fn loese(&mut self, max: Technik) -> Option<Technik> {
        let mut schwerste = Technik::Nachbarn;
        loop {
            if self.voll() {
                // Endkontrolle mit allen Regeln
                return self.linien_durchdenken(true).ok().map(|_| schwerste);
            }
            let mut weiter = false;
            for t in Technik::ALLE.into_iter().filter(|&t| t <= max) {
                match self.anwenden(t) {
                    Err(_) => return None,
                    Ok(true) => {
                        schwerste = schwerste.max(t);
                        weiter = true;
                        break;
                    }
                    Ok(false) => {}
                }
            }
            if !weiter {
                return None;
            }
        }
    }
}

/// Zählt die Lösungen (höchstens `limit`) – für die Tests.
pub fn anzahl_loesungen(aufgabe: &[u8], n: usize, limit: u32) -> u32 {
    fn suche(f: Feld, limit: u32) -> u32 {
        let mut f = f;
        if f.folgere(Technik::Einzigartig).is_err() {
            return 0;
        }
        let Some(i) = f.z.iter().position(|&v| v == LEER) else { return 1 };
        let mut summe = 0;
        for v in [X, O] {
            let mut g = f.clone();
            g.z[i] = v;
            summe += suche(g, limit - summe);
            if summe >= limit {
                break;
            }
        }
        summe
    }
    suche(Feld { n, z: aufgabe.to_vec() }, limit)
}

/// Bewertet eine Aufgabe: Stufe und schwerste nötige Technik (`None`, wenn auch Annahmen nicht reichen).
pub fn bewerte(aufgabe: &[u8], n: usize) -> Option<(Stufe, &'static str)> {
    Feld { n, z: aufgabe.to_vec() }
        .loese(Technik::Annahme)
        .map(|t| (t.stufe(), t.name()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn pruefe_loesung(b: &Binoxo) {
        let n = b.n;
        let f = Feld { n, z: b.loesung.clone() };
        let mut zeilen = Vec::new();
        let mut spalten = Vec::new();
        for l in 0..2 * n {
            let (x, o) = f.masken(l);
            assert_eq!(x | o, ((1u32 << n) - 1) as u16, "Lösung ist voll");
            assert!(linie_gueltig(x, n), "Linie {l} gültig");
            if l < n { zeilen.push(x) } else { spalten.push(x) }
        }
        for v in [&zeilen, &spalten] {
            for a in 0..n {
                for c in a + 1..n {
                    assert_ne!(v[a], v[c], "keine doppelten Linien");
                }
            }
        }
        for i in 0..n * n {
            assert!(b.aufgabe[i] == LEER || b.aufgabe[i] == b.loesung[i]);
        }
    }

    #[test]
    fn erzeugte_binoxos_sind_eindeutig_und_richtig_bewertet() {
        let mut rng = StdRng::seed_from_u64(7);
        for n in [10, 12] {
            for stufe in Stufe::ALLE {
                let b = erzeuge_groesse(n, stufe, &mut rng);
                pruefe_loesung(&b);
                assert_eq!(anzahl_loesungen(&b.aufgabe, n, 2), 1, "{n}×{n} {stufe:?} eindeutig");
                assert_eq!(bewerte(&b.aufgabe, n).map(|(s, _)| s), Some(stufe));
            }
        }
    }

    #[test]
    fn linien_tabelle() {
        for n in [6, 8, 10, 12] {
            assert!(!linien(n).is_empty());
            assert!(linien(n).iter().all(|&m| linie_gueltig(m, n)));
        }
        // 6er-Linien von Hand gezählt: 14
        assert_eq!(linien(6).len(), 14);
    }
}
