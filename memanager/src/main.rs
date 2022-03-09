//! Using a hash map and vectors, create a text interface
//! to allow a user to add employee names to a department
//! in a company. Then let the user retrieve a list
//! of all people in a department or all people in the company
//! by department, sorted alphabetically.
//!
//! For example:
//! `Add Sally to Engineering`
//! `Add Amir to Sales`
//! `List All`
//! `List Engineering`
//! `Close`
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error::Error;
use std::io;

mod cmd;
use cmd::Cmd;

mod db;
use db::Db;

// Employee, Department => HashMap<Department, Employee>

fn main() -> Result<(), Box<dyn Error>> {
    let mut db = Db::new();

    loop {
        let mut buffer = String::new();

        println!("Enter your command =>");
        io::stdin().read_line(&mut buffer)?;

        // parse a command out of string.
        if !cmd::parse(&buffer).exec(&mut db) {
            break;
        }
    }

    Ok(())
}
