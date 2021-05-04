use clap::{crate_version, App, Arg};
use indicatif::{ProgressBar, ProgressStyle};
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

mod reader;

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

fn remove_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).file_stem().and_then(OsStr::to_str)
}

fn main() -> std::io::Result<()> {
    let matches = App::new("wordlist-dedup")
        .version(crate_version!())
        .author("Michael Sasser <Michael@MichaelSasser.org>")
        .about("Deduplicate presorted wordlists.")
        .arg(
            Arg::with_name("SRC")
                .required(true)
                .help("The presorted source file, wich may contains duplicated lines"),
        )
        .arg(
            Arg::with_name("DEST")
                .required(false)
                .help("The destination file, to write the deduplicated file to"),
        )
        .get_matches();

    let src_file = matches.value_of("SRC").unwrap();
    // println!("The file passed is: {}", myfile);

    // let dest_file = matches.value_of("DEST").unwrap_or("input.txt");
    let mut new_dest_file = String::from("");
    let mut dest_file;
    dest_file = match matches.value_of("DEST") {
        Some(t) => t,
        None => {
            let ext = match get_extension_from_filename(src_file) {
                Some(t) => t,
                None => "",
            };
            let out_file_stem = match remove_extension_from_filename(src_file) {
                Some(t) => t,
                None => "",
            };
            if ext.is_empty() {
                new_dest_file = format!("{}_uniq", out_file_stem);
            } else {
                new_dest_file = format!("{}_uniq.{}", out_file_stem, ext);
            }
            ""
        }
    };

    // let args: Vec<String> = env::args().collect();
    // let mut new_dest_file = String::from("");

    if !new_dest_file.is_empty() {
        dest_file = new_dest_file.as_str();
    }

    if dest_file == src_file {
        eprintln!("Error: The source file must be different from the destination file.");
        std::process::exit(1)
    }

    // println!("Inputfile:  {}", src_file);
    // println!("Outputfile: {}", dest_file);

    let out_path = Path::new(dest_file);
    let buf_reader = reader::BufReader::open(src_file)?;
    let output = File::create(out_path)?;
    let mut buf_writer = BufWriter::new(output);
    let mut line_last: Rc<String> = Rc::new(String::from(""));
    let mut dups: u64 = 0;

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(80);
    pb.set_style(
        ProgressStyle::default_spinner()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}"),
    );
    pb.set_message("Checking for duplicates...");

    for line in buf_reader {
        let line_cur = match line {
            Ok(t) => t,
            Err(_) => {
                continue;
            }
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
