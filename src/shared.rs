use std::cell::{RefCell, RefMut, Ref};
use std::rc::Rc;

pub trait SharedMut<T> {
    fn borrow_mut(&self) -> RefMut<T>;
}

impl <T> SharedMut<T> for Rc<RefCell<T>> {
    fn borrow_mut(&self) -> RefMut<T> {
        RefCell::borrow_mut(self)
    }
}

pub struct Mapped<S,T,U> {
    orig: S,
    f: fn(&mut T) -> &mut U,
}
impl <S,T,U> Mapped<S,T,U> where S: SharedMut<T> {
    pub fn new(orig: S, f: fn(&mut T) -> &mut U) -> Self {
        Self { orig, f }
    }
}
impl <S,T,U> SharedMut<U> for Mapped<S,T,U> where S: SharedMut<T> {
    fn borrow_mut(&self) -> RefMut<U> {
        RefMut::map(self.orig.borrow_mut(), self.f)
    }
}

struct T {
    x: i32
}

#[test]
fn test_shared_mut() {
    let x = Rc::new(RefCell::new(T { x: 0 }));
    let y0 = Mapped::new(x.clone(), |t| &mut t.x);
    let y1 = Mapped::new(x.clone(), |t| &mut t.x);
    *y1.borrow_mut() += 10;
    assert_eq!(*y0.borrow_mut(), 10);
    assert_eq!(*y1.borrow_mut(), 10);
}