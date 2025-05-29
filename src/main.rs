use std::env;
use std::path::PathBuf;
use std::process::exit;
use chromalog::{error, info, warn, ChromaLog, ColorConfig, LevelFilter};
use clap::Parser;
use elf::ElfBytes;
use elf::endian::AnyEndian;
use glob::PatternError;
use subprocess::Exec;

#[derive(Parser, Debug)]
#[command(about, long_about = None, arg_required_else_help = true)]
struct Args {
    #[arg(long, env = "APPDIR")]
    appdir: Option<PathBuf>,

    #[arg(long, env = "GLIBC_VERSION")]
    glibc_version: String,

    #[arg(long, default_value_t = false)]
    plugin_type: bool,

    #[arg(long, default_value_t = false)]
    plugin_api_version: bool,

    // note: cannot use env here because it does not accept any string as true
    // needs to be handled in parse_args()
    #[arg(long, default_value_t = false)]
    debug: bool,

}

fn parse_args() -> Args {
    let mut args = Args::parse();

    if args.plugin_type {
        println!("input");
        // println!("post-processing");
        exit(0);
    }

    if args.plugin_api_version {
        println!("0");
        exit(0);
    }

    if env::var_os("DEBUG").is_some() {
        args.debug = true;
    }

    if args.appdir.is_none() {
        error!("AppDir not specified");
        exit(1);
    }

    args
}

fn configure_logging(args: &Args) {
    let level_filter;
    // note: should be done in reverse order as we use an else-if chain
    // i.e., from most to least verbose
    if args.debug {
        level_filter = LevelFilter::Debug;
    } else {
        level_filter = LevelFilter::Info;
    }

    // unwrap() may not be the cleanest solution, but if setting up logging _really_ fails, it's
    // acceptable for the program to just terminate with an error message
    ChromaLog::init(level_filter, ColorConfig::default(), None).unwrap();
}

fn process_file(path: PathBuf, glibc_version: &str) {
    let proc = Exec::cmd("polyfill-glibc")
        .arg(&path)
        .arg(format!("--target-glibc={}", glibc_version))
        .join();
    match proc {
        Err(error) => {
            error!("Could not run polyfill-glibc: {}", error);
        }
        Ok(status) => {
            if !status.success() {
                warn!("polyfill-glibc process failed");
            }
        },
    }
}

fn process(files: Vec<PathBuf>, glibc_version: &str) {
    if files.is_empty() {
        warn!("no files found to process");
    }

    for entry in files {
        let path = entry.as_path();
        info!("Processing {}", path.display());
        let file = std::fs::read(&path).unwrap();
        let elf_file = ElfBytes::<AnyEndian>::minimal_parse(file.as_slice());

        match elf_file {
            Err(error) => {
                warn!("Failed to parse {} as an ELF file, skipping", error);
                return;
            }
            Ok(_) => {
                process_file(entry, glibc_version);
            }
        }
    }
}

fn glob_files(pattern: &str) -> Result<Vec<PathBuf>, PatternError> {
    let files = glob::glob(pattern)?;
    Ok(files.filter_map(Result::ok).filter(|x| x.is_file()).collect())
}

fn main() {
    let args = parse_args();

    configure_logging(&args);

    let appdir = args.appdir.unwrap();
    if !appdir.exists() {
        error!("AppDir {} does not exist", appdir.display());
        exit(1);
    }

    let libs_path = appdir.join("usr/lib/*");
    let bins_path = appdir.join("usr/lib/*");

    info!("Processing libs");
    let libs = glob_files(libs_path.to_str().unwrap()).unwrap();
    process(libs, args.glibc_version.as_str());

    info!("Processing bins");
    let bins = glob_files(bins_path.to_str().unwrap()).unwrap();
    process(bins, args.glibc_version.as_str());
}
