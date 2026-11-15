//! Sudoku: Erzeugung, Eindeutigkeitsprüfung und Schwierigkeitsbewertung.
//!
//! Vorgehen beim Erzeugen:
//! 1. Ein volles, gültiges Gitter zufällig füllen (Backtracking).
//! 2. Ziffern paarweise punktsymmetrisch entfernen, solange die Lösung eindeutig bleibt.
//! 3. Das Ergebnis mit einem „menschlichen“ Löser bewerten: Die schwierigste
//!    Lösungstechnik, die man braucht, bestimmt die Stufe.
//! 4. Passt die Stufe nicht, von vorn.

use super::Stufe;
use rand::seq::SliceRandom;
use rand::Rng;
use std::sync::OnceLock;

/// 81 Felder zeilenweise, 0 = leer.
pub type Gitter = [u8; 81];

#[derive(Clone, Debug)]
pub struct Sudoku {
    pub aufgabe: Gitter,
    pub loesung: Gitter,
    pub stufe: Stufe,
    /// Schwierigste Technik, die zum Lösen nötig ist (für Neugierige).
    pub schwerste_technik: &'static str,
}

/// Erzeugt ein Sudoku genau der gewünschten Stufe.
pub fn erzeuge<R: Rng>(stufe: Stufe, rng: &mut R) -> Sudoku {
    loop {
        let loesung = volles_gitter(rng);
        let mut aufgabe = ausduennen(&loesung, rng);
        let (bewertet, technik) = bewerte(&aufgabe);
        if bewertet != stufe {
            continue;
        }
        if stufe == Stufe::Leicht {
            // Leichte Rätsel bekommen ein paar Vorgaben mehr – das macht sie angenehmer.
            auffuellen(&mut aufgabe, &loesung, 30, rng);
        }
        return Sudoku {
            aufgabe,
            loesung,
            stufe,
            schwerste_technik: technik,
        };
    }
}

// ---------------------------------------------------------------------------
// Tabellen: Einheiten (Zeilen, Spalten, Blöcke) und Nachbarn jeder Zelle
// ---------------------------------------------------------------------------

struct Tabellen {
    /// 0..9 Zeilen, 9..18 Spalten, 18..27 Blöcke
    einheiten: [[usize; 9]; 27],
    /// Die 20 Zellen, die eine Zelle „sieht“
    nachbarn: [[usize; 20]; 81],
}

fn tabellen() -> &'static Tabellen {
    static T: OnceLock<Tabellen> = OnceLock::new();
    T.get_or_init(|| {
        let mut einheiten = [[0usize; 9]; 27];
        for r in 0..9 {
            for c in 0..9 {
                einheiten[r][c] = r * 9 + c;
                einheiten[9 + c][r] = r * 9 + c;
            }
        }
        for b in 0..9 {
            let (br, bc) = (b / 3 * 3, b % 3 * 3);
            for k in 0..9 {
                einheiten[18 + b][k] = (br + k / 3) * 9 + bc + k % 3;
            }
        }
        let mut nachbarn = [[0usize; 20]; 81];
        for i in 0..81 {
            let mut n = 0;
            for j in 0..81 {
                if sieht(i, j) {
                    nachbarn[i][n] = j;
                    n += 1;
                }
            }
            debug_assert_eq!(n, 20);
        }
        Tabellen { einheiten, nachbarn }
    })
}

#[inline]
fn block(i: usize) -> usize {
    (i / 27) * 3 + (i % 9) / 3
}

/// Liegen zwei verschiedene Zellen in derselben Zeile, Spalte oder demselben Block?
#[inline]
fn sieht(a: usize, b: usize) -> bool {
    a != b && (a / 9 == b / 9 || a % 9 == b % 9 || block(a) == block(b))
}

const ALLE_ZIFFERN: u16 = 0x3FE; // Bits 1..=9

// ---------------------------------------------------------------------------
// Schneller Backtracking-Löser (zum Füllen und für die Eindeutigkeitsprüfung)
// ---------------------------------------------------------------------------

struct Backtracker {
    z: Gitter,
    zeile: [u16; 9],
    spalte: [u16; 9],
    blk: [u16; 9],
}

impl Backtracker {
    fn neu(g: &Gitter) -> Option<Self> {
        let mut b = Backtracker {
            z: [0; 81],
            zeile: [0; 9],
            spalte: [0; 9],
            blk: [0; 9],
        };
        for i in 0..81 {
            let d = g[i];
            if d != 0 {
                if b.frei(i) & (1 << d) == 0 {
                    return None; // widersprüchliche Vorgaben
                }
                b.setze(i, d);
            }
        }
        Some(b)
    }

    #[inline]
    fn frei(&self, i: usize) -> u16 {
        !(self.zeile[i / 9] | self.spalte[i % 9] | self.blk[block(i)]) & ALLE_ZIFFERN
    }

    #[inline]
    fn setze(&mut self, i: usize, d: u8) {
        let m = 1u16 << d;
        self.z[i] = d;
        self.zeile[i / 9] |= m;
        self.spalte[i % 9] |= m;
        self.blk[block(i)] |= m;
    }

    #[inline]
    fn loesche(&mut self, i: usize, d: u8) {
        let m = !(1u16 << d);
        self.z[i] = 0;
        self.zeile[i / 9] &= m;
        self.spalte[i % 9] &= m;
        self.blk[block(i)] &= m;
    }

    /// Leere Zelle mit den wenigsten Möglichkeiten; `None`, wenn alles gefüllt ist.
    fn beste_zelle(&self) -> Option<(usize, u16)> {
        let mut beste: Option<(usize, u16)> = None;
        let mut beste_n = 10;
        for i in 0..81 {
            if self.z[i] == 0 {
                let m = self.frei(i);
                let n = m.count_ones();
                if n < beste_n {
                    beste = Some((i, m));
                    beste_n = n;
                    if n <= 1 {
                        break;
                    }
                }
            }
        }
        beste
    }

    fn zaehle(&mut self, limit: u32, anzahl: &mut u32, erste: &mut Option<Gitter>) {
        let Some((i, mut m)) = self.beste_zelle() else {
            *anzahl += 1;
            if erste.is_none() {
                *erste = Some(self.z);
            }
            return;
        };
        while m != 0 {
            let d = m.trailing_zeros() as u8;
            m &= m - 1;
            self.setze(i, d);
            self.zaehle(limit, anzahl, erste);
            self.loesche(i, d);
            if *anzahl >= limit {
                return;
            }
        }
    }

    fn fuelle_zufaellig<R: Rng>(&mut self, rng: &mut R) -> bool {
        let Some((i, m)) = self.beste_zelle() else {
            return true;
        };
        let mut ziffern: Vec<u8> = (1..=9).filter(|d| m & (1 << d) != 0).collect();
        ziffern.shuffle(rng);
        for d in ziffern {
            self.setze(i, d);
            if self.fuelle_zufaellig(rng) {
                return true;
            }
            self.loesche(i, d);
        }
        false
    }
}

/// Zählt Lösungen bis höchstens `limit` und liefert die erste gefundene.
pub fn anzahl_loesungen(g: &Gitter, limit: u32) -> (u32, Option<Gitter>) {
    let Some(mut b) = Backtracker::neu(g) else {
        return (0, None);
    };
    let mut anzahl = 0;
    let mut erste = None;
    b.zaehle(limit, &mut anzahl, &mut erste);
    (anzahl, erste)
}

fn volles_gitter<R: Rng>(rng: &mut R) -> Gitter {
    let mut b = Backtracker::neu(&[0; 81]).expect("leeres Gitter ist gültig");
    let ok = b.fuelle_zufaellig(rng);
    debug_assert!(ok);
    b.z
}

/// Entfernt Ziffernpaare (punktsymmetrisch), solange die Lösung eindeutig bleibt.
fn ausduennen<R: Rng>(voll: &Gitter, rng: &mut R) -> Gitter {
    let mut g = *voll;
    let mut reihenfolge: Vec<usize> = (0..=40).collect();
    reihenfolge.shuffle(rng);
    for i in reihenfolge {
        let j = 80 - i;
        let (a, b) = (g[i], g[j]);
        g[i] = 0;
        g[j] = 0;
        if anzahl_loesungen(&g, 2).0 != 1 {
            g[i] = a;
            g[j] = b;
        }
    }
    g
}

/// Ergänzt symmetrische Vorgaben aus der Lösung, bis `ziel` erreicht ist.
fn auffuellen<R: Rng>(g: &mut Gitter, loesung: &Gitter, ziel: usize, rng: &mut R) {
    let mut leer: Vec<usize> = (0..=40).filter(|&i| g[i] == 0).collect();
    leer.shuffle(rng);
    for i in leer {
        if g.iter().filter(|&&d| d != 0).count() >= ziel {
            break;
        }
        g[i] = loesung[i];
        g[80 - i] = loesung[80 - i];
    }
}

// ---------------------------------------------------------------------------
// „Menschlicher“ Löser zur Bewertung
// ---------------------------------------------------------------------------

/// Lösungstechniken, sortiert nach Schwierigkeit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Technik {
    VersteckterSingle,
    NackterSingle,
    BlockLinie,
    NacktesPaar,
    VerstecktesPaar,
    NacktesTripel,
    VerstecktesTripel,
    XWing,
    XYWing,
    Swordfish,
    XYZWing,
    NacktesQuartett,
    VerstecktesQuartett,
    Jellyfish,
}

impl Technik {
    fn stufe(self) -> Stufe {
        use Technik::*;
        match self {
            VersteckterSingle | NackterSingle => Stufe::Leicht,
            BlockLinie | NacktesPaar | VerstecktesPaar => Stufe::Mittel,
            NacktesTripel | VerstecktesTripel | XWing => Stufe::Schwer,
            XYWing | Swordfish | XYZWing | NacktesQuartett | VerstecktesQuartett | Jellyfish => {
                Stufe::SehrSchwer
            }
        }
    }

    fn name(self) -> &'static str {
        use Technik::*;
        match self {
            VersteckterSingle => "Versteckter Single",
            NackterSingle => "Nackter Single",
            BlockLinie => "Block-Linien-Wechselwirkung",
            NacktesPaar => "Nacktes Paar",
            VerstecktesPaar => "Verstecktes Paar",
            NacktesTripel => "Nacktes Tripel",
            VerstecktesTripel => "Verstecktes Tripel",
            XWing => "X-Wing",
            XYWing => "XY-Wing",
            Swordfish => "Swordfish",
            XYZWing => "XYZ-Wing",
            NacktesQuartett => "Nacktes Quartett",
            VerstecktesQuartett => "Verstecktes Quartett",
            Jellyfish => "Jellyfish",
        }
    }
}

/// Bewertet ein (eindeutiges) Sudoku. Kommt der Löser mit seinen Techniken nicht
/// weiter, braucht man Ketten oder Annahmen → „teuflisch“.
pub fn bewerte(g: &Gitter) -> (Stufe, &'static str) {
    let mut m = Mensch::neu(g);
    let mut schwerste = Technik::VersteckterSingle;
    while !m.geloest() {
        let Some(t) = m.naechster_schritt() else {
            return (Stufe::Teuflisch, "Ketten oder Annahmen");
        };
        schwerste = schwerste.max(t);
    }
    (schwerste.stufe(), schwerste.name())
}

struct Mensch {
    wert: Gitter,
    /// Kandidaten als Bitmaske (Bits 1..=9), 0 bei gefüllten Zellen
    kand: [u16; 81],
}

impl Mensch {
    fn neu(g: &Gitter) -> Self {
        let mut m = Mensch {
            wert: [0; 81],
            kand: [ALLE_ZIFFERN; 81],
        };
        for i in 0..81 {
            if g[i] != 0 {
                m.setze(i, g[i]);
            }
        }
        m
    }

    fn geloest(&self) -> bool {
        self.wert.iter().all(|&d| d != 0)
    }

    fn setze(&mut self, i: usize, d: u8) {
        self.wert[i] = d;
        self.kand[i] = 0;
        let bit = 1u16 << d;
        for &p in &tabellen().nachbarn[i] {
            self.kand[p] &= !bit;
        }
    }

    /// Entfernt Kandidaten; `true`, wenn sich etwas geändert hat.
    #[inline]
    fn entferne(&mut self, i: usize, maske: u16) -> bool {
        if self.kand[i] & maske != 0 {
            self.kand[i] &= !maske;
            true
        } else {
            false
        }
    }

    /// Wendet die leichteste Technik an, die weiterhilft.
    fn naechster_schritt(&mut self) -> Option<Technik> {
        use Technik::*;
        if self.versteckter_single() {
            return Some(VersteckterSingle);
        }
        if self.nackter_single() {
            return Some(NackterSingle);
        }
        if self.block_linie() {
            return Some(BlockLinie);
        }
        if self.nackte_gruppe(2) {
            return Some(NacktesPaar);
        }
        if self.versteckte_gruppe(2) {
            return Some(VerstecktesPaar);
        }
        if self.nackte_gruppe(3) {
            return Some(NacktesTripel);
        }
        if self.versteckte_gruppe(3) {
            return Some(VerstecktesTripel);
        }
        if self.fisch(2) {
            return Some(XWing);
        }
        if self.xy_wing() {
            return Some(XYWing);
        }
        if self.fisch(3) {
            return Some(Swordfish);
        }
        if self.xyz_wing() {
            return Some(XYZWing);
        }
        if self.nackte_gruppe(4) {
            return Some(NacktesQuartett);
        }
        if self.versteckte_gruppe(4) {
            return Some(VerstecktesQuartett);
        }
        if self.fisch(4) {
            return Some(Jellyfish);
        }
        None
    }

    fn nackter_single(&mut self) -> bool {
        for i in 0..81 {
            if self.wert[i] == 0 && self.kand[i].count_ones() == 1 {
                let d = self.kand[i].trailing_zeros() as u8;
                self.setze(i, d);
                return true;
            }
        }
        false
    }

    fn versteckter_single(&mut self) -> bool {
        let t = tabellen();
        for e in 0..27 {
            for d in 1..=9u8 {
                let bit = 1u16 << d;
                let mut n = 0;
                let mut pos = 0;
                for &i in &t.einheiten[e] {
                    if self.kand[i] & bit != 0 {
                        n += 1;
                        pos = i;
                    }
                }
                if n == 1 {
                    self.setze(pos, d);
                    return true;
                }
            }
        }
        false
    }

    /// Pointing (Block → Linie) und Claiming (Linie → Block).
    fn block_linie(&mut self) -> bool {
        let t = tabellen();
        let mut fortschritt = false;
        for d in 1..=9u8 {
            let bit = 1u16 << d;
            // Block → Zeile/Spalte
            for b in 0..9 {
                let zellen: Vec<usize> = t.einheiten[18 + b]
                    .iter()
                    .copied()
                    .filter(|&i| self.kand[i] & bit != 0)
                    .collect();
                if zellen.len() < 2 {
                    continue;
                }
                let r = zellen[0] / 9;
                if zellen.iter().all(|&i| i / 9 == r) {
                    for &i in &t.einheiten[r] {
                        if block(i) != b && self.entferne(i, bit) {
                            fortschritt = true;
                        }
                    }
                }
                let c = zellen[0] % 9;
                if zellen.iter().all(|&i| i % 9 == c) {
                    for &i in &t.einheiten[9 + c] {
                        if block(i) != b && self.entferne(i, bit) {
                            fortschritt = true;
                        }
                    }
                }
            }
            // Zeile/Spalte → Block
            for e in 0..18 {
                let zellen: Vec<usize> = t.einheiten[e]
                    .iter()
                    .copied()
                    .filter(|&i| self.kand[i] & bit != 0)
                    .collect();
                if zellen.len() < 2 {
                    continue;
                }
                let bk = block(zellen[0]);
                if zellen.iter().all(|&i| block(i) == bk) {
                    for &i in &t.einheiten[18 + bk] {
                        if !t.einheiten[e].contains(&i) && self.entferne(i, bit) {
                            fortschritt = true;
                        }
                    }
                }
            }
        }
        fortschritt
    }

    /// Nacktes Paar/Tripel/Quartett: k Zellen einer Einheit mit zusammen genau k Kandidaten.
    fn nackte_gruppe(&mut self, k: usize) -> bool {
        let t = tabellen();
        let mut fortschritt = false;
        for e in 0..27 {
            let zellen: Vec<usize> = t.einheiten[e]
                .iter()
                .copied()
                .filter(|&i| {
                    let n = self.kand[i].count_ones() as usize;
                    self.wert[i] == 0 && n >= 2 && n <= k
                })
                .collect();
            if zellen.len() < k {
                continue;
            }
            for kombi in kombinationen(&zellen, k) {
                let vereinigung = kombi.iter().fold(0u16, |a, &i| a | self.kand[i]);
                if vereinigung.count_ones() as usize == k {
                    for &i in &t.einheiten[e] {
                        if !kombi.contains(&i) && self.entferne(i, vereinigung) {
                            fortschritt = true;
                        }
                    }
                }
            }
        }
        fortschritt
    }

    /// Verstecktes Paar/Tripel/Quartett: k Ziffern, die in einer Einheit nur in k Zellen vorkommen.
    fn versteckte_gruppe(&mut self, k: usize) -> bool {
        let t = tabellen();
        let mut fortschritt = false;
        for e in 0..27 {
            let zellen = t.einheiten[e];
            let mut pos = [0u16; 10];
            for (p, &i) in zellen.iter().enumerate() {
                for d in 1..=9 {
                    if self.kand[i] & (1 << d) != 0 {
                        pos[d] |= 1 << p;
                    }
                }
            }
            let ziffern: Vec<usize> = (1..=9)
                .filter(|&d| {
                    let n = pos[d].count_ones() as usize;
                    n >= 2 && n <= k
                })
                .collect();
            if ziffern.len() < k {
                continue;
            }
            for kombi in kombinationen(&ziffern, k) {
                let vereinigung = kombi.iter().fold(0u16, |a, &d| a | pos[d]);
                if vereinigung.count_ones() as usize == k {
                    let behalten = kombi.iter().fold(0u16, |a, &d| a | (1 << d));
                    for p in 0..9 {
                        if vereinigung & (1 << p) != 0
                            && self.entferne(zellen[p], !behalten & ALLE_ZIFFERN)
                        {
                            fortschritt = true;
                        }
                    }
                }
            }
        }
        fortschritt
    }

    /// X-Wing (k=2), Swordfish (k=3), Jellyfish (k=4) – über Zeilen und über Spalten.
    fn fisch(&mut self, k: usize) -> bool {
        let mut fortschritt = false;
        for d in 1..=9u8 {
            let bit = 1u16 << d;
            for basis_zeilen in [true, false] {
                let zelle = |l: usize, q: usize| if basis_zeilen { l * 9 + q } else { q * 9 + l };
                let mut masken = [0u16; 9];
                for l in 0..9 {
                    for q in 0..9 {
                        if self.kand[zelle(l, q)] & bit != 0 {
                            masken[l] |= 1 << q;
                        }
                    }
                }
                let linien: Vec<usize> = (0..9)
                    .filter(|&l| {
                        let n = masken[l].count_ones() as usize;
                        n >= 2 && n <= k
                    })
                    .collect();
                if linien.len() < k {
                    continue;
                }
                for kombi in kombinationen(&linien, k) {
                    let quer = kombi.iter().fold(0u16, |a, &l| a | masken[l]);
                    if quer.count_ones() as usize != k {
                        continue;
                    }
                    for q in 0..9 {
                        if quer & (1 << q) == 0 {
                            continue;
                        }
                        for l in 0..9 {
                            if !kombi.contains(&l) && self.entferne(zelle(l, q), bit) {
                                fortschritt = true;
                            }
                        }
                    }
                }
            }
        }
        fortschritt
    }

    /// Angelpunkt {x,y} mit Zangen {x,z} und {y,z}: z fällt überall weg, wo beide Zangen hinsehen.
    fn xy_wing(&mut self) -> bool {
        let t = tabellen();
        for p in 0..81 {
            let kp = self.kand[p];
            if kp.count_ones() != 2 {
                continue;
            }
            for &a in &t.nachbarn[p] {
                let ka = self.kand[a];
                if ka.count_ones() != 2 || ka == kp || (ka & kp).count_ones() != 1 {
                    continue;
                }
                for &b in &t.nachbarn[p] {
                    let kb = self.kand[b];
                    if b == a || kb.count_ones() != 2 || kb == kp || kb == ka {
                        continue;
                    }
                    let z = ka & kb;
                    if z.count_ones() != 1 || z & kp != 0 || (ka | kb | kp).count_ones() != 3 {
                        continue;
                    }
                    let mut fortschritt = false;
                    for &c in &t.nachbarn[a] {
                        if c != b && sieht(c, b) && self.entferne(c, z) {
                            fortschritt = true;
                        }
                    }
                    if fortschritt {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Angelpunkt {x,y,z} mit Zangen {x,z} und {y,z}: z fällt dort weg, wo alle drei hinsehen.
    fn xyz_wing(&mut self) -> bool {
        let t = tabellen();
        for p in 0..81 {
            let kp = self.kand[p];
            if kp.count_ones() != 3 {
                continue;
            }
            for &a in &t.nachbarn[p] {
                let ka = self.kand[a];
                if ka.count_ones() != 2 || ka & !kp != 0 {
                    continue;
                }
                for &b in &t.nachbarn[p] {
                    let kb = self.kand[b];
                    if b <= a || kb.count_ones() != 2 || kb & !kp != 0 || kb == ka {
                        continue;
                    }
                    let z = ka & kb;
                    let mut fortschritt = false;
                    for &c in &t.nachbarn[p] {
                        if c != a && c != b && sieht(c, a) && sieht(c, b) && self.entferne(c, z) {
                            fortschritt = true;
                        }
                    }
                    if fortschritt {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Alle k-elementigen Teilmengen (in Reihenfolge).
fn kombinationen<T: Copy>(liste: &[T], k: usize) -> Vec<Vec<T>> {
    fn rek<T: Copy>(liste: &[T], k: usize, start: usize, akt: &mut Vec<T>, aus: &mut Vec<Vec<T>>) {
        if akt.len() == k {
            aus.push(akt.clone());
            return;
        }
        for i in start..liste.len() {
            if liste.len() - i < k - akt.len() {
                break;
            }
            akt.push(liste[i]);
            rek(liste, k, i + 1, akt, aus);
            akt.pop();
        }
    }
    let mut aus = Vec::new();
    rek(liste, k, 0, &mut Vec::with_capacity(k), &mut aus);
    aus
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn parse(s: &str) -> Gitter {
        let mut g = [0u8; 81];
        for (i, c) in s.chars().filter(|c| !c.is_whitespace()).enumerate() {
            g[i] = c.to_digit(10).unwrap_or(0) as u8;
        }
        g
    }

    fn ist_gueltige_loesung(g: &Gitter) -> bool {
        let t = tabellen();
        t.einheiten.iter().all(|e| {
            let m = e.iter().fold(0u16, |a, &i| a | (1 << g[i]));
            m == ALLE_ZIFFERN
        })
    }

    #[test]
    fn erzeugte_sudokus_sind_eindeutig_und_passen() {
        let mut rng = StdRng::seed_from_u64(42);
        for stufe in [Stufe::Leicht, Stufe::Mittel, Stufe::Schwer] {
            let s = erzeuge(stufe, &mut rng);
            assert!(ist_gueltige_loesung(&s.loesung));
            let (n, l) = anzahl_loesungen(&s.aufgabe, 2);
            assert_eq!(n, 1);
            assert_eq!(l.unwrap(), s.loesung);
            assert_eq!(bewerte(&s.aufgabe).0, stufe);
        }
    }

    #[test]
    fn bekanntes_schweres_raetsel_wird_nicht_als_leicht_bewertet() {
        // „Platinum Blonde“ – berüchtigt schwer
        let g = parse(
            "000000012000000003002300400001800005060070800000009000008500000900040500470006000",
        );
        assert_eq!(anzahl_loesungen(&g, 2).0, 1);
        assert_eq!(bewerte(&g).0, Stufe::Teuflisch);
    }

    #[test]
    fn menschlicher_loeser_bleibt_korrekt() {
        // Der menschliche Löser darf nie etwas Falsches eintragen.
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..30 {
            let voll = volles_gitter(&mut rng);
            let aufgabe = ausduennen(&voll, &mut rng);
            let mut m = Mensch::neu(&aufgabe);
            while !m.geloest() {
                if m.naechster_schritt().is_none() {
                    break;
                }
                for i in 0..81 {
                    if m.wert[i] != 0 {
                        assert_eq!(m.wert[i], voll[i]);
                    } else {
                        assert!(m.kand[i] & (1 << voll[i]) != 0, "richtiger Kandidat gelöscht");
                    }
                }
            }
        }
    }
}
