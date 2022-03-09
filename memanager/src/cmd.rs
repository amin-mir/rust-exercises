use crate::db::Db;

pub enum Cmd {
    Add { dpt: String, empl: String },
    ListAll,
    ListDepartment(String),
    Close,
    Unknown(String),
}

/// (1) Read on custom erros and state machines.
/// (2) how to keep using different part os the input string without
///     cloning them? below we're using a ton of to_owned! what's the more
///     performant way for achieving the same thing?
/// (3) How to not perform heap allocations for fixed strings?
pub fn parse(ss: &str) -> Cmd {
    let mut parts = ss.split_whitespace();

    let p = match parts.next() {
        Some(p) => p,
        None => return Cmd::Unknown("not enough parts".to_owned()),
    };

    match p {
        "Add" => parse_add(parts),
        "List" => parse_list(parts),
        "Close" => Cmd::Close,
        _ => Cmd::Unknown("unknown command".to_owned()),
    }
}

fn parse_add<'a, T>(mut parts: T) -> Cmd
where
    T: Iterator<Item = &'a str>,
{
    let empl = match parts.next() {
        Some(e) => e,
        None => return Cmd::Unknown("`Add` command needs employee name".to_owned()),
    };

    match parts.next() {
        Some(to) if to == "to" => (),
        _ => {
            return Cmd::Unknown("employee name should be followed by `to` preposition".to_owned())
        }
    }

    let dpt = match parts.next() {
        Some(d) => d,
        None => return Cmd::Unknown("`Add` command needs department".to_owned()),
    };

    Cmd::Add {
        empl: empl.to_owned(),
        dpt: dpt.to_owned(),
    }
}

fn parse_list<'a, T>(mut parts: T) -> Cmd
where
    T: Iterator<Item = &'a str>,
{
    match parts.next() {
        Some(dpt) if dpt == "All" => Cmd::ListAll,
        Some(dpt) => Cmd::ListDepartment(dpt.into()),
        None => return Cmd::Unknown("`List` command requires department as argument".to_owned()),
    }
}

impl Cmd {
    pub fn exec(self, db: &mut Db) -> bool {
        match self {
            Cmd::Add { dpt, empl } => {
                db.add_empl(dpt, empl);
                println!("success\n");
                true
            }
            Cmd::ListAll => {
                for (dpt, empl) in db.get_all_dpt_empls() {
                    println!("{} => {}", dpt, empl);
                }
                println!();
                true
            }
            Cmd::ListDepartment(dpt) => {
                for empl in db.get_empls(&dpt) {
                    print!("{}, ", empl);
                }
                println!("");
                true
            }
            Cmd::Close => false,
            Cmd::Unknown(reason) => {
                println!("{}\n", reason);
                true
            }
        }
    }
}
