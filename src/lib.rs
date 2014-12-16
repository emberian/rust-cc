use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};

pub type RefList = Option<Vec<Box<CyclicReference+'static>>>;

/// A possibly-cyclic reference that should be considered during cycle collection.
///
/// To break a cycle, `break_references` will be used. To walk the object graph, `get_references`
/// will be used.
pub trait CyclicReference {
    /// Break any nested references this reference might contain, to remove it from a cycle.
    ///
    /// Returns `true` if any possible references this held were broken, `false` if no progress was
    /// made (due to not being able to acquire a RefCell or RWLock, for example).
    fn break_references(&mut self) -> bool;

    /// Return any references referenced by this reference.
    fn get_references(&self) -> RefList;

    /// Get the id of this reference, used to determine whether this object has been seen
    /// before.
    ///
    /// It is intended that smart pointers will return their address here. If there is no useful id
    /// to return for a given implementation of CyclicReference, return None. Odds are, some
    /// wrapper type higher up has you covered (eg, `Rc<RefCell<R>>`)
    fn get_id(&self) -> Option<uint>;
}

impl<R: CyclicReference> CyclicReference for Rc<RefCell<R>> {
    fn break_references(&mut self) -> bool {
        self.try_borrow_mut().map(|mut r| r.break_references()).unwrap_or(false)
    }

    fn get_references(&self) -> RefList {
        self.try_borrow().as_ref().and_then(|r| r.get_references())
    }

    fn get_id(&self) -> Option<uint> { Some(&*self as *const _ as uint) }
}

impl<'a, R: CyclicReference> CyclicReference for RefMut<'a, R> {
    fn break_references(&mut self) -> bool { (**self).break_references() }
    fn get_references(&self) -> RefList { (**self).get_references() }
    fn get_id(&self) -> Option<uint> { (**self).get_id() }
}

impl<'a, R: CyclicReference> CyclicReference for &'a mut R {
    fn break_references(&mut self) -> bool { (**self).break_references() }
    fn get_references(&self) -> RefList { (**self).get_references() }
    fn get_id(&self) -> Option<uint> { (**self).get_id() }
}

impl<'a, R: CyclicReference> CyclicReference for Ref<'a, R> {
    fn break_references(&mut self) -> bool { false }
    fn get_references(&self) -> RefList { (**self).get_references() }
    fn get_id(&self) -> Option<uint> { (**self).get_id() }
}

impl<'a, R: CyclicReference> CyclicReference for &'a R {
    fn break_references(&mut self) -> bool { false }
    fn get_references(&self) -> RefList { (**self).get_references() }
    fn get_id(&self) -> Option<uint> { (**self).get_id() }
}

impl<R: CyclicReference> CyclicReference for Option<R> {
    fn break_references(&mut self) -> bool { *self = None; true }
    fn get_references(&self) -> RefList { self.as_ref().and_then(|r| r.get_references()) }
    fn get_id(&self) -> Option<uint> { self.as_ref().and_then(|r| r.get_id()) }
}

/// Run a cycle collection, starting at `reference`, returning the number of objects collected, or
/// None if `reference` returned `None` from either `get_id` or `get_references`.
///
/// This will do a depth-first search on the object graph as seen by `R::get_references`. As it
/// walks the graph, it records the ids of objects it has already seen, via `R::get_id`. If it sees
/// an object it has already seen, it will call `break_references` on that reference.
///
/// For this to be effective with, for example, `Rc`, the system using this to perform cycle
/// collection should store a list of weak pointers to every object in the system, and periodically
/// remove weak pointers to destroyed objects. Any remaining pointers in the list would then be a
/// good candidate for cycle collection.
pub fn collect<R>(reference: &mut R) -> Option<u32> where R : CyclicReference {
    // reconsider this choice of set after profiling
    use std::collections::BTreeSet;

    let mut seen = BTreeSet::new();
    let mut to_visit = Vec::new();
    let mut broken = 0;

    seen.insert(match reference.get_id() {
        None => return None,
        Some(id) => id
    });

    match reference.get_references() {
        None => return None,
        Some(refs) => to_visit.extend(refs.into_iter())
    }

    while !to_visit.is_empty() {
        let mut refe = to_visit.pop().expect("to_visit was empty but we just checked it wasn't!?");
        if seen.insert(match refe.get_id() {
            None => continue,
            Some(id) => id
        }) {
            if refe.break_references() { broken += 1 }
            match refe.get_references() {
                None => continue,
                Some(refs) => to_visit.extend(refs.into_iter())
            }
        }
    }

    Some(broken)
}
