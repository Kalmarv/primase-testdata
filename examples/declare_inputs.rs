//! Declares a crate's real-data inputs and asks for one. With `GAME_DATA` unset this prints one SKIP line to standard
//! error and returns.
mod inputs {
    primase_testdata::inputs! {
        games {
            Sekiro = "sekiro",
        }
        installs {
            Sekiro {
                about: "Sekiro's install folder",
                source: "the user's own install",
                gate: true,
            },
        }
        keys {}
        tools {
            Oodle6 = "oodle6" {
                about: "an Oodle version-6 DLL",
                source: "the game's own DLL from the user's install",
                gate: true,
            },
        }
        oracle_sets {
            Corpus = "corpus" {
                about: "the games' own files cut from their installs",
                source: "the user's own installs: files extracted as stored",
            },
        }
        declared {
            Query::Oracle(OracleSet::Corpus, Game::Sekiro, "dcx") => {
                status: Live,
                gate: false,
                about: "Sekiro's DCX files as stored",
                source: "the user's own install",
            },
        }
        debug {
            "oodle.dump" => "a folder under the scratch root the Oodle gate writes to",
        }
    }
}

use inputs::{Game, OracleSet};

fn main() {
    assert!(inputs::REGISTRY.problems().is_empty());
    let Some(install) = inputs::install(Game::Sekiro) else {
        return; // `installs.sekiro` is not configured here: the SKIP line names it
    };
    println!("Sekiro is installed at {}", install.display());
    if let Some(folder) = inputs::oracle(OracleSet::Corpus, Game::Sekiro, "dcx") {
        println!("DCX files are in {}", folder.display());
    }
}
