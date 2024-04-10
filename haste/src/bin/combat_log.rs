use haste::parser::{EventHandler, Parser, ParserContext};
use haste_protobuf::{dota::DotaCombatlogTypes, Packet, PacketKind};
use std::{fs::File, io::Write, time::Instant};

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let replay_file_path =
        "H:\\SteamLibrary\\steamapps\\common\\dota 2 beta\\game\\dota\\replays\\7588607085.dem";
    let mut file = File::open(replay_file_path).unwrap();
    let mut handler = CombatLogHandler {
        output: File::create("./combat-log.txt").unwrap(),
    };
    let start = Instant::now();
    Parser::new(&[], &[PacketKind::CombatLogData], false)
        .parse(&mut file, &mut handler)
        .unwrap();
    let duration = Instant::now().duration_since(start);
    eprintln!("Parsing took {:?}", duration);
}

struct CombatLogHandler<W: Write> {
    output: W,
}

const DEFAULT_NAME: &str = "NULL";

impl<W: Write> EventHandler for CombatLogHandler<W> {
    fn on_packet(&mut self, demo: Packet, context: &ParserContext) -> haste::Result<()> {
        if let Packet::CombatLogData(entry) = demo {
            let combat_log_names = context.string_tables.get_by_name("CombatLogNames");
            let get_name = |index: u32| -> &str {
                combat_log_names
                    .and_then(|names| names.get_entry(index).map(|entry| entry.name))
                    .unwrap_or(DEFAULT_NAME)
            };

            let seconds = entry.timestamp();
            let time = format!(
                "[{:02}:{:02}:{:02}.{:03}]",
                (seconds / 3600.0).floor(),
                ((seconds % 3600.0) / 60.0).floor(),
                (seconds % 60.0).floor(),
                ((seconds % 1.0) * 1000.0).round()
            );

            match entry.r#type() {
                DotaCombatlogTypes::DotaCombatlogDamage => writeln!(
                    &mut self.output,
                    "{} {} hits {}{} for {} damage{}",
                    time,
                    get_name(entry.attacker_name()),
                    get_name(entry.target_name()),
                    match entry.inflictor_name() {
                        0 => "".to_owned(),
                        x => format!(" with {}", get_name(x)),
                    },
                    entry.value(),
                    match entry.health() {
                        0 => "".to_owned(),
                        health => format!(" ({}->{})", health + entry.value() as i32, health),
                    }
                ),
                DotaCombatlogTypes::DotaCombatlogHeal => writeln!(
                    &mut self.output,
                    "{} {}'s {} heals {} for {} health({}->{})",
                    time,
                    get_name(entry.attacker_name()),
                    get_name(entry.inflictor_name()),
                    get_name(entry.target_name()),
                    entry.value(),
                    entry.health() - entry.value() as i32,
                    entry.health()
                ),
                DotaCombatlogTypes::DotaCombatlogModifierAdd => writeln!(
                    &mut self.output,
                    "{} {} receives {} buff/debuff from {}",
                    time,
                    get_name(entry.target_name()),
                    get_name(entry.inflictor_name()),
                    get_name(entry.attacker_name()),
                ),
                DotaCombatlogTypes::DotaCombatlogModifierRemove => writeln!(
                    &mut self.output,
                    "{} {} loses {} buff/debuff from {}",
                    time,
                    get_name(entry.target_name()),
                    get_name(entry.inflictor_name()),
                    get_name(entry.attacker_name()),
                ),
                DotaCombatlogTypes::DotaCombatlogDeath => writeln!(
                    &mut self.output,
                    "{} {} is killed by {}",
                    time,
                    get_name(entry.target_name()),
                    get_name(entry.attacker_name()),
                ),
                DotaCombatlogTypes::DotaCombatlogAbility => writeln!(
                    &mut self.output,
                    "{} {} {} ability {} (lvl {}){}{}",
                    time,
                    get_name(entry.attacker_name()),
                    match entry.is_ability_toggle_on() || entry.is_ability_toggle_on() {
                        true => "toggles",
                        false => "casts",
                    },
                    get_name(entry.inflictor_name()),
                    entry.ability_level(),
                    match (entry.is_ability_toggle_on(), entry.is_ability_toggle_on()) {
                        (true, _) => " on",
                        (_, true) => " off",
                        _ => "",
                    },
                    match entry.target_name() {
                        0 => "".to_owned(),
                        x => format!(" on {}", get_name(x)),
                    },
                ),
                DotaCombatlogTypes::DotaCombatlogItem => writeln!(
                    &mut self.output,
                    "{} {} uses {}",
                    time,
                    get_name(entry.attacker_name()),
                    get_name(entry.inflictor_name()),
                ),
                DotaCombatlogTypes::DotaCombatlogGold => writeln!(
                    &mut self.output,
                    "{} {} {} {} gold",
                    time,
                    get_name(entry.target_name()),
                    match entry.value() as i32 > 0 {
                        true => "receives",
                        false => "looses",
                    },
                    (entry.value() as i32).abs_diff(0)
                ),
                DotaCombatlogTypes::DotaCombatlogGameState => writeln!(
                    &mut self.output,
                    "{} game state is now {}",
                    time,
                    entry.value()
                ),
                DotaCombatlogTypes::DotaCombatlogXp => writeln!(
                    &mut self.output,
                    "{} {} gains {} XP",
                    time,
                    get_name(entry.target_name()),
                    entry.value()
                ),
                DotaCombatlogTypes::DotaCombatlogPurchase => writeln!(
                    &mut self.output,
                    "{} {} buys item {}",
                    time,
                    get_name(entry.target_name()),
                    get_name(entry.value()),
                ),
                DotaCombatlogTypes::DotaCombatlogBuyback => writeln!(
                    &mut self.output,
                    "{} player in slot {} has bought back",
                    time,
                    entry.value()
                ),
                _ => Ok(()),
            }?;
        }
        Ok(())
    }
}
