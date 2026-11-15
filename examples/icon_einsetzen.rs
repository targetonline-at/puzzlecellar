//! Schreibt ein .ico als Programm-Icon in eine fertige .exe (Windows-Funktion UpdateResource).
//! Nötig, weil die schlanke GNU-Toolchain keinen Ressourcen-Compiler (windres) mitbringt.
//! Aufruf (macht bauen.bat automatisch):
//!   cargo run --release --example icon_einsetzen -- PuzzleCellar.exe assets\icon.ico

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;

#[link(name = "kernel32")]
extern "system" {
    fn BeginUpdateResourceW(datei: *const u16, alte_loeschen: i32) -> *mut c_void;
    fn UpdateResourceW(h: *mut c_void, typ: *const u16, name: *const u16, sprache: u16, daten: *const c_void, laenge: u32) -> i32;
    fn EndUpdateResourceW(h: *mut c_void, verwerfen: i32) -> i32;
}

const RT_ICON: usize = 3;
const RT_GROUP_ICON: usize = 14;

/// MAKEINTRESOURCE: eine Nummer als Ressourcen-„Name“
fn nummer(n: usize) -> *const u16 {
    n as *const u16
}

fn u16_bei(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}

fn u32_bei(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Aufruf: icon_einsetzen <Programm.exe> <icon.ico>");
        std::process::exit(2);
    }
    let ico = std::fs::read(&args[2]).unwrap_or_else(|e| panic!("{}: {e}", args[2]));
    assert!(ico.len() >= 6 && u16_bei(&ico, 2) == 1, "{} ist keine .ico-Datei", args[2]);
    let anzahl = u16_bei(&ico, 4) as usize;

    let exe: Vec<u16> = std::ffi::OsStr::new(&args[1]).encode_wide().chain(Some(0)).collect();
    // Gruppen-Verzeichnis: wie der ICO-Kopf, aber mit Ressourcen-Nummer statt Dateiposition
    let mut gruppe = Vec::new();
    gruppe.extend_from_slice(&[0, 0, 1, 0]);
    gruppe.extend_from_slice(&(anzahl as u16).to_le_bytes());
    unsafe {
        let h = BeginUpdateResourceW(exe.as_ptr(), 0);
        assert!(!h.is_null(), "{} lässt sich nicht öffnen (läuft das Programm noch?)", args[1]);
        for k in 0..anzahl {
            let e = 6 + 16 * k; // ICONDIRENTRY
            let laenge = u32_bei(&ico, e + 8) as usize;
            let start = u32_bei(&ico, e + 12) as usize;
            let bild = &ico[start..start + laenge];
            let ok = UpdateResourceW(h, nummer(RT_ICON), nummer(k + 1), 0, bild.as_ptr().cast(), laenge as u32);
            assert!(ok != 0, "Icon {k} konnte nicht geschrieben werden");
            gruppe.extend_from_slice(&ico[e..e + 12]); // Breite, Höhe, Farben, reserviert, Ebenen, Bit, Länge
            gruppe.extend_from_slice(&((k + 1) as u16).to_le_bytes());
        }
        let ok = UpdateResourceW(h, nummer(RT_GROUP_ICON), nummer(1), 0, gruppe.as_ptr().cast(), gruppe.len() as u32);
        assert!(ok != 0, "Icon-Gruppe konnte nicht geschrieben werden");
        assert!(EndUpdateResourceW(h, 0) != 0, "Icon konnte nicht gespeichert werden");
    }
    println!("Icon ({anzahl} Größen) in {} eingesetzt", args[1]);
}
