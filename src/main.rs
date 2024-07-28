use clap::Parser;
use std::path::PathBuf;

/// Generates a meta directory structure to remember files on unplugged drives.
///
/// Standard output lists properly processed files separated by newlines.
#[derive(Parser, Debug)]
struct Args {
    source: PathBuf,
    destination: PathBuf,

    /// Max size in bytes to copy file unchanged
    #[arg(short, long, value_name = "SIZE_IN_BYTES", default_value = "0")]
    max_size: u64,

    /// Extension for meta files
    #[arg(short, long, value_name = "EXTENSION", default_value = ".drivetan.txt")]
    extension: String,

    /// Magic at the start of a meta file
    #[arg(long, value_name = "MAGIC", default_value = "DRIVETAN")]
    magic: String,

    /// TODO: Skip files/directories matching regexes in the provided file,
    /// separated by newlines (e.g. ".*\.git.*")
    #[arg(long, value_name = "PATH")]
    skip_file: Option<PathBuf>,
}

// i love non-unicode paths
const NON_UNICODE_PATH: &str = "<NON_UNICODE_PATH>";

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    check_args(&args)?;

    let walker = walkdir::WalkDir::new(&args.source).into_iter();

    let mut success_entries: u128 = 0;
    let mut error_entries: u128 = 0;

    for entry in walker {
        match entry {
            Err(err) => {
                eprintln!("{}, skipping...", err);
                error_entries += 1;
            }
            Ok(entry) => {
                match handle_direntry(&args, &entry) {
                    Err(err) => {
                        eprintln!("{}, skipping...", err);
                        error_entries += 1;
                    }
                    Ok(()) => {
                        println!("{}", entry.path().to_str().unwrap_or(NON_UNICODE_PATH));
                        success_entries += 1;
                    }
                };
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
        "success count: {}, error count: {}",
        success_entries, error_entries
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
        format!("{}B", len)
    } else if len < 1024 * 1024 {
        format!("{:.2}KiB", len as f64 / 1024.0)
    } else if len < 1024 * 1024 * 1024 {
        format!("{}MiB", len as f64 / (1024.0 * 1024.0))
    } else {
        format!("{}GiB", len as f64 / (1024.0 * 1024.0 * 1024.0))
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
    if args.skip_file.is_some() {
        todo!();
    }

    if !args.source.exists() {
        anyhow::bail!(
            "source path {} does not exist or cannot be accessed",
            args.source.to_str().unwrap_or(NON_UNICODE_PATH)
        );
    }

    if !args.destination.exists() {
        std::fs::create_dir(&args.destination)?;
    } else if std::fs::read_dir(&args.destination)?.next().is_some() {
        // TODO: some kind of diffing should probably be supported
        anyhow::bail!(
            "destination path {} is not empty",
            args.destination.to_str().unwrap_or(NON_UNICODE_PATH)
        );
    }

    Ok(())
}
