use glob;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=vendor/github.com/SteamDatabase/Protobufs");
    let protos = glob::glob("vendor/github.com/SteamDatabase/Protobufs/dota2/*.proto")?
        .map(|path| Ok(path?.into_boxed_path()));
    prost_build::compile_protos(
        &protos.collect::<Result<Vec<_>>>()?,
        &["vendor/github.com/SteamDatabase/Protobufs/dota2"],
    )?;
    Ok(())
}
