//! Entwickler-Werkzeug: erzeugt ein Testheft mit allen Stufen und misst die Zeiten.
//! Aufruf: cargo run --release --example testheft [--buch] [Ausgabe.pdf]

use puzzlecellar::heft::{self, Einstellungen, Mischung};
use puzzlecellar::pdf::{schreibe_heft, Format, HeftKopf};
use std::time::Instant;

fn main() {
    // Aufruf mit --buch erzeugt das Heft im Format 170 × 240 mm
    let buch = std::env::args().any(|a| a == "--buch");
    let ziel = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .unwrap_or_else(|| if buch { "target/testheft_buch.pdf" } else { "target/testheft.pdf" }.to_string());
    let einst = Einstellungen {
        anzahl: 12,
        mischung: Mischung::Ansteigend,
        ..Default::default()
    };
    let nummer = 4711;
    let plan = heft::plane(&einst, nummer);
    println!("Plan: {:?}", plan.iter().map(|(a, s)| format!("{} {}", a.name(), s.name())).collect::<Vec<_>>());

    let start = Instant::now();
    let raetsel = heft::erzeuge_raetsel(&plan, nummer, &|f, n| {
        println!("  {f}/{n} fertig nach {:.2} s", start.elapsed().as_secs_f32())
    });
    println!("Erzeugung: {:.2} s", start.elapsed().as_secs_f32());

    let kopf = HeftKopf { nummer, datum: heft::datum_heute() };
    let t = Instant::now();
    for (k, r) in raetsel.iter().enumerate() {
        use puzzlecellar::raetsel::Raetsel;
        match r {
            Raetsel::Binoxo(b) => {
                let vorgaben = b.aufgabe.iter().filter(|&&v| v != 0).count();
                println!("  {:>2}: Binoxo {}×{} {} – {vorgaben} Vorgaben, {}", k + 1, b.n, b.n, b.stufe.name(), b.schwerste_technik);
            }
            Raetsel::Schiffe(s) => {
                let fehlend = s.zeilen.iter().chain(&s.spalten).filter(|z| z.is_none()).count();
                println!(
                    "  {:>2}: Schiffe {:?} {} – {} Hinweise, {fehlend} Randzahlen fehlen, {}",
                    k + 1,
                    s.variante,
                    s.stufe.name(),
                    s.hinweise.len(),
                    s.schwerste_technik
                );
            }
            Raetsel::Nonogramm(ng) => {
                println!(
                    "  {:>2}: Nonogramm {} {}×{} {} – {} Vorgaben",
                    k + 1,
                    ng.name,
                    ng.breite,
                    ng.hoehe,
                    ng.stufe.name(),
                    ng.vorgaben.len()
                );
            }
            _ => {}
        }
    }
    schreibe_heft(&raetsel, &kopf, if buch { Format::Buch } else { Format::A4 }, std::path::Path::new(&ziel)).expect("PDF schreiben");
    println!("PDF: {:.2} s → {ziel}", t.elapsed().as_secs_f32());
}
