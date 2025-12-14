use parsebuf::{FallBack as Or, InwardStrategy as Strat, ParseCursor, PatternLoc as Loc};
use stable_string_patterns_method::WhiteSpace;

#[derive(Debug, PartialEq)]
struct ErrorLog<'a> {
    msg: &'a str,
    hint: Option<&'a str>,
    file: Option<&'a str>,
    loc: Option<(u64, u64)>,
}


fn parse(input: &'_ str) -> Option<ErrorLog<'_>> {
    let mut cursor = ParseCursor::new_empty_start(input);

    cursor
        .back_forward("ERROR:", Loc::BeginningOnce, Strat::WholeData)
        .ok()?;

    cursor.front_forward_or("(", Loc::FirstExcluded, Or::ToTheEnd);
    let msg = cursor.cursor().trim();
    cursor.back_to_front();

    let mut parens = cursor.step(|c| {
        c.back_forward('(', Loc::FirstIncluded, Strat::WholeData)?;
        c.front_forward(')', Loc::FirstExcluded)
    });

    let Some(first_par) = parens.next() else {
        return Some(ErrorLog {
            msg,
            hint: None,
            file: None,
            loc: None,
        });
    };

    let file;
    let loc;
    let hint;
    if first_par.starts_with('/') {
        let mut first_par = ParseCursor::new_empty_start(first_par);
        first_par
            .front_forward(WhiteSpace, Loc::FirstExcluded)
            .unwrap();
        file = Some(first_par.cursor());
        let parse_num_prefix = |c: &mut ParseCursor, pref| {
            c.back_forward(pref, Loc::FirstIncluded, Strat::WholeData)
                .unwrap()
                .front_forward(|c: char| c.is_ascii_digit(), Loc::BeginningMany)
                .unwrap();
            c.cursor().parse().unwrap()
        };
        let line = parse_num_prefix(&mut first_par, "at line ");
        let column = parse_num_prefix(&mut first_par, ", column ");
        loc = Some((line, column));
        hint = parens.next();
    } else {
        file = None;
        loc = None;
        hint = Some(first_par);
    }
    Some(ErrorLog {
        msg,
        hint,
        file,
        loc,
    })
}

// Error message syntax:
// ERROR: <msg> [(<file path> at line <line>, column <col>)] [(<hint>)]
// (disambiguated because file path starts with /)

fn main() {
    let inputs = [
        "ERROR: unflagazed plungus (/file/location at line 42, column 67) (plungus was flagazed line 2)",
        "ERROR: fluxmoxxed (very bad!)",
        "ERROR: bazoombled (/etc/baz at line 0, column 6)",
        "Not an error message",
    ];

    for input in inputs {
        println!("{input}");
        let value = parse(input);
        dbg!(value);
        println!("------")
    }
}
