use {
    clap::Parser,
    indicatif::{
        ProgressBar,
        ProgressStyle,
    },
    memchr::memchr,
    memmap2::Mmap,
    rayon::prelude::*,
    std::{
        ffi::OsStr,
        fs::File,
        io,
        os::unix::fs::FileExt,
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

#[derive(Default)]
struct ChunkInfo {
    start: usize,
    end: usize,
    drop_first: bool,
    offset: u64,
    output_size: u64,
    local_dups: u64,
    local_lines: u64,
    first_line: Vec<u8>,
    last_line: Vec<u8>,
}

struct ChunkOffsets {
    total_dups: u64,
    total_output_size: u64,
    total_unique_lines: u64,
}

impl ChunkOffsets {
    fn new(
        total_dups: u64,
        total_output_size: u64,
        total_unique_lines: u64,
    ) -> Self {
        Self {
            total_dups,
            total_output_size,
            total_unique_lines,
        }
    }
}

#[inline(always)]
fn strip_eol(buf: &[u8]) -> &[u8] {
    let mut len = buf.len();
    if len > 0 && buf[len - 1] == b'\n' {
        len -= 1;
        if len > 0 && buf[len - 1] == b'\r' {
            len -= 1;
        }
    } else if len > 0 && buf[len - 1] == b'\r' {
        len -= 1;
    }
    &buf[..len]
}

fn compute_chunks(data: &[u8], num_threads: usize) -> Vec<ChunkInfo> {
    let num_threads = num_threads.max(1);
    let chunk_size = (data.len() / num_threads).max(1);
    let mut chunks = Vec::with_capacity(num_threads);
    let mut start = 0;

    for i in 0..num_threads {
        if start >= data.len() {
            break;
        }
        let mut end = if i == num_threads - 1 {
            data.len()
        } else {
            std::cmp::min(start + chunk_size, data.len())
        };

        if end < data.len() {
            if let Some(p) = memchr(b'\n', &data[end..]) {
                end += p + 1;
            } else {
                end = data.len();
            }
        }

        chunks.push(ChunkInfo {
            start,
            end,
            ..Default::default()
        });
        start = end;
    }
    chunks
}

/// Scans chunks in parallel
fn analyze_chunks(chunks: &mut [ChunkInfo], data: &[u8], pb: &ProgressBar) {
    chunks.par_iter_mut().for_each(|chunk| {
        let chunk_data = &data[chunk.start..chunk.end];
        let mut prev: &[u8] = &[];
        let mut has_prev = false;
        let mut start = 0;

        let mut output_size: u64 = 0;
        let mut local_dups: u64 = 0;
        let mut local_lines: u64 = 0;
        let mut first_line: Vec<u8> = Vec::new();
        let mut last_line: Vec<u8> = Vec::new();

        let mut local_progress: u64 = 0;

        while let Some(pos) = memchr(b'\n', &chunk_data[start..]) {
            let line = strip_eol(&chunk_data[start..start + pos]);

            if !has_prev {
                first_line.extend_from_slice(line);
            }

            if has_prev && prev == line {
                local_dups += 1;
            } else {
                if has_prev {
                    output_size += (prev.len() as u64) + 1;
                    local_lines += 1;
                }
                prev = line;
                has_prev = true;
            }

            last_line.clear();
            last_line.extend_from_slice(line);
            start += pos + 1;

            local_progress += (pos + 1) as u64;
            if local_progress >= 1 << 20 {
                pb.inc(local_progress);
                local_progress = 0;
            }
        }

        if start < chunk_data.len() {
            let line = strip_eol(&chunk_data[start..]);
            if !has_prev {
                first_line.extend_from_slice(line);
            }
            if has_prev && prev == line {
                local_dups += 1;
            } else {
                if has_prev {
                    output_size += (prev.len() as u64) + 1;
                    local_lines += 1;
                }
                prev = line;
                has_prev = true;
            }
            last_line.clear();
            last_line.extend_from_slice(line);
            local_progress += (chunk_data.len() - start) as u64;
        }

        if has_prev {
            output_size += (prev.len() as u64) + 1;
            local_lines += 1;
        }

        pb.inc(local_progress);

        chunk.output_size = output_size;
        chunk.local_dups = local_dups;
        chunk.local_lines = local_lines;
        chunk.first_line = first_line;
        chunk.last_line = last_line;
    });
}

/// Calculates actual offsets.
fn stitch_chunks(chunks: &mut [ChunkInfo]) -> ChunkOffsets {
    let mut total_dups: u64 = 0;
    let mut total_lines: u64 = 0;
    let mut current_offset: u64 = 0;

    for i in 0..chunks.len() {
        if i > 0 && !chunks[i].first_line.is_empty() {
            if chunks[i - 1].last_line == chunks[i].first_line {
                chunks[i].drop_first = true;
                chunks[i].output_size -=
                    (chunks[i].first_line.len() as u64) + 1;
                chunks[i].local_lines -= 1;
                total_dups += 1;
            }
        }
        chunks[i].offset = current_offset;
        current_offset += chunks[i].output_size;
        total_dups += chunks[i].local_dups;
        total_lines += chunks[i].local_lines;
    }

    ChunkOffsets::new(total_dups, current_offset, total_lines)
}

/// Writes deduplicated chunks in parallel.
fn write_output(
    chunks: &[ChunkInfo],
    data: &[u8],
    total_output_size: u64,
    dest: &Path,
    pb: &ProgressBar,
) -> io::Result<()> {
    let output_file = File::create(dest)?;
    output_file.set_len(total_output_size)?;

    chunks.par_iter().try_for_each(|chunk| -> io::Result<()> {
        let chunk_data = &data[chunk.start..chunk.end];
        let mut prev: &[u8] = &[];
        let mut has_prev = false;
        let mut start = 0;
        let mut write_offset = chunk.offset;
        let mut drop_first = chunk.drop_first;

        let mut buf: Vec<u8> = Vec::with_capacity(1 << 20);

        let flush_buf =
            |buf: &mut Vec<u8>, offset: &mut u64| -> io::Result<()> {
                if !buf.is_empty() {
                    output_file.write_all_at(buf, *offset)?;
                    pb.inc(buf.len() as u64);
                    *offset += buf.len() as u64;
                    buf.clear();
                }
                Ok(())
            };

        while let Some(pos) = memchr(b'\n', &chunk_data[start..]) {
            let line = strip_eol(&chunk_data[start..start + pos]);

            if has_prev && prev == line {
                // duplicate
            } else {
                if has_prev {
                    if drop_first {
                        drop_first = false;
                    } else {
                        buf.extend_from_slice(prev);
                        buf.push(b'\n');
                        if buf.len() >= 1 << 20 {
                            flush_buf(&mut buf, &mut write_offset)?;
                        }
                    }
                }
                prev = line;
                has_prev = true;
            }
            start += pos + 1;
        }

        if start < chunk_data.len() {
            let line = strip_eol(&chunk_data[start..]);
            if has_prev && prev == line {
                // duplicate
            } else {
                if has_prev {
                    if drop_first {
                        drop_first = false;
                    } else {
                        buf.extend_from_slice(prev);
                        buf.push(b'\n');
                    }
                }
                prev = line;
                has_prev = true;
            }
        }

        if has_prev && !drop_first {
            buf.extend_from_slice(prev);
            buf.push(b'\n');
        }

        flush_buf(&mut buf, &mut write_offset)?;
        Ok(())
    })?;

    Ok(())
}

fn resolve_dest_path(cli: &Cli) -> PathBuf {
    cli.dest.clone().unwrap_or_else(|| {
        let ext = cli.src.extension().and_then(OsStr::to_str).unwrap_or("");
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
    })
}

fn print_headline(title: &str) {
    let prefix = format!("── {} ", title);
    let dash_count = 80_usize.saturating_sub(prefix.chars().count());
    let dashes = "─".repeat(dash_count);
    println!("\x1b[36m{}{}\x1b[0m\n", prefix, dashes);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let src_file = File::open(&cli.src)?;
    let mmap = unsafe { Mmap::map(&src_file)? };
    let data = &mmap[..];

    let style = ProgressStyle::with_template(
        "[{elapsed}] {bar:35.cyan/blue} {bytes}/{total_bytes} \
         ({bytes_per_sec}, {eta})",
    )
    .unwrap()
    .progress_chars("██▒");

    // Analyzing
    print_headline("Analyzing");

    let pb_analyze = ProgressBar::new(data.len() as u64);
    pb_analyze.set_style(style.clone());

    let mut chunks = compute_chunks(data, rayon::current_num_threads());
    analyze_chunks(&mut chunks, data, &pb_analyze);
    let chunk_offsets = stitch_chunks(&mut chunks);

    pb_analyze.finish();
    println!(
        "\n  \x1b[32m✔\x1b[0m  Found {} duplicates\n",
        chunk_offsets.total_dups
    );

    // Writing
    if !cli.count {
        let dest = resolve_dest_path(&cli);

        if cli.src == dest {
            eprintln!(
                "Error: The source file must be different from the \
                 destination file."
            );
            std::process::exit(1);
        }

        print_headline("Writing");

        let pb_write = ProgressBar::new(chunk_offsets.total_output_size);
        pb_write.set_style(style.clone());

        write_output(
            &chunks,
            data,
            chunk_offsets.total_output_size,
            &dest,
            &pb_write,
        )?;

        pb_write.finish();
        println!(
            "\n  \x1b[32m✔\x1b[0m  Written {} unique lines to {}\n",
            chunk_offsets.total_unique_lines,
            dest.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        indicatif::ProgressBar,
        tempfile::tempdir,
    };

    /// Helper function to run the full pipeline on in-memory data
    /// and return (duplicates_found, output_file_contents).
    fn run_pipeline(data: &[u8], num_chunks: usize) -> (u64, Vec<u8>) {
        let mut chunks = compute_chunks(data, num_chunks);

        let pb = ProgressBar::hidden();

        analyze_chunks(&mut chunks, data, &pb);
        let chunk_offsets = stitch_chunks(&mut chunks);

        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("output.txt");

        write_output(
            &chunks,
            data,
            chunk_offsets.total_output_size,
            &dest_path,
            &pb,
        )
        .unwrap();

        let output_data = std::fs::read(&dest_path).unwrap();
        (chunk_offsets.total_dups, output_data)
    }

    #[test]
    fn test_strip_eol() {
        assert_eq!(strip_eol(b"hello\n"), b"hello");
        assert_eq!(strip_eol(b"hello\r\n"), b"hello");
        assert_eq!(strip_eol(b"hello"), b"hello");
        assert_eq!(strip_eol(b""), b"");
    }

    #[test]
    fn test_no_duplicates_single_chunk() {
        let input = b"a\nb\nc\n";
        let (dups, output) = run_pipeline(input, 1);
        assert_eq!(dups, 0);
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_no_duplicates_multi_chunk() {
        let input = b"a\nb\nc\nd\ne\nf\n";
        let (dups, output) = run_pipeline(input, 3);
        assert_eq!(dups, 0);
        assert_eq!(output, b"a\nb\nc\nd\ne\nf\n");
    }

    #[test]
    fn test_basic_duplicates_single_chunk() {
        let input = b"a\na\nb\nb\nc\n";
        let (dups, output) = run_pipeline(input, 1);
        assert_eq!(dups, 2);
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_duplicates_across_boundaries() {
        let input = b"a\na\nb\nb\nc\nc\n";
        let (dups, output) = run_pipeline(input, 3);
        assert_eq!(dups, 3);
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_large_boundary_stress() {
        let mut input = Vec::new();
        for _ in 0..100_000 {
            input.extend_from_slice(b"a\n");
        }

        let (dups, output) = run_pipeline(&input, 8);
        assert_eq!(dups, 99_999);
        assert_eq!(output, b"a\n");
    }

    #[test]
    fn test_missing_final_newline_single_chunk() {
        let input = b"a\na\nb";
        let (dups, output) = run_pipeline(input, 1);
        assert_eq!(dups, 1);
        assert_eq!(output, b"a\nb\n");
    }

    #[test]
    fn test_missing_final_newline_multi_chunk() {
        let input = b"a\na\nb\nb\nc";
        let (dups, output) = run_pipeline(input, 2);
        assert_eq!(dups, 2);
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_windows_crlf_line_endings() {
        let input = b"a\r\na\r\nb\r\nb\r\nc\r\n";
        let (dups, output) = run_pipeline(input, 2);
        assert_eq!(dups, 2);
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_empty_file() {
        let input = b"";
        let (dups, output) = run_pipeline(input, 4);
        assert_eq!(dups, 0);
        assert_eq!(output, b"");
    }

    #[test]
    fn test_single_line_file() {
        let input = b"only line\n";
        let (dups, output) = run_pipeline(input, 4);
        assert_eq!(dups, 0);
        assert_eq!(output, b"only line\n");
    }

    #[test]
    fn test_empty_lines_duplicates() {
        let input = b"a\n\n\nb\n";
        let (dups, output) = run_pipeline(input, 2);
        assert_eq!(dups, 1);
        assert_eq!(output, b"a\n\nb\n");
    }

    #[test]
    fn test_resolve_dest_path_with_extension() {
        let cli = Cli {
            src: PathBuf::from("words.txt"),
            dest: None,
            count: false,
        };
        assert_eq!(resolve_dest_path(&cli), PathBuf::from("words_uniq.txt"));
    }

    #[test]
    fn test_resolve_dest_path_without_extension() {
        let cli = Cli {
            src: PathBuf::from("words"),
            dest: None,
            count: false,
        };
        assert_eq!(resolve_dest_path(&cli), PathBuf::from("words_uniq"));
    }

    #[test]
    fn test_resolve_dest_path_explicit_dest() {
        let cli = Cli {
            src: PathBuf::from("words.txt"),
            dest: Some(PathBuf::from("custom_output.txt")),
            count: false,
        };
        assert_eq!(
            resolve_dest_path(&cli),
            PathBuf::from("custom_output.txt")
        );
    }

    #[test]
    fn test_count_only_mode() {
        let input = b"a\na\nb\nc\nc\nc\n";
        let mut chunks = compute_chunks(input, 3);
        let pb = ProgressBar::hidden();
        analyze_chunks(&mut chunks, input, &pb);
        let chunk_offsets = stitch_chunks(&mut chunks);

        assert_eq!(chunk_offsets.total_dups, 3);
    }
}
