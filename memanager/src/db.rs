use std::collections::{hash_map::Entry, HashMap};

pub struct Db {
    db: HashMap<String, Vec<String>>,
}

pub enum AddEmplResult {
    Added,
    AlreadyExists,
}

impl Db {
    pub fn new() -> Self {
        Self { db: HashMap::new() }
    }

    /// adds an employee to a new department.
    pub fn add_empl(&mut self, dpt: String, empl: String) -> AddEmplResult {
        match self.db.entry(dpt) {
            Entry::Occupied(mut o) => {
                let empls = o.get_mut();
                if empls.contains(&empl) {
                    AddEmplResult::AlreadyExists
                } else {
                    empls.push(empl);
                    AddEmplResult::Added
                }
            }
            Entry::Vacant(v) => {
                v.insert(vec![empl]);
                AddEmplResult::Added
            }
        }
    }

    // get all employees.
    pub fn get_all_empls(&self) -> impl Iterator<Item = &str> {
        self.db
            .iter()
            .flat_map(|(dpt, empls)| empls.iter().map(|e| &**e))
    }

    // get all employees with their department.
    pub fn get_all_dpt_empls(&self) -> impl Iterator<Item = (&str, &str)> {
        self.db
            .iter()
            .flat_map(|(dpt, empls)| empls.iter().map(|e| (&**dpt, &**e)))
    }

    // get employees of a particular department.
    pub fn get_empls(&self, dpt: &str) -> Box<dyn Iterator<Item = &str> + '_> {
        match self.db.get(dpt) {
            Some(empls) => Box::new(empls.iter().map(|e| &**e)),
            None => Box::new(std::iter::empty()),
        }
    }
}
