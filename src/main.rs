mod spinner;

use {
    crate::spinner::Spinner,
    clap::Parser,
    memchr::memchr,
    memmap2::Mmap,
    std::{
        ffi::OsStr,
        fs::File,
        io::{
            BufWriter,
            Write,
        },
        path::{
            Path,
            PathBuf,
        },
    },
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help(true))]
struct Cli {
    /// The pre-sorted source file, wich may contains duplicated lines
    pub src: PathBuf,
    /// The destination file, to write the deduplicated file to
    pub dest: Option<PathBuf>,
    /// Only count the number of duplicates
    #[arg(short, long)]
    pub count: bool,
}

#[inline(always)]
fn strip_eol(buf: &[u8]) -> &[u8] {
    let mut len = buf.len();
    if len > 0 && buf[len - 1] == b'\n' {
        len -= 1;
        if len > 0 && buf[len - 1] == b'\r' {
            len -= 1;
        }
    }
    &buf[..len]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let src_file = File::open(&cli.src)?;
    let mmap = unsafe { Mmap::map(&src_file)? };
    let data = &mmap[..];

    let mut duplicates_found: u64 = 0;
    let mut spinner = Spinner::new(
        ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"].to_vec(),
        "Checking for duplicates...".to_owned(),
    );

    let mut out: Option<BufWriter<File>> = if cli.count {
        None
    } else {
        let dest = cli.dest.unwrap_or_else(|| {
            let ext =
                cli.src.extension().and_then(OsStr::to_str).unwrap_or("");
            let stem = cli
                .src
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or("file");
            if ext.is_empty() {
                Path::new(&format!("{}_uniq", stem)).to_path_buf()
            } else {
                Path::new(&format!("{}_uniq.{}", stem, ext)).to_path_buf()
            }
        });

        if cli.src == dest {
            eprintln!(
                "Error: The source file must be different from the \
                 destination file."
            );
            std::process::exit(1);
        }

        Some(BufWriter::with_capacity(1 << 20, File::create(dest)?))
    };

    let mut prev: &[u8] = &[];
    let mut has_prev = false;
    let mut start = 0;

    while let Some(pos) = memchr(b'\n', &data[start..]) {
        let end = start + pos + 1; // Include the '\n'
        let line = strip_eol(&data[start..end]);

        if has_prev && prev == line {
            duplicates_found += 1;
        } else {
            if has_prev {
                if let Some(w) = out.as_mut() {
                    w.write_all(prev)?;
                    w.write_all(b"\n")?;
                }
            }
            prev = line;
            has_prev = true;
        }
        start = end;
    }

    // Handle trailing line without a final '\n'
    if start < data.len() {
        let line = strip_eol(&data[start..]);
        if has_prev && prev == line {
            duplicates_found += 1;
        } else {
            if has_prev {
                if let Some(w) = out.as_mut() {
                    w.write_all(prev)?;
                    w.write_all(b"\n")?;
                }
            }
            prev = line;
            has_prev = true;
        }
    }

    // Flush the last surviving unique line
    if has_prev {
        if let Some(w) = out.as_mut() {
            w.write_all(prev)?;
            w.write_all(b"\n")?;
        }
    }

    if let Some(mut w) = out {
        w.flush()?;
    }

    spinner
        .finish(Some(format!("🚀 Found {} duplicates)", duplicates_found)))?;
    Ok(())
}
