use haste::parser::{EventHandler, Parser};
use std::{collections::HashSet, fs::File, time::Instant};

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let replay_file_path =
        "H:\\SteamLibrary\\steamapps\\common\\dota 2 beta\\game\\dota\\replays\\7588607085.dem";
    let mut file = File::open(replay_file_path).unwrap();
    let mut handler = Handler::new();
    let start = Instant::now();
    Parser::new(&[], &[], true)
        .parse(&mut file, &mut handler)
        .unwrap();
    let duration = Instant::now().duration_since(start);
    eprintln!("Parsing took {:?}", duration);
}

struct Handler {
    pub class_names: HashSet<String>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            class_names: HashSet::new(),
        }
    }
}

impl EventHandler for Handler {
    fn on_entity_changed(
        &mut self,
        entity: &haste::entities::Entity,
        _context: &haste::parser::ParserContext,
    ) -> haste::Result<()> {
        self.class_names
            .insert(entity.class.as_ref().borrow().name.clone());
        Ok(())
    }
}
