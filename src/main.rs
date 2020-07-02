use indicatif::{ProgressBar, ProgressStyle};
use std::env;
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
    let args: Vec<String> = env::args().collect();
    //println!("{:?}", args);
    if args.len() < 2 {
        std::process::exit(1);
    }
    let input_filename: &str;
    let mut output_filename = "";
    let mut new_output_filename = String::from("");
    match args.len() {
        2 => {
            input_filename = &args[1];
            let ext = match get_extension_from_filename(input_filename) {
                Some(t) => t,
                None => "",
            };
            let out_file_stem = match remove_extension_from_filename(input_filename) {
                Some(t) => t,
                None => "",
            };
            new_output_filename = format!("{}_uniq.{}", out_file_stem, ext);
        }
        3 => {
            input_filename = &args[1];
            output_filename = &args[2];
        }
        _ => std::process::exit(1),
    };
    if !new_output_filename.is_empty() {
        output_filename = new_output_filename.as_str();
    }

    // println!("Inputfile:  {}", input_filename);
    // println!("Outputfile: {}", output_filename);

    let out_path = Path::new(output_filename);
    let lines = reader::BufReader::open(input_filename)?;
    let output = File::create(out_path)?;
    let mut writer = BufWriter::new(output);
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

    for line in lines {
        let line_cur = match line {
            Ok(t) => t,
            Err(_) => {
                continue;
            }
        };
        if line_cur == line_last {
            dups += 1;
        //println!("Found DUP: {:?} and {:?}", line_cur, line_last)
        } else {
            write!(writer, "{}", line_cur)?;
        }
        line_last = line_cur;
    }
    let msg = format!("Done. Found {} duplicates.", dups);
    writer.flush().unwrap();
    pb.finish_with_message(msg.as_str());
    Ok(())
}
