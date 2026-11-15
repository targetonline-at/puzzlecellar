# PuzzleCellar – Plan und Entscheidungen

Eine Windows-App (Rust), die auf Knopfdruck ein **abwechslungsreiches Rätselheft**
als PDF erzeugt – zum Ausdrucken. Immer neue, zufällig erzeugte Rätsel mit
eindeutiger Lösung.

## Leitidee

- **Das Rätselheft ist das Hauptformat.** Man soll nicht viel einstellen müssen:
  ein Knopf → ein gemischtes Heft. Einstellungen gibt es, sie haben aber gute
  Vorgaben (eher schwer, alle Rätselarten gemischt).
- **Alles visuell**, keine Kommandozeile.
- Zufallsauswahl von Rätselart und Schwierigkeit ist erwünscht.

## Heft-Layout

- **Zwei Papierformate** (Auswahl im Fenster): **A4** zum Ausdrucken und
  **170 × 240 mm** (Buch-/Magazinformat) für die Druckerei – gleiche Aufteilung,
  schmalere Ränder. Dateiname bekommt dann den Zusatz `_170x240`.
  Für Rückendrahtheftung muss die Seitenzahl ein Vielfaches von 4 sein (noch nicht automatisch).
- A4, **vier Rätsel pro Seite** (2 × 2) für Sudoku, Binoxo und Schiffe versenken;
  **Nonogramme je eine ganze Seite**. Die kleinen Rätsel werden auf volle Seiten
  aufgerundet (Vielfaches von 4), Nonogramm-Seiten liegen dazwischen.
- Kopf eines Rätsels nur **Nummer und Sterne** – kein Titel, keine Regelbeschreibung.
- **Nur schwer, sehr schwer und teuflisch** im Heft (leicht/mittel gibt es intern
  weiterhin, sie werden nur nicht ausgewählt).
- **Keine Lösungsbilder.** Bei Nonogrammen steht nur der Motivname als Auflösung –
  klein, hellgrau und **spiegelverkehrt** in der Fußzeile der Folgeseite (das letzte
  Nonogramm auf Seite 1), damit man das Bild nicht aus Versehen sieht.
- **Farbige Nonogramme auch schwarz-weiß druckbar:** jede Farbe hat zusätzlich ein Muster
  (einfarbig, Schraffur /, Punkte, waagrechte Streifen, Kreuzschraffur, Schraffur \, senkrechte
  Streifen) in den Zahlenkästchen und vorgegebenen Feldern; die Legende zeigt Muster und
  Farbnamen (automatisch aus dem RGB-Wert).

## Rätselarten und Wünsche

| Rätsel | Festlegungen |
|---|---|
| Sudoku | 9×9, Schwierigkeit über einen „menschlichen“ Löser bewertet (Lösungstechniken), bis „teuflisch“ |
| Binoxo | X/O, **10×10 oder 12×12** (zufällig). Regeln: nie drei gleiche nebeneinander, gleich viele X und O pro Linie, keine doppelten Zeilen/Spalten. Stufen: schwer = Linie ganz durchdenken, sehr schwer = Regel „keine doppelten Zeilen“ nötig, teuflisch = Annahme mit Widerspruch nötig |
| Schiffe versenken | Varianten (zufällig): **10×10 klassisch** (Flotte 1×4, 2×3, 3×2, 4×1; Hälfte der Rätsel), 10×10 mit bis zu 5 fehlenden Randzahlen (Viertel), 12×12 mit Flotte 1×5, 1×4, 2×3, 3×2, 4×1 (Viertel). Randzahlen rechts/unten, Flotte unter dem Gitter. Stufen: schwer = Flotte abhaken (fertige Schiffe, Schiffe abschließen, Felder ohne Platz fluten), sehr schwer = alle Lagen eines Schiffstyps durchgehen nötig, teuflisch = Annahme mit Widerspruch nötig |
| Nonogramme | **15×15 bis 25×25**, schwarz-weiß **und farbig**; Motive aus `src/raetsel/motive.txt` (Format dort erklärt, neue Motive einfach anhängen und neu bauen). Jedes Motiv wird zufällig gespiegelt; im Heft keine Wiederholung. Ein Löser prüft jedes Motiv; ist es nicht eindeutig, werden einzelne Felder vorgegeben. Stufe: 15 = schwer, bis 20 = sehr schwer, größer = teuflisch; braucht es Annahmen, eine Stufe mehr. Prüfen der Sammlung: `cargo run --release --example motive`. Stand: 37 Motive (9× schwer, 15× sehr schwer, 13× teuflisch); freie Fremdquellen geprüft und verworfen – wir zeichnen selbst |

Bewusst **nicht** gewünscht: Kommandozeilen-Modus, Bild-Import, abstrakte Zufallsbilder bei Nonogrammen.

## Technik

- Rust (GNU-Toolchain, keine Visual-Studio-Build-Tools nötig)
- `eframe`/`egui` – Fenster, Renderer **wgpu** (DirectX 12). Mit OpenGL (`glow`)
  blieb das Fenster auf der AMD-Grafikkarte weiß.
- `printpdf` – PDF (Vektorgrafik), Schriften werden zur Laufzeit aus `C:\Windows\Fonts` geladen
- `rand` – Zufall mit Startwert (Heftnummer → jedes Heft reproduzierbar)

### Toolchain-Kniffe (GNU ohne vollständiges MinGW)

Die schlanke GNU-Toolchain hat kein funktionierendes `dlltool` (es fehlt `as.exe`).
Pakete, die `raw-dylib` bzw. `windows-link` nutzen, lassen sich damit nicht bauen.
Deshalb sind in `Cargo.lock` ältere Versionen festgenagelt:

| Paket | Version |
|---|---|
| parking_lot / parking_lot_core | 0.12.3 / 0.9.10 |
| libloading | 0.8.6 |
| chrono | 0.4.39 |
| arboard | 3.4.1 |
| webbrowser | 1.0.4 |

`link-gnu/libshlwapi.a` (aus `winapi-x86_64-pc-windows-gnu`) ergänzt eine fehlende
Importbibliothek; `build.rs` macht sie dem Linker bekannt.
**Kein pauschales `cargo update`** – sonst kommen die neuen Versionen zurück.
Will man das loswerden: vollständiges MinGW (z. B. WinLibs) oder die MSVC-Build-Tools installieren.

## Entwickeln

- `cargo test --release --lib` – Tests (Eindeutigkeit, Bewertung, Korrektheit des Lösers)
- `cargo run --release --example testheft` – Testheft nach `target/testheft.pdf` mit Zeitmessung

## Etappen

1. ✅ Grundgerüst, Sudoku mit Schwierigkeitsbewertung, PDF-Heft, Fenster
2. ✅ Binoxo; Layout auf vier Rätsel pro Seite, ohne Lösungsteil
3. ✅ Schiffe versenken (+ Varianten)
4. ✅ Nonogramme (Motiv-Sammlung, schwarz-weiß und farbig) – mit Lösungsteil
5. Feinschliff (z. B. weitere Motive, Seitenzahl auf Vielfaches von 4 für die Druckerei, kleinere PDFs durch Schrift-Subsetting)

Ein Rätsel-Vorrat im Hintergrund ist unnötig: ein ganzes Heft entsteht in wenigen
Sekunden (am längsten brauchen teuflische Schiffe-Rätsel, etwa 1–3 s).

## Bauen und starten

```
bauen.bat     – baut die App (Release) und legt sie als PuzzleCellar.exe ins Projektverzeichnis
```
