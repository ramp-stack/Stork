use std::rc::Rc;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;

pub struct Shared<T> {
    value:   Rc<RefCell<T>>,
    changed: Rc<Cell<bool>>,
}

impl<T> Shared<T> {
    pub fn new(val: T) -> Self {
        Self {
            value:   Rc::new(RefCell::new(val)),
            changed: Rc::new(Cell::new(false)),
        }
    }

    #[track_caller]
    pub fn get(&self) -> Ref<'_, T> {
        let loc = std::panic::Location::caller();
        match self.value.try_borrow() {
            Ok(r)  => r,
            Err(_) => panic!("Shared::get() called while a mut borrow is live.\n  caller: {loc}"),
        }
    }

    #[track_caller]
    pub fn get_mut(&self) -> RefMut<'_, T> {
        let loc = std::panic::Location::caller();
        match self.value.try_borrow_mut() {
            Ok(r)  => r,
            Err(_) => panic!("Shared::get_mut() called while a borrow is live.\n  caller: {loc}"),
        }
    }

    pub fn try_get_mut(&self) -> Option<RefMut<'_, T>> {
        self.value.try_borrow_mut().ok()
    }

    #[track_caller]
    pub fn set(&self, val: T) {
        let loc = std::panic::Location::caller();
        match self.value.try_borrow_mut() {
            Ok(mut v) => { *v = val; self.changed.set(true); }
            Err(_)    => panic!("Shared::set() called while a borrow is live.\n  caller: {loc}"),
        }
    }

    #[track_caller]
    pub fn update<F: FnOnce(&mut T)>(&self, f: F) {
        let loc = std::panic::Location::caller();
        match self.value.try_borrow_mut() {
            Ok(mut v) => f(&mut v),
            Err(_)    => panic!("Shared::update() called while a borrow is live.\n  caller: {loc}"),
        }
    }

    pub fn changed(&self) -> bool {
        self.changed.replace(false)
    }

    pub(crate) fn value_rc(&self) -> &Rc<RefCell<T>> {
        &self.value
    }

    pub(crate) fn changed_rc(&self) -> &Rc<Cell<bool>> {
        &self.changed
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            value:   Rc::clone(&self.value),
            changed: Rc::clone(&self.changed),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shared({:?})", self.value.borrow())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceSettings {
    strings: HashMap<String, String>,
    floats:  HashMap<String, f32>,
    usizes:  HashMap<String, usize>,
}

impl SourceSettings {
    pub fn parse(src: &str) -> Self {
        let mut out = Self::default();
        for raw in src.lines() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") { continue; }
            let Some(colon) = line.find(':') else { continue };
            let key  = line[..colon].trim();
            if key.is_empty() || key.contains(' ') || key.contains('"') { continue; }
            let rest = line[colon + 1..].trim();

            if rest.starts_with('"') {
                if let Some(end) = rest[1..].find('"') {
                    out.strings.insert(key.to_string(), rest[1..1 + end].to_string());
                    continue;
                }
            }
            let num = rest.trim_end_matches(',').trim();
            if !num.contains('.') {
                if let Ok(v) = num.parse::<usize>() {
                    out.usizes.insert(key.to_string(), v);
                    continue;
                }
            }
            if let Ok(v) = num.parse::<f32>() {
                out.floats.insert(key.to_string(), v);
            }
        }
        out
    }

    pub fn str(&self,   key: &str) -> Option<String> { self.strings.get(key).cloned() }
    pub fn f32(&self,   key: &str) -> Option<f32>    { self.floats.get(key).copied() }
    pub fn usize(&self, key: &str) -> Option<usize>  { self.usizes.get(key).copied() }
}

pub trait FromSource: Sized {
    fn from_source(p: &SourceSettings) -> Self;
}