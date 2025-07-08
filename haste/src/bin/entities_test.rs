use ahash::AHashSet;
use haste::parser::{EventHandler, Parser};
use std::{fs::File, time::Instant};

fn main() {
    let match_id = 8364473605u64;
    //let match_id = 7588607085u64;
    let replay_file_path = format!(
        "H:\\SteamLibrary\\steamapps\\common\\dota 2 beta\\game\\dota\\replays\\{:}.dem",
        match_id
    );
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
    pub class_names: AHashSet<String>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            class_names: AHashSet::new(),
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
