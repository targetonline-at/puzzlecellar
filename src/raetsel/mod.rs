//! Alle Rätselarten und was sie gemeinsam haben.

pub mod binoxo;
pub mod nonogramm;
pub mod schiffe;
pub mod sudoku;

/// Schwierigkeitsstufe – gilt für alle Rätselarten.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stufe {
    Leicht,
    Mittel,
    Schwer,
    SehrSchwer,
    Teuflisch,
}

impl Stufe {
    pub const ALLE: [Stufe; 5] = [
        Stufe::Leicht,
        Stufe::Mittel,
        Stufe::Schwer,
        Stufe::SehrSchwer,
        Stufe::Teuflisch,
    ];

    /// Die Stufen, die ins Heft kommen – leicht und mittel sind nicht gewünscht.
    pub const IM_HEFT: [Stufe; 3] = [Stufe::Schwer, Stufe::SehrSchwer, Stufe::Teuflisch];

    pub fn name(self) -> &'static str {
        match self {
            Stufe::Leicht => "leicht",
            Stufe::Mittel => "mittel",
            Stufe::Schwer => "schwer",
            Stufe::SehrSchwer => "sehr schwer",
            Stufe::Teuflisch => "teuflisch",
        }
    }

    /// 1 bis 5 Sterne
    pub fn sterne(self) -> u8 {
        self as u8 + 1
    }
}

/// Wie viel Platz ein Rätsel im Heft beansprucht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platz {
    /// Vier Rätsel pro Seite
    Viertelseite,
    Halbseite,
    Ganzseite,
}

/// Ein fertig erzeugtes Rätsel beliebiger Art.
#[derive(Clone, Debug)]
pub enum Raetsel {
    Sudoku(sudoku::Sudoku),
    Binoxo(binoxo::Binoxo),
    Schiffe(schiffe::Schiffe),
    Nonogramm(nonogramm::Nonogramm),
}

impl Raetsel {
    pub fn art_name(&self) -> &'static str {
        match self {
            Raetsel::Sudoku(_) => "Sudoku",
            Raetsel::Binoxo(_) => "Binoxo",
            Raetsel::Schiffe(_) => "Schiffe versenken",
            Raetsel::Nonogramm(_) => "Nonogramm",
        }
    }

    pub fn stufe(&self) -> Stufe {
        match self {
            Raetsel::Sudoku(s) => s.stufe,
            Raetsel::Binoxo(b) => b.stufe,
            Raetsel::Schiffe(s) => s.stufe,
            Raetsel::Nonogramm(n) => n.stufe,
        }
    }

    pub fn platz(&self) -> Platz {
        match self {
            Raetsel::Sudoku(_) | Raetsel::Binoxo(_) | Raetsel::Schiffe(_) => Platz::Viertelseite,
            // Zwei Nonogramme teilen sich eine Seite – spart Papier
            Raetsel::Nonogramm(_) => Platz::Halbseite,
        }
    }

    /// Nur Nonogramme bekommen eine Lösung hinten im Heft.
    pub fn hat_loesung(&self) -> bool {
        matches!(self, Raetsel::Nonogramm(_))
    }
}
