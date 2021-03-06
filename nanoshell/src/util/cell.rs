use std::cell::{Ref, RefCell, RefMut};

// RefCell implementation that supports late initialization and can only be set once;
// Panics if data is accessed before set has been called or if set is called more than once.
#[derive(Clone)]
pub struct LateRefCell<T> {
    value: RefCell<Option<T>>,
}

impl<T> LateRefCell<T> {
    pub fn new() -> Self {
        Self {
            value: RefCell::new(None),
        }
    }

    pub fn set(&self, value: T) {
        let mut v = self.value.borrow_mut();
        match v.as_ref() {
            Some(_) => {
                panic!("Value already set")
            }
            None => *v = Some(value),
        }
    }

    pub fn is_set(&self) -> bool {
        self.value.borrow().is_some()
    }

    pub fn clone_value(&self) -> T
    where
        T: Clone,
    {
        let value = self.value.borrow();
        match &*value {
            Some(value) => value.clone(),
            None => {
                panic!("Value has not been set yet");
            }
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        Ref::map(self.value.borrow(), |t| t.as_ref().unwrap())
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        RefMut::map(self.value.borrow_mut(), |t| t.as_mut().unwrap())
    }
}
