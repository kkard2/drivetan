use clap::Parser;
use regex::bytes::RegexSet;
use std::path::PathBuf;

/// Generates a meta directory structure to remember files on unplugged drives.
/// Standard output lists properly processed files separated by newlines.
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    source: PathBuf,
    destination: PathBuf,

    /// Max size in bytes to copy file unchanged.
    #[arg(short, long, value_name = "SIZE_IN_BYTES", default_value = "0")]
    max_size: u64,

    /// Extension for meta files.
    #[arg(short, long, value_name = "EXTENSION", default_value = ".drivetan.txt")]
    extension: String,

    /// Magic at the start of a meta file.
    #[arg(long, value_name = "MAGIC", default_value = "DRIVETAN")]
    magic: String,

    /// Skip files/directories matching regexes in the provided file,
    /// separated by newlines (e.g. "\.git").
    /// Empty folders do not have trailing slashes.
    #[arg(long, value_name = "PATH")]
    skip_file: Option<PathBuf>,
}

// i love non-unicode paths
const NON_UNICODE_PATH: &str = "<NON_UNICODE_PATH>";

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    check_args(&args)?;

    let walker = walkdir::WalkDir::new(&args.source).into_iter();

    let regex = match &args.skip_file {
        Some(skip_file) => RegexSet::new(
            match std::fs::read_to_string(skip_file) {
                Ok(it) => it,
                Err(err) => anyhow::bail!(
                    "could not read file {}: {}",
                    skip_file.to_str().unwrap_or(NON_UNICODE_PATH),
                    err
                ),
            }
            .lines(),
        )?,
        None => RegexSet::empty(),
    };

    let mut success_entries: u128 = 0;
    let mut error_entries: u128 = 0;
    let mut skipped_entries: u128 = 0;

    for entry in walker {
        match entry {
            Err(err) => {
                eprintln!("could not read directory entry: {}", err);
                error_entries += 1;
            }
            Ok(entry) => {
                if !regex.is_match(entry.path().as_os_str().as_encoded_bytes()) {
                    match handle_direntry(&args, &entry) {
                        Err(err) => {
                            eprintln!(
                                "handling directory entry {} failed: {}",
                                entry.path().to_str().unwrap_or(NON_UNICODE_PATH),
                                err
                            );
                            error_entries += 1;
                        }
                        Ok(()) => {
                            println!("{}", entry.path().to_str().unwrap_or(NON_UNICODE_PATH));
                            success_entries += 1;
                        }
                    };
                } else {
                    skipped_entries += 1;
                }
            }
        }
    }

    if success_entries == 0 {
        anyhow::bail!(
            "no entries successfuly processed; error count: {}",
            error_entries
        );
    }

    eprintln!(
        "success count: {}, error count: {}, skipped count: {}",
        success_entries, error_entries, skipped_entries
    );
    Ok(())
}

fn handle_direntry(args: &Args, direntry: &walkdir::DirEntry) -> anyhow::Result<()> {
    let diff = pathdiff::diff_paths(direntry.path(), &args.source);

    match diff {
        None => anyhow::bail!(
            "could not diff paths: {}, {}",
            direntry.path().to_str().unwrap_or(NON_UNICODE_PATH),
            &args.source.to_str().unwrap_or(NON_UNICODE_PATH)
        ),
        Some(diff) => {
            let meta = direntry.metadata()?;
            let mut dest = args.destination.join(diff);

            if meta.is_dir() {
                std::fs::create_dir_all(&dest)?;
            } else {
                if meta.len() > args.max_size {
                    let mut file_name = dest
                        .file_name()
                        .expect("not dir, should have a file name")
                        .to_os_string();
                    file_name.push(&args.extension);
                    dest.set_file_name(file_name);
                    std::fs::write(&dest, construct_meta_content(args, &meta))?;
                } else {
                    std::fs::copy(direntry.path(), &dest)?;
                }

                // NOTE: even if this fails, file is already created, which is fine.
                //       the only issue arises from the fact it's not stdouted with the rest and
                //       error is displayed.
                filetime::set_file_times(dest, meta.accessed()?.into(), meta.modified()?.into())?;
            }

            Ok(())
        }
    }
}

fn construct_meta_content(args: &Args, meta: &std::fs::Metadata) -> Vec<u8> {
    let len = meta.len();
    let human_size = if len < 1024 {
        format!("{} B", len)
    } else if len < 1024 * 1024 {
        format!("{:.2} KiB", len as f64 / 1024.0)
    } else if len < 1024 * 1024 * 1024 {
        format!("{:.2} MiB", len as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GiB", len as f64 / (1024.0 * 1024.0 * 1024.0))
    };

    let result = format!(
        "{}

size:       {}
human_size: {}
",
        args.magic, len, human_size
    );

    result.into()
}

fn check_args(args: &Args) -> anyhow::Result<()> {
    if !args.source.exists() {
        anyhow::bail!(
            "source path {} does not exist or cannot be accessed",
            args.source.to_str().unwrap_or(NON_UNICODE_PATH)
        );
    }

    if !args.destination.exists() {
        match std::fs::create_dir(&args.destination) {
            Ok(it) => it,
            Err(err) => anyhow::bail!("creating destination directory failed: {}", err),
        };
    } else if match std::fs::read_dir(&args.destination) {
        Ok(it) => it,
        Err(err) => anyhow::bail!("reading destination directory failed: {}", err),
    }
    .next()
    .is_some()
    {
        // TODO: some kind of diffing should probably be supported
        anyhow::bail!(
            "destination path {} is not empty",
            args.destination.to_str().unwrap_or(NON_UNICODE_PATH)
        );
    }

    Ok(())
}
