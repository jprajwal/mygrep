/*
* Regular expression:
* - Composition of:
*   - non special characters
*     - matches the characters in the string literally.
*   - special characters
*     - these characters are handled by parser.
*   - character class
*     - literally matches any one character in the class.
*     - if range is provided, then matches any character in the range.
*     - if caret is provided in the beginning, then performs inverse match.
*     - if ']' is provided in the beginning, then literally matches the ']'
*     - if named character classes are provided, then matches according to the
*       rules of the named character class.
*     - if [. then match according to the rules of collation.
*     - if [= then match according to the equivalence class.
*   - repeatition operators and interval expressions
*   - anchors
*   - alternatives
*   - concatination
*   - backslash expressions
*   - backreferences and subexpressions
*
*   - Regex evaluation process
*     1. Parse Regex Expression
*       - input: regular-expression: string
*       - ouput: intermediate-representation: Regex
*     2. Evaluate Regex Expression
*       - input: data: string
*       - output: match-object: Match
*
*/
use std::env;
use std::boxed::Box;

fn get_locale() -> Option<String> {
    env::var("LC_ALL")
        .or(env::var("LC_CTYPE"))
        .ok()
}

fn is_locale_c() -> bool {
    let locale = get_locale();
    match locale {
        Some(l) if *l == String::from("C") => return true,
        _ => return false,
    }
}

struct MatchState;

trait RegexTrait {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()>;
}

struct LiteralRegex {
    regex: Vec<u8>
}

impl LiteralRegex {
    fn new(expr: &[u8]) -> Self {
        let mut v = Vec::with_capacity(expr.len());
        v.extend_from_slice(expr);
        Self{ regex: v }
    }
}

impl RegexTrait for LiteralRegex {
    fn evaluate(&self, data: &[u8], start: usize, _match_state: &mut MatchState) -> Result<usize, ()> {
        let mut i = 0usize;
        while i < self.regex.len() {
            if start + i >= data.len() || self.regex[i] != data[start + i] {
                return Err(());
            }
            i += 1;
        }
        return Ok(i);
    }
}

struct CharClassRegex {
    regex: Vec<u8>
}

impl CharClassRegex {
    fn new(expr: &[u8]) -> Self {
        let mut v = Vec::with_capacity(expr.len());
        v.extend_from_slice(expr);
        Self{ regex: v }
    }
}

impl RegexTrait for CharClassRegex {
    fn evaluate(&self, data: &[u8], start: usize, _match_state: &mut MatchState) -> Result<usize, ()> {
        todo!()
    }
}

struct DotRegex;

impl DotRegex {
    fn new() -> Self {
        Self {}
    }
}

impl RegexTrait for DotRegex {
    fn evaluate(&self, data: &[u8], start: usize, _match_state: &mut MatchState) -> Result<usize, ()> {
        todo!()
    }
}

struct ZeroOrOneRegex {
    regex: Box<dyn RegexTrait>
}

impl ZeroOrOneRegex {
    fn new(re: Box<dyn RegexTrait>) -> Self {
        Self { regex: re }
    }
}

impl RegexTrait for ZeroOrOneRegex {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()> {
        self.regex.evaluate(data, start, match_state).or::<()>(Ok(0usize))
    }
}

struct ZeroOrMoreRegex {
    regex: Box<dyn RegexTrait>
}

impl ZeroOrMoreRegex {
    fn new(re: Box<dyn RegexTrait>) -> Self {
        Self { regex: re }
    }
}

impl RegexTrait for ZeroOrMoreRegex {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()> {
        let mut count = 0;
        while start + count < data.len() {
            let step_count = self.regex.evaluate(data, start + count, match_state).or::<()>(Ok(0usize)).unwrap();
            if step_count == 0 {
                 return Ok(count);
            }
            count += step_count
        }
        return Ok(count);
    }
}

struct OneOrMoreRegex {
    regex: Box<dyn RegexTrait>
}

impl OneOrMoreRegex {
    fn new(re: Box<dyn RegexTrait>) -> Self {
        Self { regex: re }
    }
}

impl RegexTrait for OneOrMoreRegex {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()> {
        let mut count = 0;
        let step_count = self.regex.evaluate(data, start + count, match_state)?;
        count += step_count;
        while start + count < data.len() {
            let step_count = self.regex.evaluate(data, start + count, match_state).or::<()>(Ok(0usize)).unwrap();
            if step_count == 0 {
                 return Ok(count);
            }
            count += step_count
        }
        return Ok(count);
    }
}

struct IntervalRegex {
    regex: Box<dyn RegexTrait>,
    atleast: usize,
    atmost: usize,
}

impl IntervalRegex {
    fn new(re: Box<dyn RegexTrait>, atleast: usize, atmost: usize) -> Self {
        Self { regex: re, atleast, atmost }
    }
}

impl RegexTrait for IntervalRegex {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()> {
        let mut i = 0;
        let (mut atleast, mut atmost) = (0usize, 0usize);
        while start + i < data.len() && atleast < self.atleast {
            let step_count = self.regex.evaluate(data, start + i, match_state)?;
            i += step_count;
            atleast += 1;
        }
        while start + i < data.len() && atmost < self.atmost {
            let step_count = self.regex.evaluate(data, start + i, match_state).or::<()>(Ok(0usize)).unwrap();
            if step_count == 0 {
                 return Ok(i);
            }
            i += step_count;
            atmost += 1;
        }
        return Ok(i);
    }
}

#[derive(Default)]
struct Regex {
    expr_list: Vec<Box<dyn RegexTrait>>,
}

impl RegexTrait for Regex {
    fn evaluate(&self, data: &[u8], start: usize, match_state: &mut MatchState) -> Result<usize, ()> {
        let mut i = 0;
        let mut begin = start;
        for expr in self.expr_list.iter() {
            if begin + i >= data.len() {
                return Err(());
            }
            i += expr.evaluate(data, begin + i, match_state)?;
        }
        return Ok(i);
    }
}

#[derive(Default)]
struct RegexBuilder {
    regex: Regex,
}

impl RegexBuilder {
    fn create_literal_regex(mut self, expr: &[u8]) -> Self {
        let regex = LiteralRegex::new(expr);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_char_class_regex(mut self, expr: &[u8]) -> Self {
        let regex = CharClassRegex::new(expr);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_dot_regex(mut self) -> Self {
        let regex = DotRegex::new();
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_zero_or_one_regex(mut self) -> Self {
        let last_opt = self.regex.expr_list.pop();
        if last_opt.is_none() {
            unreachable!();
        }
        let last = last_opt.unwrap();
        let regex = ZeroOrOneRegex::new(last);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_zero_or_more_regex(mut self) -> Self {
        let last_opt = self.regex.expr_list.pop();
        if last_opt.is_none() {
            unreachable!();
        }
        let last = last_opt.unwrap();
        let regex = ZeroOrMoreRegex::new(last);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_one_or_more_regex(mut self) -> Self {
        let last_opt = self.regex.expr_list.pop();
        if last_opt.is_none() {
            unreachable!();
        }
        let last = last_opt.unwrap();
        let regex = OneOrMoreRegex::new(last);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn create_interval_expr_regex(mut self, atleast: usize, atmost: usize) -> Self {
        let opt = self.regex.expr_list.pop();
        if opt.is_none() {
            unreachable!();
        }
        let last = opt.unwrap();
        let regex = IntervalRegex::new(last, atleast, atmost);
        self.regex.expr_list.push(Box::new(regex));
        self
    }

    fn build(self) -> Regex {
        return self.regex;
    }
}

#[derive(Default)]
struct ParseState {
    character_class_parsing_in_progress: bool,
    literal_string_in_progress: bool,
    stack: Vec<usize>,
    literal_string: Vec<u8>,
    caret_anchor_present: bool,
    dollar_anchor_present: bool,
    backslash_present: bool,
    interval_expr_parsing_in_progress: bool,
}

pub fn parse_regex(expr: &[u8]) -> Result<Regex, ()> {
    // ex: "test: [[:digit:]]"
    if !is_locale_c() {
        eprintln!("not c locale");
        return Err(());
    }
    let mut regex_builder = RegexBuilder::default();
    let mut state: ParseState = ParseState::default();
    let mut i = 0;
    while i < expr.len() {
        match expr[i] as char {
            '[' => {
                if !state.character_class_parsing_in_progress {
                    state.character_class_parsing_in_progress = true;
                    state.stack.push(i);
                }
            },
            ']' if state.character_class_parsing_in_progress => {
                let pop_opt = state.stack.pop();
                if pop_opt.is_none() {
                    unreachable!();
                }
                let start = pop_opt.unwrap();
                if state.stack.len() == 0 {
                    if i - start == 1 {
                        state.stack.push(start);
                        if !state.literal_string_in_progress {
                            state.literal_string.clear();
                        }
                        state.literal_string.push(expr[i]);
                    } else {
                        state.character_class_parsing_in_progress = false;
                        regex_builder = regex_builder.create_char_class_regex(&expr[start..i+1]);
                    }
                }
            },
            '^' if i == 0 => {
                state.caret_anchor_present = true;
            },
            '$' if i == expr.len() - 1 => {
                state.dollar_anchor_present = true;
            },
            '\\' if !state.backslash_present => {
                state.backslash_present = true;
            },
            '.' if !state.character_class_parsing_in_progress && !state.backslash_present => {
                if state.literal_string_in_progress {
                    regex_builder = regex_builder
                        .create_literal_regex(state.literal_string.as_slice());
                    state.literal_string.clear();
                    state.literal_string_in_progress = false;
                }
                regex_builder = regex_builder.create_dot_regex();
            },
            '?' if !state.character_class_parsing_in_progress && !state.backslash_present => {
                if state.literal_string_in_progress {
                    regex_builder = regex_builder
                        .create_literal_regex(state.literal_string.as_slice());
                    state.literal_string.clear();
                    state.literal_string_in_progress = false;
                }
                regex_builder = regex_builder.create_zero_or_one_regex();
            },
            '*' if !state.character_class_parsing_in_progress && !state.backslash_present => {
                if state.literal_string_in_progress {
                    regex_builder = regex_builder
                        .create_literal_regex(state.literal_string.as_slice());
                    state.literal_string.clear();
                    state.literal_string_in_progress = false;
                }
                regex_builder = regex_builder.create_zero_or_more_regex();
            },
            '+' if !state.character_class_parsing_in_progress && !state.backslash_present => {
                if state.literal_string_in_progress {
                    regex_builder = regex_builder
                        .create_literal_regex(state.literal_string.as_slice());
                    state.literal_string.clear();
                    state.literal_string_in_progress = false;
                }
                regex_builder = regex_builder.create_one_or_more_regex();
            },
            '{' if !state.character_class_parsing_in_progress && !state.backslash_present => {
                state.interval_expr_parsing_in_progress = true;
                state.stack.push(i);
            },
            '}' if !state.character_class_parsing_in_progress && !state.backslash_present && state.interval_expr_parsing_in_progress => {
                state.interval_expr_parsing_in_progress = false;
                let opt = state.stack.pop();
                if opt.is_none() {
                    unreachable!();
                }
                let start = opt.unwrap();
                let interval_slice = &expr[start+1..i];
                let interval_data = str::from_utf8(interval_slice).map_err(|_| ())?;
                // case 1: {}: treat it as literal
                // case 2: {,}: treat it as literal
                // case 3: {a}: treat it as literal
                // case 4: {1}: ok
                // case 5: {1,}: ok
                // case 6: {,1}: ok
                // case 7: {1,2}: ok
                if interval_data == "" {
                    if !state.literal_string_in_progress {
                        state.literal_string_in_progress = true;
                        state.literal_string.clear();
                    }
                    state.literal_string.extend_from_slice(interval_slice);
                    i += 1;
                    continue;
                }
                let mut split_iter = interval_data.split(|v| v == ',');
                let first = split_iter.next().unwrap();
                let first_num_res = usize::from_str_radix(first, 10);
                if first.len() > 0 && first_num_res.is_err() {
                    if !state.literal_string_in_progress {
                        state.literal_string_in_progress = true;
                        state.literal_string.clear();
                    }
                    state.literal_string.extend_from_slice(interval_slice);
                    i += 1;
                    continue;
                }
                let opt = split_iter.next();
                if opt.is_none() {
                    if first_num_res.is_err() {
                        if !state.literal_string_in_progress {
                            state.literal_string_in_progress = true;
                            state.literal_string.clear();
                        }
                        state.literal_string.extend_from_slice(interval_slice);
                        i += 1;
                        continue;
                    }
                    let first_num = first_num_res.unwrap();
                    regex_builder = regex_builder.create_interval_expr_regex(first_num, first_num);
                } else {
                    let second = opt.unwrap();
                    let second_res = usize::from_str_radix(second, 10);
                    if (second.len() > 0 && second_res.is_err()) || (first_num_res.is_err() && second_res.is_err()) {
                        if !state.literal_string_in_progress {
                            state.literal_string_in_progress = true;
                            state.literal_string.clear();
                        }
                        state.literal_string.extend_from_slice(interval_slice);
                        i += 1;
                        continue;
                    }

                    let first_num = first_num_res.unwrap_or_default();
                    let second_num = second_res.unwrap_or(usize::MAX);
                    regex_builder = regex_builder.create_interval_expr_regex(first_num, second_num);
                }
            },
            _ => {
                if !state.literal_string_in_progress {
                    regex_builder = regex_builder
                        .create_literal_regex(state.literal_string.as_slice());
                    state.literal_string.clear();
                }
                state.literal_string_in_progress = true;
                state.literal_string.push(expr[i]);
            },
        }
        i += 1;
    }
    Ok(regex_builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_literal_regex() {
        let regex = "test literal";
        let literal_regex = LiteralRegex::new(regex.as_bytes());
        assert_eq!(
            Ok(regex.len()),
            literal_regex.evaluate("test literal".as_bytes(), 0, &mut MatchState{})
        );
    }

    #[test]
    fn test_literal_regex_non_matching_case() {
        let regex = "test literal";
        let literal_regex = LiteralRegex::new(regex.as_bytes());
        assert_eq!(
            Err(()),
            literal_regex.evaluate("test non matching literal".as_bytes(), 0, &mut MatchState{})
        );
    }

    #[test]
    fn test_interval_regex() {
        let regex_str = "test{0,1}";
        let res = parse_regex(regex_str.as_bytes());
        let regex = res.unwrap();
        let data = "testtest";
        assert_eq!(Ok(data.len()-4), regex.evaluate(data.as_bytes(), 0, &mut MatchState{}));
    }
}
