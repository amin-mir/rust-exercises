use std::error::Error;

pub enum Cmd {
    Add {
        employee: String,
        department: String,
    },
    ListAll,
    ListDepartment(String),
    Close,
}

/// TODO: Read on custom erros and state machines.
pub fn parse(ss: &str) -> Result<Cmd, Box<dyn Error>> {
    let mut parts = ss.split_whitespace();

    let command = parts.next().ok_or("not enough parts")?;
    match command {
        "Add" => parse_add(parts),
        "List" => parse_list(parts),
        "Close" => Ok(Cmd::Close),
        _ => Err("unknown command".into()),
    }
}

fn parse_add<'a, T>(mut parts: T) -> Result<Cmd, Box<dyn Error>>
where
    T: Iterator<Item = &'a str>,
{
    let employee = match parts.next() {
        Some(emp) => emp,
        None => return Err("`Add` command needs employee name".into()),
    };

    match parts.next() {
        Some(to) if to == "to" => (),
        _ => return Err("employee name should be followed by `to` preposition".into()),
    }

    let department = match parts.next() {
        Some(dpt) => dpt,
        None => return Err("`Add` command needs department".into()),
    };

    Ok(Cmd::Add {
        employee: employee.to_owned(),
        department: department.to_owned(),
    })
}

fn parse_list<'a, T>(mut parts: T) -> Result<Cmd, Box<dyn Error>>
where
    T: Iterator<Item = &'a str>,
{
    
    Ok(Cmd::Close)
}
