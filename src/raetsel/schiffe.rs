//! Schiffe versenken (auch „Bimaru“): Die Flotte ist so im Gitter versteckt, dass
//! – sich keine zwei Schiffe berühren, auch nicht über Eck,
//! – die Zahlen am Rand angeben, wie viele Schiffsfelder in der Zeile/Spalte liegen,
//! – vorgegebene Schiffsteile (Einzelschiff, Bug/Heck, Mittelstück) und Wasserfelder passen.
//!
//! Erzeugt wird wie bei Sudoku und Binoxo: Flotte zufällig legen, alle Felder als
//! Hinweise vorgeben, dann Hinweise (und bei einer Variante auch Randzahlen) entfernen,
//! solange ein „menschlicher“ Löser mit den Techniken der Zielstufe durchkommt.

use super::Stufe;
use rand::seq::SliceRandom;
use rand::Rng;

/// Richtung, in die ein Schiff von einem Bug/Heck aus weitergeht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Richtung {
    Oben,
    Unten,
    Links,
    Rechts,
}

impl Richtung {
    pub const ALLE: [Richtung; 4] = [Richtung::Oben, Richtung::Unten, Richtung::Links, Richtung::Rechts];

    /// (Zeile, Spalte)-Schritt
    pub fn schritt(self) -> (isize, isize) {
        match self {
            Richtung::Oben => (-1, 0),
            Richtung::Unten => (1, 0),
            Richtung::Links => (0, -1),
            Richtung::Rechts => (0, 1),
        }
    }
}

/// Was ein Hinweisfeld zeigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Teil {
    Wasser,
    /// Ein-Feld-Schiff
    Einzel,
    /// Bug oder Heck; das Schiff geht in diese Richtung weiter
    Ende(Richtung),
    /// Mittelstück (waagrecht oder senkrecht)
    Mitte,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variante {
    /// 10×10, Flotte 1×4, 2×3, 3×2, 4×1, alle Randzahlen
    Klassisch,
    /// Wie klassisch, aber einige Randzahlen fehlen
    OhneRandzahlen,
    /// 12×12, Flotte 1×5, 1×4, 2×3, 3×2, 4×1
    Gross,
}

#[derive(Clone, Debug)]
pub struct Schiffe {
    pub n: usize,
    /// Schiffslängen, absteigend
    pub flotte: Vec<usize>,
    /// Randzahlen; `None` = nicht verraten
    pub zeilen: Vec<Option<u8>>,
    pub spalten: Vec<Option<u8>>,
    pub hinweise: Vec<(usize, Teil)>,
    /// true = Schiffsfeld
    pub loesung: Vec<bool>,
    pub variante: Variante,
    pub stufe: Stufe,
    pub schwerste_technik: &'static str,
}

/// Höchstens so viele Randzahlen fehlen bei der Variante „ohne Randzahlen“.
const MAX_FEHLENDE_ZAHLEN: usize = 5;

/// Erzeugt ein Schiffe-Rätsel der gewünschten Stufe in einer zufälligen Variante
/// (meist klassisch 10×10).
pub fn erzeuge<R: Rng>(stufe: Stufe, rng: &mut R) -> Schiffe {
    let variante = match rng.gen_range(0..4) {
        0 | 1 => Variante::Klassisch,
        2 => Variante::OhneRandzahlen,
        _ => Variante::Gross,
    };
    erzeuge_variante(variante, stufe, rng)
}

pub fn erzeuge_variante<R: Rng>(variante: Variante, stufe: Stufe, rng: &mut R) -> Schiffe {
    let (n, flotte) = match variante {
        Variante::Klassisch | Variante::OhneRandzahlen => (10, vec![4, 3, 3, 2, 2, 2, 1, 1, 1, 1]),
        Variante::Gross => (12, vec![5, 4, 3, 3, 2, 2, 2, 1, 1, 1, 1]),
    };
    let ziel = Technik::fuer(stufe);
    loop {
        let loesung = flotte_legen(n, &flotte, rng);
        let zahl = |zellen: &mut dyn Iterator<Item = usize>| Some(zellen.filter(|&i| loesung[i]).count() as u8);
        let mut a = Aufgabe {
            n,
            flotte: flotte.clone(),
            zeilen: (0..n).map(|r| zahl(&mut (0..n).map(|c| r * n + c))).collect(),
            spalten: (0..n).map(|c| zahl(&mut (0..n).map(|r| r * n + c))).collect(),
            hinweise: (0..n * n).map(|i| (i, teil_aus_loesung(&loesung, n, i))).collect(),
        };
        ausduennen(&mut a, ziel, variante == Variante::OhneRandzahlen, rng);
        let mut f = Feld::neu(&a);
        if f.loese(ziel) == Some(ziel) {
            debug_assert!(f.z.iter().zip(&loesung).all(|(&z, &l)| (z == SCH) == l));
            let mut hinweise = a.hinweise;
            hinweise.sort_by_key(|h| h.0);
            return Schiffe {
                n,
                flotte,
                zeilen: a.zeilen,
                spalten: a.spalten,
                hinweise,
                loesung,
                variante,
                stufe,
                schwerste_technik: ziel.name(),
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Flotte legen
// ---------------------------------------------------------------------------

fn flotte_legen<R: Rng>(n: usize, flotte: &[usize], rng: &mut R) -> Vec<bool> {
    'neu: loop {
        let mut z = vec![false; n * n];
        for &s in flotte {
            let mut moeglich = Vec::new();
            for (dr, dc) in [(0usize, 1usize), (1, 0)] {
                if s == 1 && dr == 1 {
                    continue;
                }
                for r0 in 0..n - dr * (s - 1) {
                    for c0 in 0..n - dc * (s - 1) {
                        let zellen: Vec<usize> = (0..s).map(|k| (r0 + dr * k) * n + c0 + dc * k).collect();
                        let frei = zellen.iter().all(|&i| {
                            let (r, c) = ((i / n) as isize, (i % n) as isize);
                            (-1..=1).all(|a| {
                                (-1..=1).all(|b| {
                                    let (rr, cc) = (r + a, c + b);
                                    rr < 0 || cc < 0 || rr >= n as isize || cc >= n as isize || !z[rr as usize * n + cc as usize]
                                })
                            })
                        });
                        if frei {
                            moeglich.push(zellen);
                        }
                    }
                }
            }
            let Some(wahl) = moeglich.choose(rng) else { continue 'neu };
            for &i in wahl {
                z[i] = true;
            }
        }
        return z;
    }
}

fn teil_aus_loesung(loesung: &[bool], n: usize, i: usize) -> Teil {
    if !loesung[i] {
        return Teil::Wasser;
    }
    let (r, c) = ((i / n) as isize, (i % n) as isize);
    let weiter: Vec<Richtung> = Richtung::ALLE
        .into_iter()
        .filter(|d| {
            let (dr, dc) = d.schritt();
            let (rr, cc) = (r + dr, c + dc);
            rr >= 0 && cc >= 0 && rr < n as isize && cc < n as isize && loesung[rr as usize * n + cc as usize]
        })
        .collect();
    match weiter.len() {
        0 => Teil::Einzel,
        1 => Teil::Ende(weiter[0]),
        _ => Teil::Mitte,
    }
}

// ---------------------------------------------------------------------------
// Ausdünnen
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Aufgabe {
    n: usize,
    flotte: Vec<usize>,
    zeilen: Vec<Option<u8>>,
    spalten: Vec<Option<u8>>,
    hinweise: Vec<(usize, Teil)>,
}

fn loesbar(a: &Aufgabe, t: Technik) -> bool {
    Feld::neu(a).loese(t).is_some()
}

fn ausduennen<R: Rng>(a: &mut Aufgabe, ziel: Technik, randzahlen_weg: bool, rng: &mut R) {
    // Für „teuflisch“ erst mit den schnellen Techniken, dann mit Annahmen weiter
    let phasen: &[Technik] = if ziel == Technik::Annahme {
        &[Technik::Platzierung, Technik::Annahme]
    } else {
        std::slice::from_ref(&ziel)
    };
    for &t in phasen {
        let mut reihenfolge = a.hinweise.clone();
        reihenfolge.shuffle(rng);
        for h in reihenfolge {
            let pos = a.hinweise.iter().position(|x| *x == h).unwrap();
            a.hinweise.remove(pos);
            if !loesbar(a, t) {
                a.hinweise.insert(pos, h);
            }
        }
    }
    if randzahlen_weg {
        let n = a.n;
        let mut linien: Vec<usize> = (0..2 * n).collect();
        linien.shuffle(rng);
        let mut weg = 0;
        for l in linien {
            if weg >= MAX_FEHLENDE_ZAHLEN {
                break;
            }
            let eintrag = if l < n { &mut a.zeilen[l] } else { &mut a.spalten[l - n] };
            let alt = eintrag.take();
            if loesbar(a, ziel) {
                weg += 1;
            } else if l < n {
                a.zeilen[l] = alt;
            } else {
                a.spalten[l - n] = alt;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Menschlicher Löser
// ---------------------------------------------------------------------------

/// Lösungstechniken, nach Schwierigkeit sortiert.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Technik {
    /// Wasser über Eck um jedes Schiffsfeld, Hinweisteile auswerten
    Nachbarn,
    /// Randzahl erreicht → Rest Wasser; Randzahl nur noch so knapp erreichbar → Rest Schiff
    Zaehlen,
    /// Fertige Schiffe abhaken, Schiffe abschließen, die nicht länger sein dürfen,
    /// Felder fluten, in die kein Schiff der Restflotte mehr passt
    Flotte,
    /// Alle möglichen Lagen eines Schiffstyps durchgehen
    Platzierung,
    /// Feld probeweise belegen, bis ein Widerspruch entsteht
    Annahme,
}

impl Technik {
    const ALLE: [Technik; 5] = [
        Technik::Nachbarn,
        Technik::Zaehlen,
        Technik::Flotte,
        Technik::Platzierung,
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
            Technik::Nachbarn => "Nachbarfelder",
            Technik::Zaehlen => "Randzahlen",
            Technik::Flotte => "Flotte abhaken",
            Technik::Platzierung => "Lagen eines Schiffs durchgehen",
            Technik::Annahme => "Annahme mit Widerspruch",
        }
    }
}

const UNB: u8 = 0;
const WAS: u8 = 1;
const SCH: u8 = 2;

struct Widerspruch;

/// Zusammenhängende Schiffsfelder.
struct Lauf {
    zellen: Vec<usize>,
    /// Ringsum Wasser oder Rand – das Schiff ist fertig
    komplett: bool,
}

/// Eine mögliche Lage eines Schiffs: seine Felder und das Wasser ringsum.
struct Lage {
    zellen: Vec<usize>,
    umgebung: Vec<usize>,
}

#[derive(Clone)]
struct Feld<'a> {
    a: &'a Aufgabe,
    n: usize,
    /// Randzahlen (0..n Zeilen, n..2n Spalten); eine einzelne fehlende Zahl
    /// pro Richtung lässt sich aus der Flottengröße erschließen.
    soll: Vec<Option<usize>>,
    /// Anzahl Schiffe je Länge (Index = Länge)
    flotte: Vec<usize>,
    z: Vec<u8>,
}

impl<'a> Feld<'a> {
    fn neu(a: &'a Aufgabe) -> Self {
        let gesamt: usize = a.flotte.iter().sum();
        let erschliessen = |v: &[Option<u8>]| -> Vec<Option<usize>> {
            let fehlend = v.iter().filter(|x| x.is_none()).count();
            let summe: usize = v.iter().flatten().map(|&x| x as usize).sum();
            v.iter()
                .map(|x| match x {
                    Some(k) => Some(*k as usize),
                    None if fehlend == 1 => Some(gesamt.saturating_sub(summe)),
                    None => None,
                })
                .collect()
        };
        let mut soll = erschliessen(&a.zeilen);
        soll.extend(erschliessen(&a.spalten));
        let mut flotte = vec![0; a.flotte.iter().max().copied().unwrap_or(0) + 1];
        for &s in &a.flotte {
            flotte[s] += 1;
        }
        Feld { a, n: a.n, soll, flotte, z: vec![UNB; a.n * a.n] }
    }

    fn nachbar(&self, i: usize, dr: isize, dc: isize) -> Option<usize> {
        let n = self.n as isize;
        let (r, c) = ((i / self.n) as isize + dr, (i % self.n) as isize + dc);
        (r >= 0 && c >= 0 && r < n && c < n).then(|| (r * n + c) as usize)
    }

    fn in_richtung(&self, i: usize, d: Richtung) -> Option<usize> {
        let (dr, dc) = d.schritt();
        self.nachbar(i, dr, dc)
    }

    fn zelle(&self, l: usize, k: usize) -> usize {
        if l < self.n {
            l * self.n + k
        } else {
            k * self.n + (l - self.n)
        }
    }

    fn setze(&mut self, i: usize, v: u8) -> Result<bool, Widerspruch> {
        match self.z[i] {
            x if x == v => Ok(false),
            UNB => {
                self.z[i] = v;
                Ok(true)
            }
            _ => Err(Widerspruch),
        }
    }

    fn anwenden(&mut self, t: Technik) -> Result<bool, Widerspruch> {
        match t {
            Technik::Nachbarn => self.nachbarn(),
            Technik::Zaehlen => self.zaehlen(),
            Technik::Flotte => self.flotte_abhaken(),
            Technik::Platzierung => self.platzierung(),
            Technik::Annahme => self.annahme(),
        }
    }

    fn nachbarn(&mut self) -> Result<bool, Widerspruch> {
        let mut neu = false;
        for i in 0..self.n * self.n {
            if self.z[i] != SCH {
                continue;
            }
            for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                if let Some(j) = self.nachbar(i, dr, dc) {
                    neu |= self.setze(j, WAS)?;
                }
            }
            let schiff = |d: Richtung| self.in_richtung(i, d).is_some_and(|j| self.z[j] == SCH);
            let waagrecht = schiff(Richtung::Links) || schiff(Richtung::Rechts);
            let senkrecht = schiff(Richtung::Oben) || schiff(Richtung::Unten);
            if waagrecht && senkrecht {
                return Err(Widerspruch);
            }
        }
        for k in 0..self.a.hinweise.len() {
            let (i, t) = self.a.hinweise[k];
            if t == Teil::Wasser {
                neu |= self.setze(i, WAS)?;
                continue;
            }
            neu |= self.setze(i, SCH)?;
            match t {
                Teil::Einzel => {
                    for d in Richtung::ALLE {
                        if let Some(j) = self.in_richtung(i, d) {
                            neu |= self.setze(j, WAS)?;
                        }
                    }
                }
                Teil::Ende(weiter) => {
                    for d in Richtung::ALLE {
                        match self.in_richtung(i, d) {
                            Some(j) => neu |= self.setze(j, if d == weiter { SCH } else { WAS })?,
                            None if d == weiter => return Err(Widerspruch),
                            None => {}
                        }
                    }
                }
                Teil::Mitte => {
                    use Richtung::*;
                    // Achse möglich, wenn beide Nachbarn auf ihr existieren und kein Wasser sind
                    // und quer dazu kein Schiff liegt
                    let moeglich = |achse: [Richtung; 2], quer: [Richtung; 2]| {
                        achse.iter().all(|&d| self.in_richtung(i, d).is_some_and(|j| self.z[j] != WAS))
                            && quer.iter().all(|&d| self.in_richtung(i, d).is_none_or(|j| self.z[j] != SCH))
                    };
                    let (achse, quer) = match (moeglich([Links, Rechts], [Oben, Unten]), moeglich([Oben, Unten], [Links, Rechts])) {
                        (false, false) => return Err(Widerspruch),
                        (true, false) => ([Links, Rechts], [Oben, Unten]),
                        (false, true) => ([Oben, Unten], [Links, Rechts]),
                        (true, true) => continue,
                    };
                    for d in achse {
                        let j = self.in_richtung(i, d).unwrap();
                        neu |= self.setze(j, SCH)?;
                    }
                    for d in quer {
                        if let Some(j) = self.in_richtung(i, d) {
                            neu |= self.setze(j, WAS)?;
                        }
                    }
                }
                Teil::Wasser => unreachable!(),
            }
        }
        Ok(neu)
    }

    fn zaehlen(&mut self) -> Result<bool, Widerspruch> {
        let n = self.n;
        let mut neu = false;
        for l in 0..2 * n {
            let Some(soll) = self.soll[l] else { continue };
            let zellen: Vec<usize> = (0..n).map(|k| self.zelle(l, k)).collect();
            let schiffe = zellen.iter().filter(|&&i| self.z[i] == SCH).count();
            let offen = zellen.iter().filter(|&&i| self.z[i] == UNB).count();
            if schiffe > soll || schiffe + offen < soll {
                return Err(Widerspruch);
            }
            if offen == 0 {
                continue;
            }
            let v = if schiffe == soll {
                WAS
            } else if schiffe + offen == soll {
                SCH
            } else {
                continue;
            };
            for i in zellen {
                if self.z[i] == UNB {
                    self.z[i] = v;
                }
            }
            neu = true;
        }
        Ok(neu)
    }

    /// Zusammenhängende Schiffsfelder; Widerspruch bei Knicken.
    fn laeufe(&self) -> Result<Vec<Lauf>, Widerspruch> {
        let n = self.n;
        let mut gesehen = vec![false; n * n];
        let mut laeufe = Vec::new();
        for start in 0..n * n {
            if self.z[start] != SCH || gesehen[start] {
                continue;
            }
            let mut zellen = vec![start];
            gesehen[start] = true;
            let mut k = 0;
            while k < zellen.len() {
                let i = zellen[k];
                for d in Richtung::ALLE {
                    if let Some(j) = self.in_richtung(i, d) {
                        if self.z[j] == SCH && !gesehen[j] {
                            gesehen[j] = true;
                            zellen.push(j);
                        }
                    }
                }
                k += 1;
            }
            let gerade = zellen.iter().all(|&i| i / n == start / n) || zellen.iter().all(|&i| i % n == start % n);
            if !gerade {
                return Err(Widerspruch);
            }
            let komplett = zellen.iter().all(|&i| {
                Richtung::ALLE.iter().all(|&d| {
                    self.in_richtung(i, d).is_none_or(|j| zellen.contains(&j) || self.z[j] == WAS)
                })
            });
            laeufe.push(Lauf { zellen, komplett });
        }
        Ok(laeufe)
    }

    /// Restflotte (Index = Länge) nach Abzug der fertigen Schiffe.
    fn restflotte(&self, laeufe: &[Lauf]) -> Result<Vec<usize>, Widerspruch> {
        let mut rest = self.flotte.clone();
        for l in laeufe {
            let s = l.zellen.len();
            if s >= rest.len() {
                return Err(Widerspruch);
            }
            if l.komplett {
                if rest[s] == 0 {
                    return Err(Widerspruch);
                }
                rest[s] -= 1;
            }
        }
        Ok(rest)
    }

    fn schiffzahlen(&self) -> Vec<usize> {
        (0..2 * self.n)
            .map(|l| (0..self.n).filter(|&k| self.z[self.zelle(l, k)] == SCH).count())
            .collect()
    }

    /// Alle Lagen, in denen ein Schiff der Länge `s` noch liegen könnte – ohne die fertigen.
    fn lagen(&self, s: usize, schiffzahlen: &[usize]) -> Vec<Lage> {
        let n = self.n;
        let mut ergebnis = Vec::new();
        for (dr, dc) in [(0usize, 1usize), (1, 0)] {
            if s == 1 && dr == 1 {
                continue;
            }
            for r0 in 0..n - dr * (s - 1) {
                for c0 in 0..n - dc * (s - 1) {
                    let zellen: Vec<usize> = (0..s).map(|k| (r0 + dr * k) * n + c0 + dc * k).collect();
                    if zellen.iter().any(|&i| self.z[i] == WAS) {
                        continue;
                    }
                    let (r1, c1) = (r0 + dr * (s - 1), c0 + dc * (s - 1));
                    let mut umgebung = Vec::new();
                    for r in r0.saturating_sub(1)..=(r1 + 1).min(n - 1) {
                        for c in c0.saturating_sub(1)..=(c1 + 1).min(n - 1) {
                            if !(r >= r0 && r <= r1 && c >= c0 && c <= c1) {
                                umgebung.push(r * n + c);
                            }
                        }
                    }
                    if umgebung.iter().any(|&j| self.z[j] == SCH) {
                        continue;
                    }
                    let fertig = zellen.iter().all(|&i| self.z[i] == SCH) && umgebung.iter().all(|&j| self.z[j] == WAS);
                    if fertig {
                        continue;
                    }
                    // Passen die Randzahlen noch?
                    let mut dazu = vec![0usize; 2 * n];
                    for &i in &zellen {
                        if self.z[i] == UNB {
                            dazu[i / n] += 1;
                            dazu[n + i % n] += 1;
                        }
                    }
                    let passt = (0..2 * n).all(|l| dazu[l] == 0 || self.soll[l].is_none_or(|k| schiffzahlen[l] + dazu[l] <= k));
                    if passt {
                        ergebnis.push(Lage { zellen, umgebung });
                    }
                }
            }
        }
        ergebnis
    }

    fn flotte_abhaken(&mut self) -> Result<bool, Widerspruch> {
        let laeufe = self.laeufe()?;
        let rest = self.restflotte(&laeufe)?;
        let mut neu = false;
        // Schiffe abschließen, die nicht mehr länger werden dürfen
        for l in laeufe.iter().filter(|l| !l.komplett) {
            let s = l.zellen.len();
            if (s + 1..rest.len()).any(|t| rest[t] > 0) {
                continue;
            }
            if rest[s] == 0 {
                return Err(Widerspruch);
            }
            for &i in &l.zellen {
                for d in Richtung::ALLE {
                    if let Some(j) = self.in_richtung(i, d) {
                        if !l.zellen.contains(&j) {
                            neu |= self.setze(j, WAS)?;
                        }
                    }
                }
            }
        }
        if neu {
            return Ok(true);
        }
        // Felder, in die kein Schiff der Restflotte mehr passt, sind Wasser
        let zahlen = self.schiffzahlen();
        let mut bedeckt = vec![false; self.n * self.n];
        for s in 1..rest.len() {
            if rest[s] > 0 {
                for lage in self.lagen(s, &zahlen) {
                    for i in lage.zellen {
                        bedeckt[i] = true;
                    }
                }
            }
        }
        for i in 0..self.n * self.n {
            if self.z[i] == UNB && !bedeckt[i] {
                self.z[i] = WAS;
                neu = true;
            }
        }
        Ok(neu)
    }

    /// Für jeden Schiffstyp alle Lagen durchgehen: Was in allen Lagen Schiff (bzw. Wasser) ist,
    /// steht fest. Gibt es genau so viele Lagen wie fehlende Schiffe, liegen sie alle.
    fn platzierung(&mut self) -> Result<bool, Widerspruch> {
        let laeufe = self.laeufe()?;
        let rest = self.restflotte(&laeufe)?;
        let zahlen = self.schiffzahlen();
        let nn = self.n * self.n;
        for s in (1..rest.len()).rev() {
            let m = rest[s];
            if m == 0 {
                continue;
            }
            let lagen = self.lagen(s, &zahlen);
            if lagen.len() < m {
                return Err(Widerspruch);
            }
            let mut neu = false;
            if lagen.len() == m {
                for lage in &lagen {
                    for &i in &lage.zellen {
                        neu |= self.setze(i, SCH)?;
                    }
                    for &j in &lage.umgebung {
                        neu |= self.setze(j, WAS)?;
                    }
                }
            } else {
                let (mut schiff, mut wasser) = (vec![0usize; nn], vec![0usize; nn]);
                for lage in &lagen {
                    for &i in &lage.zellen {
                        schiff[i] += 1;
                    }
                    for &j in &lage.umgebung {
                        wasser[j] += 1;
                    }
                }
                for i in 0..nn {
                    if schiff[i] == lagen.len() {
                        neu |= self.setze(i, SCH)?;
                    } else if wasser[i] == lagen.len() {
                        neu |= self.setze(i, WAS)?;
                    }
                }
            }
            if neu {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn annahme(&mut self) -> Result<bool, Widerspruch> {
        for i in 0..self.z.len() {
            if self.z[i] != UNB {
                continue;
            }
            for (v, sonst) in [(SCH, WAS), (WAS, SCH)] {
                let mut probe = self.clone();
                probe.z[i] = v;
                if probe.folgere(Technik::Platzierung).is_err() {
                    self.z[i] = sonst;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

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

    /// Volles Gitter: erfüllt es alle Regeln?
    fn pruefe_voll(&mut self) -> bool {
        self.nachbarn().is_ok()
            && self.zaehlen().is_ok()
            && self
                .laeufe()
                .and_then(|l| self.restflotte(&l))
                .is_ok_and(|rest| rest.iter().all(|&r| r == 0))
    }

    /// Löst wie ein Mensch – immer mit der einfachsten Technik, die weiterhilft.
    fn loese(&mut self, max: Technik) -> Option<Technik> {
        let mut schwerste = Technik::Nachbarn;
        loop {
            if !self.z.contains(&UNB) {
                return self.pruefe_voll().then_some(schwerste);
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

fn aufgabe_von(s: &Schiffe) -> Aufgabe {
    Aufgabe {
        n: s.n,
        flotte: s.flotte.clone(),
        zeilen: s.zeilen.clone(),
        spalten: s.spalten.clone(),
        hinweise: s.hinweise.clone(),
    }
}

/// Zählt die Lösungen (höchstens `limit`) – für die Tests.
pub fn anzahl_loesungen(s: &Schiffe, limit: u32) -> u32 {
    fn suche(mut f: Feld, limit: u32) -> u32 {
        if f.folgere(Technik::Platzierung).is_err() {
            return 0;
        }
        let Some(i) = f.z.iter().position(|&v| v == UNB) else {
            return f.pruefe_voll() as u32;
        };
        let mut summe = 0;
        for v in [SCH, WAS] {
            let mut g = f.clone();
            g.z[i] = v;
            summe += suche(g, limit - summe);
            if summe >= limit {
                break;
            }
        }
        summe
    }
    let a = aufgabe_von(s);
    suche(Feld::neu(&a), limit)
}

/// Bewertet ein Rätsel: Stufe und schwerste nötige Technik (`None`, wenn auch Annahmen nicht reichen).
pub fn bewerte(s: &Schiffe) -> Option<(Stufe, &'static str)> {
    let a = aufgabe_von(s);
    Feld::neu(&a).loese(Technik::Annahme).map(|t| (t.stufe(), t.name()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn erzeugte_schiffe_sind_eindeutig_und_richtig_bewertet() {
        let mut rng = StdRng::seed_from_u64(11);
        for variante in [Variante::Klassisch, Variante::OhneRandzahlen, Variante::Gross] {
            for stufe in Stufe::IM_HEFT {
                let s = erzeuge_variante(variante, stufe, &mut rng);
                let n = s.n;
                // Lösung passt zur Flotte und zu den Randzahlen
                let felder = s.loesung.iter().filter(|&&b| b).count();
                assert_eq!(felder, s.flotte.iter().sum::<usize>());
                for r in 0..n {
                    if let Some(k) = s.zeilen[r] {
                        assert_eq!((0..n).filter(|&c| s.loesung[r * n + c]).count(), k as usize);
                    }
                }
                for &(i, t) in &s.hinweise {
                    assert_eq!(t, teil_aus_loesung(&s.loesung, n, i));
                }
                assert_eq!(anzahl_loesungen(&s, 2), 1, "{variante:?} {stufe:?} eindeutig");
                assert_eq!(bewerte(&s).map(|b| b.0), Some(stufe), "{variante:?} {stufe:?}");
                if variante == Variante::OhneRandzahlen {
                    let fehlend = s.zeilen.iter().chain(&s.spalten).filter(|z| z.is_none()).count();
                    assert!(fehlend <= MAX_FEHLENDE_ZAHLEN);
                }
            }
        }
    }
}
