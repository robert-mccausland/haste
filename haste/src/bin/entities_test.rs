use haste::parser::{EventHandler, Parser};
use std::{fs::File, time::Instant};

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let replay_file_path =
        "H:\\SteamLibrary\\steamapps\\common\\dota 2 beta\\game\\dota\\replays\\7588607085.dem";
    let mut file = File::open(replay_file_path).unwrap();
    let handler = Handler {};
    let start = Instant::now();
    Parser::new(handler, &[], &[], true)
        .parse(&mut file)
        .unwrap();
    let duration = Instant::now().duration_since(start);
    eprintln!("Parsing took {:?}", duration);
}

struct Handler {}

impl EventHandler for Handler {}
