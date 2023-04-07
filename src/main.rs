mod parser;
mod reader;

use {
    anyhow::Result,
    clap::Parser,
    core::time,
    indicatif::{
        ProgressBar,
        ProgressStyle,
    },
    parser::Cli,
    std::{
        ffi::OsStr,
        fs::File,
        io::{
            BufWriter,
            Write,
        },
        path::Path,
        rc::Rc,
    },
};

fn main() -> Result<()> {
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

    println!("Inputfile:  {:?}", cli.src);
    println!("Outputfile: {:?}", dest);

    let buf_reader = reader::BufReader::open(cli.src)?;
    let output = File::create(dest)?;
    let mut buf_writer = BufWriter::new(output);
    let mut line_last: Rc<String> = Rc::new(String::from(""));
    let mut dups: u64 = 0;

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(time::Duration::new(80, 0));
    pb.set_style(
        ProgressStyle::default_spinner()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")?,
    );
    pb.set_message("Checking for duplicates...");

    for line in buf_reader {
        let line_cur = match line {
            Ok(t) => t,
            Err(_) => {
                continue;
            },
        };
        if line_cur == line_last {
            dups += 1;
        // for debug purpose uncomment this line:
        // println!("Found DUP: {:?} and {:?}", line_cur, line_last)
        } else {
            write!(buf_writer, "{}", line_cur)?;
        }
        line_last = line_cur;
    }
    let msg = format!("Done. Found {} duplicates.", dups);
    buf_writer.flush().unwrap();
    pb.finish_with_message(msg);
    Ok(())
}
