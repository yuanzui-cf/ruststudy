use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::Value,
    error::{self, Result},
};

#[derive(Debug)]
pub struct Environment {
    store: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Rc<RefCell<Environment>> {
        Rc::new(RefCell::new(Environment {
            store: HashMap::new(),
            parent: None,
        }))
    }

    pub fn new_child(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Environment>> {
        Rc::new(RefCell::new(Environment {
            store: HashMap::new(),
            parent: Some(parent),
        }))
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.store.get(name) {
            Some(val.clone())
        } else if let Some(parent) = &self.parent {
            let parent = parent.borrow();
            parent.get(name)
        } else {
            None
        }
    }

    pub fn define(&mut self, name: &str, val: Option<Value>) -> Result<()> {
        if self.store.contains_key(name) {
            return Err(error::error!(Name, "name '{name}' is already defined"));
        } else {
            self.store.insert(
                name.into(),
                if let Some(val) = val {
                    val
                } else {
                    Value::None
                },
            );
        }
        Ok(())
    }

    pub fn assign(&mut self, name: &str, val: Value) -> Result<()> {
        if self.store.contains_key(name) {
            self.store.insert(name.into(), val);
        } else if let Some(parent) = &mut self.parent {
            let mut parent = parent.borrow_mut();
            parent.assign(name, val)?;
        } else {
            return Err(error::error!(Name, "name {name} is not defined"));
        }

        Ok(())
    }
}
