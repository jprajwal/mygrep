use std::ffi::OsStr;
use std::path::Path;

pub struct Glob {
    glob_pattern: String,
}

impl Glob {
    pub fn new(pattern: &String) -> Self {
        Self {
            glob_pattern: pattern.clone(),
        }
    }

    fn is_file_pattern_match(pattern: &OsStr, leafname: &OsStr) -> bool {
        // TODO: do something other than unwrap
        let pat_leaf = pattern.to_str().unwrap().chars().collect::<Vec<_>>();
        let pth_leaf = leafname.to_str().unwrap().chars().collect::<Vec<_>>();
        let (mut i, mut j) = (0, 0);
        let mut star_pos = pat_leaf.len();
        while i < pat_leaf.len() && j < pth_leaf.len() {
            if pat_leaf[i] == '*' {
                star_pos = i;
                break;
            } else if pat_leaf[i] != pth_leaf[j] {
                return false;
            } else {
                i += 1;
                j += 1;
            }
        }
        if i == pat_leaf.len() && j == pth_leaf.len() {
            return true;
        }
        i = pat_leaf.len() - 1;
        j = pth_leaf.len() - 1;
        while i > star_pos && j > 0 {
            if pat_leaf[i] != pth_leaf[j] {
                return false;
            }
            i -= 1;
            j -= 1;
        }
        if i > star_pos && j == 0 && pat_leaf[i] != pth_leaf[j] {
            return false;
        }
        return true;
    }

    pub fn is_match<S: AsRef<str>>(&self, filename: &S) -> bool {
        let path_pattern = Path::new(&self.glob_pattern);
        let path = Path::new(filename.as_ref());
        let mut pat_iter = path_pattern.iter().peekable();
        let mut pth_iter = path.iter().peekable();
        loop {
            let next_pat = pat_iter.next();
            let next_pth = pth_iter.next();
            // pat = ./foo/bar.py, pth = ./foo/bar.py
            // pat = ./**/*.py, pth = ./foo/bar.py
            match (next_pat, next_pth) {
                (Some(p), Some(q)) => {
                    if pth_iter.peek().is_some() {
                        if pat_iter.peek().is_some() {
                            if p != "**" && p != q {
                                return false;
                            } else {
                                continue;
                            }
                        } else {
                            if p == "**" {
                                return true;
                            } else {
                                return false;
                            }
                        }
                    } else {
                        let pattern = p.to_str().unwrap();
                        if pat_iter.peek().is_some() {
                            if pattern == "**" {
                                let next = pat_iter.next().unwrap();
                                let next_str = next.to_str().unwrap();
                                if next_str.contains('*') {
                                    return Self::is_file_pattern_match(next, q);
                                } else if next != q {
                                    return false;
                                }
                                return true;
                            } else {
                                return false;
                            }
                        } else if p == "**" {
                            return true;
                        } else if pattern.contains('*') {
                            return Self::is_file_pattern_match(p, q);
                        } else if p != q {
                            return false;
                        }
                        return true;
                    }
                }
                (None, None) => return true,
                (Some(p), None) if p == "**" => return true,
                (_, _) => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_1() {
        let path = String::from("./foo/bar.py");
        let pattern = String::from("./foo/bar.py");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
    }

    #[test]
    fn test_glob_2() {
        let pattern = String::from("./**/bar.py");
        let path = String::from("./foo/bar.py");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
    }

    #[test]
    fn test_glob_3() {
        let pattern = String::from("./**/*.py");
        let path = String::from("./foo/bar.py");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
    }

    #[test]
    fn test_glob_4() {
        let pattern = String::from("./**/*");
        let path_1 = String::from("./foo/bar.py");
        let path_2 = String::from("./foo/baz.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path_1), true);
        assert_eq!(Glob::new(&pattern).is_match(&path_2), true);
    }

    #[test]
    fn test_glob_5() {
        let pattern = String::from("./**");
        let path_1 = String::from("./foo/bar.py");
        let path_2 = String::from("./foo/baz.txt");
        let path_3 = String::from("./foo/bar/baz.txt");
        let path_4 = String::from("./foo/");
        assert_eq!(Glob::new(&pattern).is_match(&path_1), true);
        assert_eq!(Glob::new(&pattern).is_match(&path_2), true);
        assert_eq!(Glob::new(&pattern).is_match(&path_3), true);
        assert_eq!(Glob::new(&pattern).is_match(&path_4), true);
    }

    #[test]
    fn test_glob_6() {
        let pattern = String::from("foo/bar/baz/**/a.txt");
        let path = String::from("foo/bar/baz/a.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
        let pattern = String::from("foo/bar/baz/**/*.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
    }

    #[test]
    fn test_glob_7() {
        let pattern = String::from("foo/**/bar/baz/a.txt");
        let path = String::from("foo/bar/bar/baz/a.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), true);
    }

    #[test]
    fn test_glob_8() {
        let pattern = String::from("foo/**/bar.txt");
        let path = String::from("foo/baz.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
        let path = String::from("bar/bar.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
        let path = String::from("foo/bar");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
    }

    #[test]
    fn test_glob_9() {
        let pattern = String::from("foo/**/*.txt");
        let path = String::from("foo/bar/baz.py");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
    }

    #[test]
    fn test_glob_10() {
        let pattern = String::from("foo/**/*.txt");
        let path = String::from("foo/bar/baz/a.txt");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
    }

    #[test]
    fn test_glob_11() {
        let pattern = String::from("/foo/bar");
        let path = String::from("foo/bar");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
    }

    #[test]
    fn test_glob_12() {
        let pattern = String::from("./foo/bar");
        let path = String::from("foo/bar");
        assert_eq!(Glob::new(&pattern).is_match(&path), false);
    }

}
