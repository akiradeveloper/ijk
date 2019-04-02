use std::cell::{RefCell, RefMut, Ref};
use std::rc::Rc;

pub trait AsRefMut<T>: Clone {
    fn borrow_mut(&self) -> RefMut<T>;
    fn map<U>(self, f: fn(&mut T) -> &mut U) -> Map<Self,T,U> where Self: Sized {
        Map::new(self, f)
    }
}

impl <T> AsRefMut<T> for Rc<RefCell<T>> {
    fn borrow_mut(&self) -> RefMut<T> {
        RefCell::borrow_mut(self)
    }
}

pub struct Map<S,T,U> {
    orig: S,
    f: fn(&mut T) -> &mut U,
}
impl <S,T,U> Map<S,T,U> where S: AsRefMut<T> {
    pub fn new(orig: S, f: fn(&mut T) -> &mut U) -> Self {
        Self { orig, f }
    }
}
impl <S,T,U> AsRefMut<U> for Map<S,T,U> where S: AsRefMut<T> {
    fn borrow_mut(&self) -> RefMut<U> {
        RefMut::map(self.orig.borrow_mut(), self.f)
    }
}
impl <S,T,U> Clone for Map<S,T,U> where S: AsRefMut<T> {
    fn clone(&self) -> Self {
        Map::new(self.orig.clone(), self.f)
    }
}

struct T {
    x: i32
}

#[test]
fn test_shared_mut() {
    let x = Rc::new(RefCell::new(T { x: 0 }));
    let y0 = x.clone().map(|t| &mut t.x);
    let y1 = x.clone().map(|t| &mut t.x);
    let y2 = y1.clone();
    *y2.borrow_mut() += 10;
    assert_eq!(x.borrow().x, 10);
    assert_eq!(*y0.borrow_mut(), 10);
    assert_eq!(*y1.borrow_mut(), 10);
    assert_eq!(*y2.borrow_mut(), 10);
}