use clap::{ArgAction, Parser};
use std::boxed::Box;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Lines};
use std::iter::{Enumerate, Iterator};

/// mygrep searches for PATTERNS in each FILE
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // #[arg(short = 'p')]
    // pattern: Option<String>,
    pattern: String,

    /// Search for PATTERN in each FILE
    #[arg(required = true)]
    file: Vec<String>,

    /// ignore case distinctions in patterns and data
    #[arg(short, long, action = ArgAction::SetTrue)]
    ignore_case: bool,

    /// invert match
    #[arg(short = 'v', long, action = ArgAction::SetTrue)]
    invert_match: bool,

    #[arg(short = 's', long, action = ArgAction::SetTrue)]
    no_messages: bool,

    #[arg(short = 'm', long)]
    max_count: Option<u32>,

    #[arg(short = 'n', long, action = ArgAction::SetTrue)]
    line_number: bool,

    #[arg(short = 'H', long, action = ArgAction::SetTrue)]
    no_filename: bool,
}

#[derive(Debug)]
struct GrepData {
    line_number: u32,
    line: String,
    filename: String,
}

fn is_match(pattern: &String, line: &String) -> bool {
    line.contains(pattern.as_str())
}

fn is_case_insensitive_match(pattern: &String, line: &String) -> bool {
    is_match(&pattern.to_lowercase(), &line.to_lowercase())
}

fn eprintln(msg: String, ok: bool) {
    if ok {
        eprintln!("{}", msg);
    }
}

struct GrepState<'a> {
    pattern: &'a String,
    ignore_case: bool,
    invert_match: bool,
    no_messages: bool,
    max_count: u32,
    show_line_number: bool,
    no_filename: bool,
}

struct GrepIterator<'a, B: BufRead> {
    lines_iter: Enumerate<Lines<B>>,
    grep_state: &'a GrepState<'a>,
    filename: String,
}

impl<'a, B: BufRead> GrepIterator<'a, B> {
    fn new(e: Enumerate<Lines<B>>, grep_state: &'a GrepState<'a>, filename: String) -> Self {
        GrepIterator {
            lines_iter: e,
            grep_state,
            filename: filename.clone(),
        }
    }
}

impl<'a, B: BufRead> Iterator for GrepIterator<'a, B> {
    // type Item = Enumerate<&'a String>::Item;
    type Item = GrepData;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, line) = self.lines_iter.next()?;
            if !line.is_ok() {
                continue;
            }
            let line = line.unwrap();
            let mut flag: bool;
            if self.grep_state.ignore_case {
                flag = is_case_insensitive_match(self.grep_state.pattern, &line);
            } else {
                flag = is_match(self.grep_state.pattern, &line);
            }
            if self.grep_state.invert_match {
                flag = !flag;
            }
            if flag {
                let grep_data = GrepData {
                    line_number: (i + 1) as u32,
                    line: line.clone(),
                    filename: self.filename.clone(),
                };
                return Some(grep_data);
            }
        }
    }
}

fn grep_file<'a>(
    filename: String,
    grep_state: &'a GrepState<'a>,
) -> Result<GrepIterator<'a, BufReader<fs::File>>, Box<dyn Error>> {
    assert!(fs::exists(&filename).is_ok_and(|x| x));
    let file = fs::File::open(&filename)?;
    let reader = BufReader::new(file);
    Ok(GrepIterator::new(
        reader.lines().enumerate(),
        &grep_state,
        filename,
    ))
}

fn print_grep_data<'a>(grep_data: &GrepData, grep_state: &GrepState<'a>) {
    if !grep_state.no_filename {
        print!("{}: ", grep_data.filename);
    }
    if grep_state.show_line_number {
        print!("{}: ", grep_data.line_number);
    }
    println!("{}", grep_data.line);
}

struct GrepDirIterator<'a> {
    stack: Vec<std::io::Result<fs::ReadDir>>,
    grep_state: &'a GrepState<'a>,
}

impl<'a> GrepDirIterator<'a> {
    fn new(dir_iter: std::io::Result<fs::ReadDir>, grep_state: &'a GrepState<'a>) -> Self {
        GrepDirIterator {
            stack: vec![dir_iter],
            grep_state,
        }
    }
}

impl<'a> Iterator for GrepDirIterator<'a> {
    type Item = Result<GrepIterator<'a, BufReader<fs::File>>, Box<dyn Error>>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let dir_iter_res = self.stack.pop()?;
            if dir_iter_res.is_err() {
                continue;
            }

            let mut dir_iter = dir_iter_res.unwrap();
            let entry_op = dir_iter.next();
            if entry_op.is_none() {
                continue;
            }
            let entry_res = entry_op.unwrap();
            if entry_res.is_err() {
                self.stack.push(Ok(dir_iter));
                continue;
            }
            let entry = entry_res.unwrap();
            if entry.metadata().is_err() {
                self.stack.push(Ok(dir_iter));
                continue;
            }
            if entry.metadata().unwrap().is_file() {
                let filename_res = entry.path().into_os_string().into_string();
                // eprintln!("filename: {:?}", filename_res);
                self.stack.push(Ok(dir_iter));
                if filename_res.is_err() {
                    continue;
                }
                let filename = filename_res.unwrap();
                return Some(grep_file(filename, self.grep_state));
            }
            if entry.metadata().unwrap().is_dir() {
                let dirname_res = entry.path().into_os_string().into_string();
                self.stack.push(Ok(dir_iter));
                // eprintln!("dirname: {:?}", dirname_res);
                if dirname_res.is_err() {
                    continue;
                }
                let dirname = dirname_res.unwrap();
                self.stack.push(fs::read_dir(&dirname));
                continue;
            }
        }
    }
}

fn grep_dir<'a>(
    filename: &String,
    grep_state: &'a GrepState<'a>,
) -> Result<GrepDirIterator<'a>, Box<dyn Error>> {
    assert!(fs::exists(filename).is_ok_and(|x| x));
    Ok(GrepDirIterator::new(fs::read_dir(filename), grep_state))
}

fn main() {
    let args = Args::parse();
    let grep_state = GrepState {
        pattern: &args.pattern,
        ignore_case: args.ignore_case,
        invert_match: args.invert_match,
        no_messages: args.no_messages,
        max_count: args.max_count.map(|x| if x == 0 {u32::MAX} else {x}).unwrap_or(u32::MAX),
        show_line_number: args.line_number,
        no_filename: args.no_filename,
    };

    let mut remaining_count = grep_state.max_count as i64;
    // eprintln!("beginning remaining_count = {}", remaining_count);
    for filename in &args.file {
        let metadata_res = fs::metadata(filename);
        if metadata_res.is_err() {
            eprintln(
                format!("{}", metadata_res.unwrap_err()),
                !grep_state.no_messages,
            );
            continue;
        }
        let metadata = metadata_res.unwrap();
        if metadata.is_dir() {
            match grep_dir(filename, &grep_state) {
                Err(e) => eprintln(format!("{}", e), !grep_state.no_messages),
                Ok(dir_iter) => {
                    for file_res in dir_iter {
                        if file_res.is_err() {
                            eprintln(format!("{}", file_res.err().unwrap()), !grep_state.no_messages);
                            continue;
                        }
                        let file = file_res.unwrap();
                        for grep_data in file {
                            // eprintln!("grep_data: {:?}, {}", grep_data, remaining_count);
                            if remaining_count <= 0 {
                                return;
                            }
                            print_grep_data(&grep_data, &grep_state);
                            remaining_count -= 1;
                        }
                    }
                }
            }
        }
        if metadata.is_file() {
            match grep_file(filename.clone(), &grep_state) {
                Err(e) => eprintln(format!("{}", e), !grep_state.no_messages),
                Ok(iterator) => {
                    for grep_data in iterator {
                        if remaining_count <= 0 {
                            return;
                        }
                        print_grep_data(&grep_data, &grep_state);
                        remaining_count -= 1;
                    }
                }
            }
        }
    }
}
