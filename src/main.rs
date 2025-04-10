use clap::{ArgAction, Parser};
use std::boxed::Box;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Lines};
use std::iter::{Enumerate, Iterator};
use std::os::unix::fs::FileTypeExt;
use std::sync::mpsc;

mod thread_pool;

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
    with_filename: bool,

    #[arg(short = 'D', long, value_parser = ["read", "skip"], default_value = "skip")]
    devices: String,

    #[arg(short = 'r', long, action = ArgAction::SetTrue)]
    recursive: bool,

    #[arg(short = 'L', long, action = ArgAction::SetTrue)]
    files_without_match: bool,

    #[arg(short, long, action = ArgAction::SetTrue)]
    count: bool,
}

#[derive(Debug)]
struct GrepData {
    line_number: u32,
    line: String,
    filename: String,
}

impl std::default::Default for GrepData {
    fn default() -> Self {
        Self {
            line_number: 0,
            line: String::new(),
            filename: String::new(),
        }
    }
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

#[derive(Clone)]
struct GrepState {
    pattern: String,
    ignore_case: bool,
    invert_match: bool,
    no_messages: bool,
    max_count: u32,
    show_line_number: bool,
    with_filename: bool,
    devices: String,
    recursive: bool,
    files_without_match: bool,
    count: bool,
}

struct GrepIterator<'a, B: BufRead> {
    lines_iter: Enumerate<Lines<B>>,
    grep_state: &'a GrepState,
    filename: String,
}

impl<'a, B: BufRead> GrepIterator<'a, B> {
    fn new(e: Enumerate<Lines<B>>, grep_state: &'a GrepState, filename: String) -> Self {
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
                flag = is_case_insensitive_match(&self.grep_state.pattern, &line);
            } else {
                flag = is_match(&self.grep_state.pattern, &line);
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
    grep_state: &'a GrepState,
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

fn print_grep_data<'a>(grep_data: &GrepData, grep_state: &GrepState) {
    if grep_state.files_without_match {
        println!("{}", grep_data.filename);
        return;
    }
    if grep_state.with_filename {
        print!("{}: ", grep_data.filename);
    }
    if grep_state.show_line_number {
        print!("{}: ", grep_data.line_number);
    }
    println!("{}", grep_data.line);
}

struct GrepDirIterator<'a> {
    stack: Vec<std::io::Result<fs::ReadDir>>,
    grep_state: &'a GrepState,
}

impl<'a> GrepDirIterator<'a> {
    fn new(dir_iter: std::io::Result<fs::ReadDir>, grep_state: &'a GrepState) -> Self {
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
            if entry.metadata().unwrap().is_dir() {
                let dirname_res = entry.path().into_os_string().into_string();
                self.stack.push(Ok(dir_iter));
                if dirname_res.is_err() {
                    continue;
                }
                let dirname = dirname_res.unwrap();
                self.stack.push(fs::read_dir(&dirname));
                continue;
            } else {
                let filename_res = entry.path().into_os_string().into_string();
                self.stack.push(Ok(dir_iter));
                if filename_res.is_err() {
                    continue;
                }
                let filename = filename_res.unwrap();
                let file_type = fs::metadata(&filename).unwrap().file_type();
                if self.grep_state.devices == String::from("skip")
                    && (file_type.is_block_device() || file_type.is_fifo() || file_type.is_socket())
                {
                    continue;
                }
                return Some(grep_file(filename.clone(), self.grep_state).map_err(|e| {
                    std::io::Error::new(
                        e.as_ref()
                            .downcast_ref::<std::io::Error>()
                            .map_or(std::io::ErrorKind::Other, |e| e.kind()),
                        format!("{}: {}", filename, e),
                    )
                    .into()
                }));
            }
        }
    }
}

fn grep_dir<'a>(
    filename: &String,
    grep_state: &'a GrepState,
) -> Result<GrepDirIterator<'a>, Box<dyn Error>> {
    assert!(fs::exists(filename).is_ok_and(|x| x));
    Ok(GrepDirIterator::new(fs::read_dir(filename), grep_state))
}

fn divide_files_by_workers(files: Vec<String>, n_workers: usize) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    let mut collected_files = Vec::new();
    let mut collected_dirs = Vec::new();
    for file in files.iter() {
        let metadata = fs::metadata(file).unwrap();
        if metadata.is_dir() {
            collected_dirs.push(file.clone());
        } else {
            collected_files.push(file.clone());
        }
    }
    result.push(collected_files);
    let n_workers = n_workers - 1;
    let per_job = (collected_dirs.len() as i32 - 1) / n_workers as i32;
    if per_job < 0 {
        return result;
    }
    let per_job = per_job as usize;
    for i in 0..n_workers {
        let start = (per_job + 1) * i;
        let end = start + (per_job + 1);
        if end > collected_dirs.len() {
            return result;
        }
        result.push(
            collected_dirs[start..end]
                .iter()
                .map(|item| item.clone())
                .collect(),
        );
    }
    return result;
}

fn main() {
    let args = Args::parse();
    let grep_state = GrepState {
        pattern: args.pattern.clone(),
        ignore_case: args.ignore_case,
        invert_match: args.invert_match,
        no_messages: args.no_messages,
        max_count: args
            .max_count
            .map(|x| if x == 0 { u32::MAX } else { x })
            .unwrap_or(u32::MAX),
        show_line_number: args.line_number,
        with_filename: args.with_filename,
        devices: args.devices.clone(),
        recursive: args.recursive,
        files_without_match: args.files_without_match,
        count: args.count,
    };
    let grep_state_clone = grep_state.clone();

    let n_workers = 4;
    let jobs = divide_files_by_workers(args.file.clone(), n_workers);
    let mut pool = thread_pool::ThreadPool::new(n_workers);

    let (tx, rx) = mpsc::channel();
    for job in jobs {
        let tx = tx.clone();
        let grep_state = grep_state.clone();
        pool.execute(move || {
            for filename in job {
                let metadata_res = fs::metadata(&filename);
                if metadata_res.is_err() {
                    eprintln(
                        format!("{}", metadata_res.unwrap_err()),
                        !grep_state.no_messages,
                    );
                    continue;
                }
                let metadata = metadata_res.unwrap().file_type();
                if metadata.is_dir() {
                    if grep_state_clone.recursive == false {
                        eprintln(
                            format!("mygrep: {}: Is a directory", filename),
                            !grep_state_clone.no_messages,
                        );
                        continue;
                    }
                    match grep_dir(&filename, &grep_state) {
                        Err(e) => eprintln(format!("{}", e), !grep_state.no_messages),
                        Ok(dir_iter) => {
                            for file_res in dir_iter {
                                if file_res.is_err() {
                                    eprintln(
                                        format!("mygrep: {}", file_res.err().unwrap()),
                                        !grep_state.no_messages,
                                    );
                                    continue;
                                }
                                let file = file_res.unwrap();
                                let name = file.filename.clone();
                                let m = fs::metadata(&name).unwrap().file_type();
                                if grep_state.devices == String::from("skip")
                                    && (m.is_block_device() || m.is_fifo() || m.is_socket())
                                {
                                    continue;
                                }
                                let mut has_match = false;
                                for grep_data in file {
                                    has_match = true;
                                    if grep_state.files_without_match {
                                        break;
                                    }
                                    let _ = tx.send(grep_data);
                                }
                                if !has_match && grep_state.files_without_match {
                                    let mut grep_data = GrepData::default();
                                    grep_data.filename = name.clone();
                                    let _ = tx.send(grep_data);
                                }
                            }
                        }
                    }
                } else if grep_state.devices == String::from("skip") && !metadata.is_file() {
                    continue;
                } else if metadata.is_file()
                    || metadata.is_block_device()
                    || metadata.is_fifo()
                    || metadata.is_socket()
                {
                    match grep_file(filename.clone(), &grep_state) {
                        Err(e) => eprintln(format!("{}", e), !grep_state.no_messages),
                        Ok(iterator) => {
                            let mut has_match = false;
                            for grep_data in iterator {
                                has_match = true;
                                if grep_state.files_without_match {
                                    break;
                                }
                                let _ = tx.send(grep_data);
                            }
                            if !has_match && grep_state.files_without_match {
                                let mut grep_data = GrepData::default();
                                grep_data.filename = filename.clone();
                                let _ = tx.send(grep_data);
                            }
                        }
                    }
                }
            }
        });
    }
    drop(tx);
    let iter = rx.iter()
        .take(grep_state_clone.max_count as usize);
    if grep_state_clone.count {
        println!("{}", iter.count());
    } else {
        iter.for_each(|grep_data| {
            print_grep_data(&grep_data, &grep_state_clone);
        });
    }
    pool.join();
}
