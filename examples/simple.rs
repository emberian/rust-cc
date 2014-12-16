#![feature(unsafe_destructor)]

extern crate cc;

use std::rc::Rc;
use std::cell::RefCell;
use List::{Void, Num, Pair};

type Blob = Rc<RefCell<List>>;


enum List {
    Void,
    Num(f64),
    Pair(Blob, Blob)
}

#[unsafe_destructor]
impl Drop for List {
    fn drop(&mut self) {
        match *self {
            Void => println!("dropping Void"),
            Num(f) => println!("dropping Num({})", f),
            Pair(..) => println!("dropping a pair!")
        }
    }
}

impl cc::CyclicReference for List {
    fn get_references(&self) -> cc::RefList {
        match *self {
            Pair(ref a, ref b) => Some(vec![box a.clone() as Box<cc::CyclicReference>,
                                            box b.clone() as Box<cc::CyclicReference>]),
            _ => Some(vec![]),
        }
    }

    fn break_references(&mut self) -> bool {
        *self = Void;
        true
    }

    fn get_id(&self) -> Option<uint> { None }
}

fn mk(a: List) -> Blob {
    Rc::new(RefCell::new(a))
}

fn main() {
    let a = mk(Num(32.));
    let b = mk(Void);
    let mut a = mk(Pair(a, b));
    match &mut *a.borrow_mut() {
        &Pair(ref mut h, ref mut t) => *t = a.clone(),
        _ => panic!()
    }

    // a, and the clone of a inside of itself will both be freed.
    assert_eq!(cc::collect(&mut a), Some(2));
}
