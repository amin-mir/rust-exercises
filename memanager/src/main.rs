//! Using a hash map and vectors, create a text interface
//! to allow a user to add employee names to a department
//! in a company. Then let the user retrieve a list
//! of all people in a department or all people in the company
//! by department, sorted alphabetically.
//!
//! For example:
//! `Add Sally to Engineering`
//! `Add Amir to Sales`
//! `List all`
//! `List Engineering`
//! `Close`
use std::collections::HashMap;
use std::error::Error;
use std::io;

mod cmd;

// Employee, Department => HashMap<Department, Employee>

fn main() -> Result<(), Box<dyn Error>> {
    let db: HashMap<String, String> = HashMap::new();

    // loop {
    let mut buffer = String::new();

    println!("Please go ahead and enter your command =>");
    io::stdin().read_line(&mut buffer)?;

    // parse a command out of string.
    match cmd::parse(&buffer)? {
        cmd::Cmd::Add {
            employee,
            department,
        } => println!("request for adding {} to {}", employee, department),
        _ => println!("command is not add"),
    }

    Ok(())

    // execute the command.
    // }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
