use {
    clap::Parser,
    std::path::PathBuf,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The pre-sorted source file, wich may contains duplicated lines
    pub src: PathBuf,

    /// The destination file, to write the deduplicated file to
    pub dest: Option<PathBuf>,
}
