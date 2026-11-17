//! Ein Rätselheft planen (welche Rätsel, welche Stufen) und parallel erzeugen.

use crate::pdf::Format;
use crate::raetsel::{binoxo, nonogramm, schiffe, sudoku, Raetsel, Stufe};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Rätselart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Art {
    Sudoku,
    Binoxo,
    Schiffe,
    Nonogramm,
}

impl Art {
    pub const ALLE: [Art; 4] = [Art::Sudoku, Art::Binoxo, Art::Schiffe, Art::Nonogramm];

    pub fn name(self) -> &'static str {
        match self {
            Art::Sudoku => "Sudoku",
            Art::Binoxo => "Binoxo",
            Art::Schiffe => "Schiffe versenken",
            Art::Nonogramm => "Nonogramme",
        }
    }
}

/// Wie die (ausgewählten) Schwierigkeitsstufen im Heft angeordnet werden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anordnung {
    /// Zufällig gemischt über die gewählten Stufen
    Gemischt,
    /// Gleichmäßig von leicht nach schwer über die gewählten Stufen
    Aufsteigend,
}

impl Anordnung {
    pub const ALLE: [Anordnung; 2] = [Anordnung::Gemischt, Anordnung::Aufsteigend];

    pub fn name(self) -> &'static str {
        match self {
            Anordnung::Gemischt => "Gemischt",
            Anordnung::Aufsteigend => "Aufsteigend",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Einstellungen {
    pub anzahl: usize,
    /// Gewünschte Stufen – beliebig kombinierbar (Mehrfachauswahl)
    pub stufen: Vec<Stufe>,
    pub anordnung: Anordnung,
    pub arten: Vec<Art>,
    pub format: Format,
}

impl Default for Einstellungen {
    fn default() -> Self {
        Einstellungen {
            anzahl: 8,
            stufen: Stufe::IM_HEFT.to_vec(),
            anordnung: Anordnung::Gemischt,
            arten: Art::ALLE.to_vec(),
            format: Format::A4,
        }
    }
}

/// Legt Art und Stufe jedes Rätsels fest. Gleiche Heftnummer → gleicher Plan.
pub fn plane(e: &Einstellungen, nummer: u32) -> Vec<(Art, Stufe)> {
    let n = e.anzahl.max(1);
    let mut rng = StdRng::seed_from_u64(nummer as u64);

    // Arten möglichst gleichmäßig, aber in zufälliger Reihenfolge
    let verfuegbar = if e.arten.is_empty() { Art::ALLE.to_vec() } else { e.arten.clone() };
    let mut arten = Vec::with_capacity(n);
    while arten.len() < n {
        let mut runde = verfuegbar.clone();
        runde.shuffle(&mut rng);
        arten.extend(runde);
    }
    arten.truncate(n);

    // Die kleinen Rätsel (vier pro Seite) auf volle Seiten aufrunden
    let klein_arten: Vec<Art> = verfuegbar.iter().copied().filter(|&a| a != Art::Nonogramm).collect();
    let anzahl_klein = arten.iter().filter(|&&a| a != Art::Nonogramm).count();
    if !klein_arten.is_empty() {
        let fehlend = (4 - anzahl_klein % 4) % 4;
        let mut zusatz = klein_arten.clone();
        zusatz.shuffle(&mut rng);
        arten.extend(zusatz.iter().cycle().take(fehlend));
    }
    let n = arten.len();

    // Gewählte Stufen (Mehrfachauswahl); leer = alle Heft-Stufen
    let mut gewaehlt: Vec<Stufe> = if e.stufen.is_empty() { Stufe::IM_HEFT.to_vec() } else { e.stufen.clone() };
    gewaehlt.sort();
    gewaehlt.dedup();
    let mut stufen: Vec<Stufe> = match e.anordnung {
        Anordnung::Aufsteigend => (0..n).map(|k| gewaehlt[k * gewaehlt.len() / n]).collect(),
        Anordnung::Gemischt => (0..n).map(|_| *gewaehlt.choose(&mut rng).unwrap()).collect(),
    };
    // Das Heft wird nach hinten schwerer
    stufen.sort();
    seiten_ordnen(arten.into_iter().zip(stufen).collect())
}

/// Ordnet so, dass die kleinen Rätsel (vier pro Seite) volle Seiten füllen und die
/// Nonogramme (zwei halbe Seiten pro Blatt) dazwischen liegen – grob nach Schwierigkeit.
fn seiten_ordnen(paare: Vec<(Art, Stufe)>) -> Vec<(Art, Stufe)> {
    let (gross, klein): (Vec<_>, Vec<_>) = paare.into_iter().partition(|p| p.0 == Art::Nonogramm);
    let mut bloecke: Vec<Vec<(Art, Stufe)>> = klein.chunks(4).map(|c| c.to_vec()).collect();
    // Nonogramme paarweise: zwei halbe Seiten füllen zusammen eine ganze
    bloecke.extend(gross.chunks(2).map(|c| c.to_vec()));
    // Stabil nach der leichtesten Stufe im Block
    bloecke.sort_by_key(|b| b[0].1);
    bloecke.concat()
}

fn saat(nummer: u32, k: usize) -> u64 {
    (nummer as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (k as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
}

fn erzeuge_einzeln(art: Art, stufe: Stufe, rng: &mut StdRng) -> Raetsel {
    match art {
        Art::Sudoku => Raetsel::Sudoku(sudoku::erzeuge(stufe, rng)),
        Art::Binoxo => Raetsel::Binoxo(binoxo::erzeuge(stufe, rng)),
        Art::Schiffe => Raetsel::Schiffe(schiffe::erzeuge(stufe, rng)),
        Art::Nonogramm => unreachable!("Nonogramme werden vorab gewählt"),
    }
}

/// Erzeugt alle Rätsel des Plans auf allen Prozessorkernen.
/// `fortschritt(fertig, gesamt)` wird nach jedem Rätsel aufgerufen.
pub fn erzeuge_raetsel(
    plan: &[(Art, Stufe)],
    nummer: u32,
    fortschritt: &(dyn Fn(usize, usize) + Sync),
) -> Vec<Raetsel> {
    let n = plan.len();
    let ergebnisse: Vec<Mutex<Option<Raetsel>>> = (0..n).map(|_| Mutex::new(None)).collect();

    // Nonogramme vorab aus der Sammlung wählen – geht schnell und vermeidet doppelte Motive
    let nono: Vec<usize> = (0..n).filter(|&k| plan[k].0 == Art::Nonogramm).collect();
    if !nono.is_empty() {
        let mut rng = StdRng::seed_from_u64(saat(nummer, usize::MAX));
        let stufen: Vec<Stufe> = nono.iter().map(|&k| plan[k].1).collect();
        for (&k, ng) in nono.iter().zip(nonogramm::waehle(&stufen, &mut rng)) {
            *ergebnisse[k].lock().unwrap() = Some(Raetsel::Nonogramm(ng));
        }
        fortschritt(nono.len(), n);
    }

    // Schwerste zuerst – die dauern am längsten
    let mut warteschlange: Vec<usize> = (0..n).filter(|&k| plan[k].0 != Art::Nonogramm).collect();
    warteschlange.sort_by_key(|&k| std::cmp::Reverse(plan[k].1));
    let m = warteschlange.len();
    let naechster = AtomicUsize::new(0);
    let fertig = AtomicUsize::new(nono.len());
    let kerne = std::thread::available_parallelism().map(|k| k.get()).unwrap_or(4);

    std::thread::scope(|sc| {
        for _ in 0..kerne.min(m) {
            sc.spawn(|| loop {
                let j = naechster.fetch_add(1, Ordering::SeqCst);
                if j >= m {
                    break;
                }
                let k = warteschlange[j];
                let mut rng = StdRng::seed_from_u64(saat(nummer, k));
                let (art, stufe) = plan[k];
                let r = erzeuge_einzeln(art, stufe, &mut rng);
                *ergebnisse[k].lock().unwrap() = Some(r);
                fortschritt(fertig.fetch_add(1, Ordering::SeqCst) + 1, n);
            });
        }
    });

    ergebnisse
        .into_iter()
        .map(|m| m.into_inner().unwrap().expect("jedes Rätsel wurde erzeugt"))
        .collect()
}

/// Heutiges Datum, z. B. „15. November 2026“.
pub fn datum_heute() -> String {
    use chrono::Datelike;
    const MONATE: [&str; 12] = [
        "Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September",
        "Oktober", "November", "Dezember",
    ];
    let d = chrono::Local::now().date_naive();
    format!("{}. {} {}", d.day(), MONATE[d.month0() as usize], d.year())
}
