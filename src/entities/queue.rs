use std::any;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt::{Display, Error, Formatter};
use std::future::Future;
use std::intrinsics::transmute;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use super::tasks::{TaskTrait, Task};






pub(super) struct InnerQueue {
    pub(super) vec_deque: RefCell<VecDeque<Rc<dyn TaskTrait>>>,
    exception_ready: Cell<bool>,
    current: RefCell<Option<Rc<dyn TaskTrait>>>,
    last_resort_exception_container: Cell<Option<Box<dyn Any>>>
    // tree_counter: Cell<u64>
}


impl InnerQueue {
    fn new() -> Self {
        InnerQueue {
            vec_deque: RefCell::new(VecDeque::new()),
            current: RefCell::new(None),
            exception_ready: Cell::new(false),
            last_resort_exception_container: Cell::new(None)
        }
    }
    pub fn pop(&self)->Option<Rc<dyn TaskTrait>>{
        self.vec_deque.borrow_mut().pop_front()
    }
    fn run(self: Rc<Self>)-> Option<Box<dyn Any>>

    {
        let rw = dummy_raw_waker();
        let w = unsafe { Waker::from_raw(rw) };
        let mut ctx = Context::from_waker(&w);

        while let Some(mut state) =  self.pop(){
            // println!("left {}",self.vec_deque.borrow().len());
            assert_eq!(0,state.get_children());
            if(0>state.get_children()){
                // println!("{}",state.get_children());
                panic!("bro")
            }
            if state.was_killed(){
                todo!("ensure that killing actually works");
            }
            let   catch= &state.get_catch();
            if catch.is_some() && catch.as_ref().unwrap().is_throwing(){
                // println!("skipping skipped caught");
                state.set_started(true);
                state.destroy(&self,false);
                let catch_parent = state.get_catch().unwrap().get_parent().unwrap();
                if catch_parent.get_children()==0{
                    // println!("adding back");
                    self.vec_deque.borrow_mut().push_back(catch_parent);
                }
                continue;
            }
            if state.is_throwing(){
                panic!("this shouldn't happen now");
            }
            *self.current.borrow_mut() = Some(state.clone());
            let  finished_function= !state.poll(&mut ctx);
            state.set_finished(finished_function);
            if self.exception_ready.get() {
                self.exception_ready.set(false);
                // println!("exception caught");
                if let None = state.get_catch() {
                    // println!("last resort");
                    return self.last_resort_exception_container.take();
                }
                state.destroy(&self,false);
                let catch_parent = state.get_catch().unwrap().get_parent().unwrap();
                if catch_parent.get_children()==0{
                    // println!("adding back");
                    self.vec_deque.borrow_mut().push_back(catch_parent);
                }
                continue;
            }
            if !finished_function && state.get_children()==0{
                // println!("readding because of empty join");
                self.vec_deque.borrow_mut().push_back(state);
            }else if finished_function && state.get_children()==0{
                // println!("finished function");
                state.destroy(&self,true);
            }
        }

        return None
    }

}

///I GOT THIS FUNCTION ONLINE. BE SURE TO CREDIT
pub(super) fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)

}
pub struct Queue{
    inner_queue: Rc<InnerQueue>
}
impl Clone for Queue{
    fn clone(&self) -> Self {
        Queue{
            inner_queue: self.inner_queue.clone()
        }
    }
}
impl Queue{
    pub fn new()->Self{
        Queue{
            inner_queue: Rc::new(InnerQueue::new())
        }
    }
    pub fn run<O: 'static, F>(&mut self, start: F) -> ExceptionPromise<O>
    where
        F: Future<Output=O>,

    {
        let final_answer = add_task(start, &self.clone(),None,None,true);
        let potential_catch = self.inner_queue.clone().run();
        ExceptionPromise {
            promise: final_answer,
            current: CurrentOrValue::Value(potential_catch)
        }


    }
}
pub enum FunctionState{
    Join,
    Done,
    Throwing,
    Panic
}
impl Future for FunctionState
{
    type Output = FunctionState;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let mut mutself = Pin::get_unchecked_mut(self);
            match mutself{
                FunctionState::Join => {
                    *mutself =  FunctionState::Done;
                    Poll::Pending
                },
                FunctionState::Done => {
                    Poll::Ready(FunctionState::Done)
                },
                FunctionState::Throwing => {
                    *mutself = FunctionState::Panic;
                    Poll::Pending
                },
                FunctionState::Panic=>{
                    panic!("polled after throwing");
                }
            }
        }
    }
}

macro_rules! join {
    () => {
        $crate::entities::FunctionState::Join.await
    };
}


pub async fn throw_internal<T: 'static>(data:T,  queue: &Queue){
    let data: Box<dyn Any>= Box::new(data);
    match queue.inner_queue.current.borrow().as_ref().unwrap().get_catch() {
        None => {
            // println!("setting last resort");
            queue.inner_queue.last_resort_exception_container.set(Some(data));

        }
        Some(catch) => {
            // println!("setting catch");
            // println!("catch children {}",catch.get_children());
            catch.set_exception_store(Some(data));
            catch.set_is_throwing(true);
        }
    };

    queue.inner_queue.exception_ready.set(true);
    FunctionState::Throwing.await;
}

#[macro_export]
macro_rules! throw {
    ($data:expr => $queue:expr) => {
        $crate::entities::throw_internal($data,$queue).await;
        panic!("returned to paniced code");
    };
}
// macro_rules! throw_exception {
//     ($data:expr => $queue:expr) => {
//         $crate::entities::throw_internal($data,$queue).await
//     };
// }

pub use throw;
// pub use throw_exception;

#[derive(Default)]
pub(super) struct PromiseInner<T>{
    pub(super) data: Cell<Option<T>>,
    pub(super) kill_flag: Cell<bool>,
    pub(super) has_started: Cell<bool>,
    pub(super) is_finished: Cell<bool>
}
pub struct Promise<T> {
    pub(super) result: Rc<PromiseInner<T>>
}
impl<T> Promise<T>{
    pub fn cancel(self) -> Option<T>{
        self.result.kill_flag.set(true);
        self.result.data.take()
    }
}
impl<T> Clone for Promise<T> {
    fn clone(&self) -> Self {
        Promise {
            result: self.result.clone()
        }
    }
}

impl<T> Promise<T> {
    pub(crate) fn new() -> Self {
        Promise {
            result: Rc::new(PromiseInner{data: Cell::new(None), kill_flag: Cell::new(false),
                has_started: Cell::new(false),
                is_finished: Cell::new(false)
            })
        }
    }
    pub fn unwrap(self) -> T {
        assert!(self.result.has_started.get() && self.result.is_finished.get());
        self.result.data.take().expect("Tried to unwrap before joining")
    }

}
enum CurrentOrValue{
    Current(Rc<dyn TaskTrait>),
    Value(Option<Box<dyn Any>>)
}
impl CurrentOrValue{
    fn take_exception(self)->Option<Box<dyn Any>>{
        match self {
            CurrentOrValue::Current(curr) =>  {
                curr.take_exception_store()
            }
            CurrentOrValue::Value(val) => {
                val
            }
        }
    }
}
pub struct ExceptionPromise<T> {
    pub(crate) promise: Promise<T>,
    pub(crate) current: CurrentOrValue
}

pub struct UnwrappedExceptionPromise<T>{
    exception_promise: ExceptionPromise<T>
}
impl<T> UnwrappedExceptionPromise<T>{
    pub fn consume_any(self)->ResultMatch<T,Box<dyn Any>>{

        let exception_promise = self.exception_promise;
        let promise = exception_promise.promise;


        let exception = exception_promise.current.take_exception();
        if !promise.result.is_finished.get()&&exception.is_none() {
            panic!("this shouldn't ever happen")
        }
        match exception {
            None => {
                return ResultMatch::Ok(promise.result.data.take().unwrap());
            }
            Some(o) => {
                return ResultMatch::Exception(o);
            }
        }
    }
    pub fn unwrap_ok(self)->T{
        match self.consume_any() {
            ResultMatch::Ok(o) => {
                o
            }
            ResultMatch::Exception(_) => {
                panic!("got exception when tried to unwrap ok")
            }
        }
    }
    pub fn consume_type<E: 'static>(mut self) ->Result<ResultMatch<T,E>,MatchTypeException>{
        let val = self.consume_any();
        match val {
            ResultMatch::Ok(val) => {
                return Ok(ResultMatch::Ok(val))
            }
            ResultMatch::Exception(e) => {
                let e = e.downcast::<E>();
                match e {
                    Ok(v) => {
                        return Ok(ResultMatch::Exception(*v));
                    }
                    Err(_)=>{
                        return Err(MatchTypeException{
                            expected: any::type_name::<E>(),
                        });
                    }
                }
            }
        }
    }
}
pub enum ResultMatch<T,E>{
    Ok(T),
    Exception(E),
}
#[derive(Debug,Clone)]
pub struct MatchTypeException{
    pub(crate) expected : &'static str,
}
impl std::error::Error for MatchTypeException{
}
impl Display for MatchTypeException{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"failed to cast to {}",self.expected)
    }
}

impl<T> ExceptionPromise<T>{
    pub fn unwrap(self)->UnwrappedExceptionPromise<T>{
        if !self.promise.result.has_started.get(){
            panic!("Tried to unwrap before joining")
        }
        UnwrappedExceptionPromise{
            exception_promise: self
        }
    }
}
pub fn catch<O,T:Future<Output = O>>(future: T, queue: &Queue) -> ExceptionPromise<O>{

    let queue = queue.clone();

    let current = queue.inner_queue.current.borrow();

    let mut promise = Promise::new();

    let boxed = Box::pin(future) as Pin<Box<dyn Future<Output=O>>>;
    let interpretation: Pin<Box<dyn Future<Output=O> + 'static>> = unsafe {transmute(boxed) };

    let unwrapped_current = current.as_ref().unwrap();
    // unwrapped_current.se
    // current
    let task = Task {
        is_head: Cell::new(false),
        future: RefCell::new(interpretation),
        promise: RefCell::new(promise.clone()),
        children: Cell::new(0),
        parent: RefCell::new(current.clone()),
        catch: Cell::new(None),
        is_throwing: Cell::new(false),
        exception_store: Cell::new(None),
        finished: Cell::new(false),
    };

    let task: Rc<dyn TaskTrait> =Rc::new(task);
    let task: Rc<dyn TaskTrait> =unsafe{
        transmute(task)
    };
    task.set_catch(Some(task.clone()));
    let mut queue_result: ExceptionPromise<O> = ExceptionPromise {
        promise: promise.clone(),
        current: CurrentOrValue::Current(task.clone())
    };

    queue.inner_queue.vec_deque.borrow_mut().push_back(task);

    current.clone().unwrap().increment_children();
    queue_result
}
type TaskRef = Rc<dyn TaskTrait>;


///DOES NOT INCREASE PARENT. THIS IS DONE IN ADD
fn add_task<O, T>(future: T, queue: &Queue,parent: Option<TaskRef>, catch: Option<TaskRef>, is_head: bool) -> Promise<O>
where
    T: Future<Output=O>
{

    let queue = queue.clone();
    let mut p = Promise::new();
    let pclone = p.clone();

    let boxed = Box::pin(future) as Pin<Box<dyn Future<Output=O>>>;
    let interpretation: Pin<Box<dyn Future<Output=O> + 'static>> = unsafe {transmute(boxed) };


    let task = Task {
        is_head: Cell::new(is_head),
        future: RefCell::new(interpretation),
        promise: RefCell::new(p),
        children: Cell::new(0),
        parent: RefCell::new(parent),
        catch: Cell::new(catch),
        exception_store: Cell::new(None),
        is_throwing: Cell::new(false),
        finished: Cell::new(false),
    };
    let task: Rc<dyn TaskTrait> =Rc::new(task);

    //extends lifetime
    let task =unsafe{
        transmute(task)
    };
    queue.inner_queue.vec_deque.borrow_mut().push_back(task);
    pclone
}
pub fn add<O, T>(future: T, queue: &Queue) -> Promise<O>
where
    T: Future<Output=O>,
{
    let current = queue.inner_queue.current.borrow();
    let unwrapped = current.as_ref().unwrap();
    unwrapped.increment_children();
    return add_task(future, queue,current.clone(),unwrapped.get_catch(),false);
}
