use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable};
use super::queue::{InnerQueue, Promise};
///PLEASE FIND SOURCE JULIAN SO I CAN CREDIT AUTHOR


pub(super) trait TaskTrait {
    fn poll(&self, ctx: &mut Context) -> bool;
    fn increment_children(&self);
    fn decrement_children(&self);
    fn children(&self) -> i32;
    fn set_parent(&self, parent: Rc<dyn TaskTrait>);
    fn get_parent(&self) -> Option<Rc<dyn TaskTrait>>;
    fn is_parent_none(&self) -> bool;
    fn take_exception_store(&self) -> Option<Box<dyn Any>>;
    fn set_exception_store(&self, value: Option<Box<dyn Any>>);
    fn is_exception_ready(&self)->bool;
    fn get_children(&self) -> i32;
    fn set_children(&self, value: i32);
    fn is_head(&self) -> bool;
    fn set_finished(&self, value: bool) ;
    fn get_finished(&self)->bool ;
    fn set_head(&self, value: bool);
    fn was_killed(&self)->bool;
    fn destroy(&self,inner_queue: &InnerQueue, readd: bool);
    fn get_catch(&self)-> Option<Rc<dyn TaskTrait>>;
    fn set_catch(&self, value: Option<Rc<dyn TaskTrait>>);
    fn is_throwing(&self)->bool;
    fn set_is_throwing(&self, value:bool);
    fn set_started(&self, val: bool);
}

pub(super) struct Task<T>
{
    pub(super) is_head: Cell<bool>,
    pub(super) children: Cell<i32>,
    pub(super) future: RefCell<Pin<Box<dyn Future<Output=T>>>>,
    pub(super) promise: RefCell<Promise<T>>,
    pub(super) exception_store: Cell<Option<Box<dyn Any>>>,
    pub(super) parent: RefCell<Option<Rc<dyn TaskTrait>>>,
    pub(super) catch: Cell<Option<Rc<dyn TaskTrait>>>,
    pub(super) is_throwing: Cell<bool>,
    pub(super) finished: Cell<bool>,
}

impl<T> TaskTrait for Task<T>
{
    fn poll(&self, ctx: &mut Context) -> bool {
        self.promise.borrow().result.has_started.set(true);
        let ret = match self.future.borrow_mut().as_mut().poll(ctx) {
            Poll::Pending => {
                true
            }
            Poll::Ready(v) => {
                self.promise.borrow_mut().result.data.set(Some(v));
                false
            }
        };
        // println!("done polling");
        ret
    }
    fn increment_children(&self) {
        self.children.set(self.children.get()+1);
    }

    fn decrement_children(&self) {
        self.children.set(self.children.get()-1);
    }

    fn children(&self) -> i32 {
        self.children.get()
    }

    fn set_parent(&self, parent: Rc<dyn TaskTrait>) {
        *self.parent.borrow_mut() = Some(parent);
    }

    fn get_parent(&self) -> Option<Rc<dyn TaskTrait>> {
        self.parent.borrow().clone()
    }

    fn is_parent_none(&self) -> bool {
        self.parent.borrow().is_none()
    }

    fn take_exception_store(&self) -> Option<Box<dyn Any>> {
        self.exception_store.take()
    }

    fn set_exception_store(&self, value: Option<Box<dyn Any>>) {
        self.exception_store.set(value);
    }
    fn is_exception_ready(&self) -> bool {
        let l =self.exception_store.take();
        let ret = l.is_some();
        self.exception_store.set(l);
        return ret;
    }


    fn get_children(&self) -> i32 {
        self.children.get()
    }
    fn set_children(&self, value: i32) {
        self.children.set(value);
    }


    fn is_head(&self) -> bool {
        self.is_head.get()
    }


    fn set_finished(&self, value: bool) {
        self.promise.borrow().result.is_finished.set(value);
        self.finished.set(value) ;
    }

    fn get_finished(&self) -> bool {
        self.finished.get()
    }

    fn set_head(&self, value: bool) {
        self.is_head.set(value);
    }

    fn was_killed(&self) -> bool {
        self.promise.borrow().result.kill_flag.get()
    }

    fn destroy(&self, inner_queue: &InnerQueue, readd: bool){
        // println!("destruction") ;
        if let Some(parent) = self.get_parent().as_ref() {
            // println!("children{}",parent.get_children());
            parent.decrement_children();
            if parent.get_finished()&& parent.get_children()==0{
                // println!("destroying parent");
                parent.destroy(inner_queue,readd);
            }else if parent.children() == 0 && readd{
                // println!("readding");
                inner_queue.vec_deque.borrow_mut().push_back(parent.clone());
            }
            if(parent.get_children()<0){
                // println!("{}",parent.get_children()) ;
                panic!("illegal children")
            }
        }
    }

    fn get_catch(&self) -> Option<Rc<dyn TaskTrait>> {
        let catch = self.catch.take();
        let clone = catch.clone();
        self.catch.set(catch);
        return clone


    }

    fn set_catch(&self, value: Option<Rc<dyn TaskTrait>>) {
        self.catch.set(value);
    }

    fn is_throwing(&self) -> bool {
        self.is_throwing.get()
    }

    fn set_is_throwing(&self, value: bool) {
        self.is_throwing.set(value);
    }

    fn set_started(&self, val: bool){
        self.promise.borrow().result.has_started.set(val);
    }
}
