use std::io;
use std::io::{BufRead, BufReader};

pub fn count_lines<R: io::Read>(handle: R) -> Result<u64, io::Error> {
    let mut reader = BufReader::new(handle);
    let mut count: u64 = 0;
    let mut line: Vec<u8> = Vec::new();
    while match reader.read_until(LF, &mut line) {
        Ok(n) if n > 0 => true,
        Err(e) => return Err(e),
        _ => false,
    } {
        if *line.last().unwrap() == LF {
            count += 1;
        };
    }
    Ok(count)
}