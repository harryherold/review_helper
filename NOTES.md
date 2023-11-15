*  Read notes into a buffer like
```rs
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Lines, Result};
​
fn read_lines(file_path: &str) -> Result<Lines<BufReader<File>>> {
    let file = File::open(file_path)?;
    Ok(BufReader::new(file).lines())
}
​
fn main() {
    match read_lines("example.txt") {
        Ok(lines) => {
            for line in lines {
                if let Ok(ip) = line {
                    println!("{}", ip);
                }
            }
        }
        Err(e) => println!("Error reading file: {}", e),
    }
}
```