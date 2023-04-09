mod spinner;

use {
    crate::spinner::Spinner,
    clap::Parser,
    itertools::Itertools,
    std::{
        ffi::OsStr,
        fs::File,
        io::{
            BufRead,
            BufReader,
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
pub struct Cli {
    /// The pre-sorted source file, wich may contains duplicated lines
    pub src: PathBuf,

    /// The destination file, to write the deduplicated file to
    pub dest: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let dest = cli.dest.unwrap_or_else(|| {
        let ext = cli.src.extension().and_then(OsStr::to_str).unwrap_or("");
        let out_file_stem = cli
            .src
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("file");

        if ext.is_empty() {
            Path::new(&format!("{}_uniq", out_file_stem)).to_path_buf()
        } else {
            Path::new(&format!("{}_uniq.{}", out_file_stem, ext)).to_path_buf()
        }
    });

    if cli.src == dest {
        // Ensure not to overwrite input file
        eprintln!(
            "Error: The source file must be different from the destination \
             file."
        );
        std::process::exit(1)
    }

    println!("Deduplicating: {:?} 🠢 {:?}\n", cli.src, dest);

    let src_file = File::open(cli.src)?;
    let src_reader = BufReader::new(src_file);

    let output = File::create(dest)?;
    let mut buf_writer = BufWriter::new(output);
    let mut duplicates_found: u64 = 0;

    let mut spinner = Spinner::new(
        ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"].to_vec(),
        "Checking for duplicates...".to_owned(),
    );

    let it = src_reader.lines().map(|l| l.unwrap());

    for (u, v) in it.tuple_windows() {
        if u == v {
            duplicates_found += 1;
            // println!("Found DUP: {}", &u);
            continue;
        } else {
            writeln!(buf_writer, "{}", u)?;
        }
    }
    buf_writer.flush()?;

    spinner
        .finish(Some(format!("🚀 Found {} duplicates)", duplicates_found)))?;
    Ok(())
}
