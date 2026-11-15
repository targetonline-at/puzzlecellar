//! Entwickler-Werkzeug: ein Heft mit allen Nonogramm-Motiven (zum Prüfen der Sammlung).
//! Aufruf: cargo run --release --example motive [Ausgabe.pdf]

use puzzlecellar::pdf::{schreibe_heft, Format, HeftKopf};
use puzzlecellar::raetsel::nonogramm::{katalog, waehle};
use puzzlecellar::raetsel::{Raetsel, Stufe};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() {
    let ziel = std::env::args().nth(1).unwrap_or_else(|| "target/motive.pdf".to_string());
    let n = katalog().len();
    let mut rng = StdRng::seed_from_u64(1);
    let raetsel: Vec<Raetsel> = waehle(&vec![Stufe::Schwer; n], &mut rng).into_iter().map(Raetsel::Nonogramm).collect();
    for (k, r) in raetsel.iter().enumerate() {
        if let Raetsel::Nonogramm(ng) = r {
            println!("{:>2}: {:<14} {}×{} {} – {} Vorgaben", k + 1, ng.name, ng.breite, ng.hoehe, ng.stufe.name(), ng.vorgaben.len());
        }
    }
    let kopf = HeftKopf { nummer: 1, datum: "Motiv-Sammlung".into() };
    schreibe_heft(&raetsel, &kopf, Format::A4, std::path::Path::new(&ziel)).expect("PDF schreiben");
    println!("→ {ziel}");
}
