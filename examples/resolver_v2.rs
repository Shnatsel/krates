use clap::Parser;
use krates::cm::MetadataCommand;
use std::{collections::BTreeMap, fmt};

/// Prints the runtime and build-time dependency graphs, matching Cargo resolver v2
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    manifest_path: String,
    #[arg(long)]
    features: Vec<String>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
    #[arg(long)]
    target: Option<String>,
}

#[derive(Debug)]
pub struct Simple {
    id: krates::Kid,
    features: BTreeMap<String, Vec<String>>,
}

pub type Graph = krates::Krates<Simple>;

impl From<krates::cm::Package> for Simple {
    fn from(pkg: krates::cm::Package) -> Self {
        Self {
            id: pkg.id.into(),
            features: pkg.features
        }
    }
}

impl fmt::Display for Simple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id.repr)
    }
}

fn main() {
    let args = Args::parse();
    let manifest_path = args.manifest_path;
    let target_triple = args.target.unwrap_or_else(|| rustc_host_target_triple());

    let metadata = {
        let mut cmd = krates::Cmd::new();
        if args.all_features {
            cmd.all_features();
        }
        if args.no_default_features {
            cmd.no_default_features();
        }
        if !args.features.is_empty() {
            cmd.features(args.features);
        }
        // Restrict the dependencies to the default platform
        cmd.other_options(vec!["--filter-platform".to_owned(), target_triple]);
        cmd.manifest_path(manifest_path);
        let metadata_cmd: MetadataCommand = cmd.into();
        metadata_cmd.exec().expect("Failed to invoke `cargo metadata`")
    };

    let normal_deps: Graph = {
        let mut builder = krates::Builder::new();
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
        builder.build_with_metadata(metadata.clone(), krates::NoneFilter).unwrap()
    };

    let build_deps: Graph = {
        let mut builder = krates::Builder::new();
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        builder.build_with_metadata(metadata, krates::NoneFilter).unwrap()
    };

    let normal_nodes = normal_deps.graph().raw_nodes();
    let build_nodes = build_deps.graph().raw_nodes();

    println!("Normal dependency tree:");
    println!("{:#?}", normal_nodes);
    println!("Build-time depdency tree:");
    println!("{:#?}", build_nodes);
}


/// Returns the default target triple for the rustc we're using
fn rustc_host_target_triple() -> String {
    use std::io::BufRead;
    std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("Failed to invoke rustc! Is it in your $PATH?")
        .stdout
        .lines()
        .map(|l| l.unwrap())
        .find(|l| l.starts_with("host: "))
        .map(|l| l[6..].to_string())
        .expect("Failed to parse rustc output to determine the current platform!")
}