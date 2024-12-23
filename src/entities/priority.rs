use std::any;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::intrinsics::transmute;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};
use thiserror::Error;
use crate::entities::{FunctionState};
use crate::entities::queue::{dummy_raw_waker, ExceptionPromise, InnerQueue,Promise};
use super::tasks::{Task, TaskTrait};
#[derive(Debug,Clone)]
pub struct MatchTypeException{
    pub(crate) expected : &'static str,
}


impl  std::error::Error for MatchTypeException {
}
impl Display for MatchTypeException {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"failed to cast to {}",self.expected)
    }
}
pub enum ResultMatch<T,E>{
    Ok(T),
    Exception(E),
}
#[derive(Clone)]
struct Contract<P: Ord>{
    priority_task: Weak<dyn PriorityTaskTrait<P>>
}
#[derive(Default)]
pub enum Priority<P: Ord>{
    Value(P),
    #[default]
    Now
}
impl<P: Ord + Copy>  Copy for Priority<P>{
}
impl<P: Ord + Clone> Clone for Priority<P>{
    fn clone(&self) -> Self {
        match &self {
            Value(v) => {
                Priority::Value(v.clone())
            }
            Priority::Now => {
                Priority::Now
            }
        }

    }
}
impl<P: Ord> Priority<P>{
    pub fn is_now(&self)-> bool{
        match &self {
            Value(_) => {false}
            Priority::Now => {true}
        }
    }
    pub fn is_value(&self)-> bool{
        !self.is_now()
    }
    pub fn unwrap(self)->P{
        match self {
            Priority::Value(v) => {
                v
            }
            Priority::Now => {
                panic!("unwrapped priority but got now")
            }
        }
    }
    pub fn to_option(self)->Option<P>{
        match self {
            Priority::Value(v) => {
                Some(v)
            }
            Priority::Now => {
                None
            }
        }
    }
}

impl<P: Ord> Eq for Priority<P> {}

impl<P: Ord> PartialEq<Self> for Priority<P> {
    fn eq(&self, other: &Self) -> bool {
        match &self {
            Priority::Value(v1) => {
                match other {
                    Priority::Value(v2) => {
                        v1 == v2
                    }
                    Priority::Now => {
                        todo!("this shouldn't be running but later on delete this")
                    }
                }
            }
            Priority::Now => {
                todo!("this shouldn't be running but later on delete this")
            }
        }
    }
}

impl<P: Ord> PartialOrd<Self> for Priority<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match &self {
            Priority::Value(v1) => {
                match other {
                    Priority::Value(v2) => {
                        v1.partial_cmp(v2)
                    }
                    Priority::Now => {
                        todo!("this shouldn't be happening in my code rn, delete this line please");
                        Some(Ordering::Less)
                    }
                }
            }
            Priority::Now => {
                match other {
                    Priority::Value(v2) => {
                        todo!("this shouldn't be happening in my code rn, delete this line please");
                        Some(Ordering::Greater)
                    }
                    Priority::Now => {
                        todo!("this shouldn't be happening in my code rn, delete this line please");
                        Some(Ordering::Equal)
                    }
                }
            }
        }
    }
}

impl<P: Ord> Ord for Priority<P>{
    fn cmp(&self, other: &Self) -> Ordering {
        match &self {
            Priority::Value(v1) => {
                match other {
                    Priority::Value(v2) => {
                        v1.cmp(v2)
                    }
                    Priority::Now => {
                        todo!("delete this later")
                    }
                }
            }
            Priority::Now => {
                todo!("delete this later")
            }
        }
    }
}
#[derive(Error,Debug)]
pub enum ContractError{
    #[error("Tried to set priority of finished function")]
    FinishedFunction
}
impl<P: Ord> Contract<P>{
    pub fn set_priority(&self,priority: Priority<P>)-> Result<(),ContractError>{
        let unwrapped_task = self.priority_task.upgrade().ok_or(ContractError::FinishedFunction)?;
        if unwrapped_task.get_finished() {
            return Err(ContractError::FinishedFunction)
        }
        unwrapped_task.set_value(priority);
        Ok(())
    }
    pub fn get_priority(&self)-> Result<Priority<P>,ContractError>
    where
        P: Clone + Copy
    {
        let unwrapped_task = self.priority_task.upgrade().ok_or(ContractError::FinishedFunction)?;
        Ok(unwrapped_task.get_value())
    }
    pub fn clone_priority(&self)-> Result<Priority<P>,ContractError>
    where
        P: Clone
    {
        let unwrapped_task = self.priority_task.upgrade().ok_or(ContractError::FinishedFunction)?;
        Ok(unwrapped_task.clone_value())
    }
}
struct PriorityTask<P: Ord,T>
{
    value: Cell<Priority<P>>,
    pub(super) is_head: Cell<bool>,
    pub(super) children_count: Cell<i32>,
    pub(super) future: RefCell<Pin<Box<dyn Future<Output=T>>>>,
    pub(super) promise: RefCell<Promise<T>>,
    pub(super) exception_store: Cell<Option<Box<dyn Any>>>,
    pub(super) parent: RefCell<Option<Rc<dyn PriorityTaskTrait<P>>>>,
    pub(super) catch: Cell<Option<Rc<dyn PriorityTaskTrait<P>>>>,
    pub(super) is_throwing: Cell<bool>,
    pub(super) finished: Cell<bool>,
    pub(super) is_select: Cell<bool>,
    pub(super) is_select_head: Cell<bool>,
    pub(super) children: RefCell<Option<Vec<Weak<dyn PriorityTaskTrait<P>>>>>,
    pub(super) select_count: Cell<i32>,
    pub(super) killed: Cell<bool>

}
trait PriorityTaskTrait<P: Ord>{
    fn set_value(&self, value: Priority<P>);
    fn poll(&self, ctx: &mut Context) -> bool;
    fn increment_children(&self);
    fn decrement_children(&self);
    fn children(&self) -> i32;
    fn set_parent(&self, parent: Rc<dyn PriorityTaskTrait<P>>);
    fn get_parent(&self) -> Option<Rc<dyn PriorityTaskTrait<P>>>;
    fn is_parent_none(&self) -> bool;
    fn take_exception_store(&self) -> Option<Box<dyn Any>>;
    fn set_exception_store(&self, value: Option<Box<dyn Any>>);
    fn is_exception_ready(&self)->bool;
    fn get_children_count(&self) -> i32;
    fn get_children(&self) -> &RefCell<Option<Vec<Weak<dyn PriorityTaskTrait<P>>>>>;
    fn set_children_count(&self, value: i32);
    fn is_head(&self) -> bool;
    fn is_select(&self) -> bool;
    fn get_select_count(&self) -> i32;
    fn set_select_count(&self, value: i32);
    fn set_finished(&self, value: bool) ;
    fn get_finished(&self)->bool ;
    fn set_head(&self, value: bool);
    fn was_killed(&self)->bool;
    fn set_killed(&self, value: bool);
    fn destroy(&self,inner_queue: &InnerPriorityQueue<P>, readd: bool);
    fn get_catch(&self)-> Option<Rc<dyn PriorityTaskTrait<P>>>;
    fn set_catch(&self, value: Option<Rc<dyn PriorityTaskTrait<P>>>);
    fn is_throwing(&self)->bool;
    fn set_is_throwing(&self, value:bool);
    fn set_started(&self, val: bool);
    fn take_value(&self)-> Priority<P>;
    fn get_value(&self)-> Priority<P> where P: Clone + Copy;
    fn clone_value(&self)-> Priority<P> where P: Clone;
    fn is_now(&self) ->bool;
}

impl<P: Ord> Eq for dyn PriorityTaskTrait<P> {}

impl<P: Ord> PartialEq<Self> for dyn PriorityTaskTrait<P> {
    fn eq(&self, other: &Self) -> bool {

        let a = self.take_value().unwrap();
        let b = other.take_value().unwrap();

        let ret = a==b;
        self.set_value(Value(a));
        other.set_value(Value(b));
        return ret;
    }
}

impl<P:Ord> PartialOrd<Self> for dyn PriorityTaskTrait<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.take_value().unwrap();
        let b = other.take_value().unwrap();

        let ret = a.partial_cmp(&b);
        self.set_value(Value(a));
        other.set_value(Value(b));
        return ret;
    }
}

impl<P:Ord> Ord for dyn PriorityTaskTrait<P> {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.take_value().unwrap();
        let b = other.take_value().unwrap();

        let ret = a.cmp(&b);
        self.set_value(Value(a));
        other.set_value(Value(b));
        return ret;
    }
}

impl<P:Ord, T> PriorityTaskTrait<P> for PriorityTask<P,T>{
    fn set_value(&self, value: Priority<P>) {
        self.value.set(value);
    }
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
        self.children_count.set(self.children_count.get()+1);
    }

    fn decrement_children(&self) {
        self.children_count.set(self.children_count.get()-1);
    }

    fn children(&self) -> i32 {
        self.children_count.get()
    }

    fn set_parent(&self, parent: Rc<dyn PriorityTaskTrait<P>>) {
        *self.parent.borrow_mut() = Some(parent);
    }

    fn get_parent(&self) -> Option<Rc<dyn PriorityTaskTrait<P>>> {
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


    fn get_children_count(&self) -> i32 {
        self.children_count.get()
    }

    fn get_children(&self) -> &RefCell<Option<Vec<Weak<dyn PriorityTaskTrait<P>>>>> {
        &self.children
    }

    fn set_children_count(&self, value: i32) {
        self.children_count.set(value);
    }


    fn is_head(&self) -> bool {
        self.is_head.get()
    }

    fn is_select(&self) -> bool {
        self.is_select.get()
    }

    fn get_select_count(&self) -> i32 {
        self.select_count.get()
    }

    fn set_select_count(&self, value: i32) {
        self.select_count.set(value)
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
       self.killed.get()
    }

    fn set_killed(&self, value: bool) {
        self.killed.set(value)
    }

    fn destroy(&self, inner_queue: &InnerPriorityQueue<P>, readd: bool){
        if let Some(parent) = self.get_parent().as_ref() {
            if  self.is_select_head.get(){
                parent.set_select_count(parent.get_select_count()-1);
            }else{
                parent.decrement_children();
            }
            if parent.get_finished()&& parent.get_children_count()==0 && parent.get_children_count()==0{
                parent.destroy(inner_queue,readd);
            }else if parent.children() == 0 && readd && parent.get_select_count() ==0{
                inner_queue.push(parent.clone());
            }
            
            if(parent.get_children_count()<0){
                println!("{}",parent.get_children_count()) ;
                panic!("illegal children")
            }
        }
    }

    fn get_catch(&self) -> Option<Rc<dyn PriorityTaskTrait<P>>> {
        let catch = self.catch.take();
        let clone = catch.clone();
        self.catch.set(catch);
        return clone


    }

    fn set_catch(&self, value: Option<Rc<dyn PriorityTaskTrait<P>>>) {
        self.catch.set(value);
    }

    fn is_throwing(&self) -> bool {
        self.is_throwing.get()
    }

    fn set_is_throwing(&self, value: bool) {
        self.is_throwing.set(value);
    }

    fn set_started(&self, val: bool) {
        self.promise.borrow().result.has_started.set(val);
    }

    fn take_value(&self) -> Priority<P> {
        self.value.take()
    }

    fn get_value(&self) -> Priority<P>
        where P: Clone + Copy
    {
        self.value.get()
    }
    fn is_now(&self) ->bool{
        let val = self.value.take();
        let  b = val.is_now();
        self.value.set(val);
        return b

    }

    fn clone_value(&self) -> Priority<P>
    where
        P: Clone
    {
        let value = self.value.take();
        self.value.set(value.clone());
        value
    }
}

enum CurrentOrValue<P: Ord>{
    Current(Rc<dyn PriorityTaskTrait<P>>),
    Value(Option<Box<dyn Any>>)
}
impl<P: Ord> CurrentOrValue<P> {
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
pub struct PriorityExceptionPromise<T, P: Ord> {
    pub(crate) promise: Promise<T>,
    pub(crate) current: CurrentOrValue<P>
}
pub struct PriorityExceptionSelect<T, P: Ord> {
    pub(crate) promise: Promise<T>,
    pub(crate) current: CurrentOrValue<P>
}

pub struct UnwrappedPriorityExceptionPromise<T, P: Ord>{
    exception_promise:PriorityExceptionPromise<T, P>
}
pub struct UnwrappedPriorityExceptionSelect<T,P:Ord>{
    priority_exception_select: PriorityExceptionSelect<T,P>
}
pub enum SelectMatch<T,E>{
    Ok(T),
    Exception(E),
    Interrupted
}
impl<T, P: Ord> UnwrappedPriorityExceptionSelect<T,P>{

    pub fn consume_any(self) -> SelectMatch<T, Box<dyn Any>> {

        let exception_promise = self.priority_exception_select;
        let promise = exception_promise.promise;


        let exception = exception_promise.current.take_exception();
        if !promise.result.is_finished.get()&&exception.is_none() {
            return SelectMatch::Interrupted
        }
        match exception {
            None => {
                return SelectMatch::Ok(promise.result.data.take().unwrap());
            }
            Some(o) => {
                return SelectMatch::Exception(o);
            }
        }
    }
    pub fn unwrap_ok(self)->T{
        match self.consume_any() {
            SelectMatch::Ok(o) => {
                o
            }
            SelectMatch::Exception(_) => {
                panic!("got exception when tried to unwrap ok")
            }
            SelectMatch::Interrupted=>{
                panic!("got interrupted when tried to unwrap ok");
            }
        }
    }
    pub fn consume_type<E: 'static>(mut self) ->Result<SelectMatch<T, E>,MatchTypeException>{
        let val = self.consume_any();
        match val {
            SelectMatch::Ok(val) => {
                return Ok(SelectMatch::Ok(val))
            }
            SelectMatch::Exception(e) => {
                let e = e.downcast::<E>();
                match e {
                    Ok(v) => {
                        return Ok(SelectMatch::Exception(*v));
                    }
                    Err(_)=>{
                        return Err(MatchTypeException {
                            expected: any::type_name::<E>(),
                        });
                    }
                }
            }
            SelectMatch::Interrupted=>{
                return Ok(SelectMatch::Interrupted)
            }
        }
    }
}
impl<T,P: Ord> UnwrappedPriorityExceptionPromise<T,P> {
    pub fn consume_any(self)->ResultMatch<T, Box<dyn Any>> {

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
    pub fn consume_type<E: 'static>(mut self) ->Result<ResultMatch<T, E>, crate::entities::queue::MatchTypeException>{
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
                        return Err(crate::entities::queue::MatchTypeException {
                            expected: any::type_name::<E>(),
                        });
                    }
                }
            }
        }
    }
}
impl<T, P: Ord> PriorityExceptionSelect<T, P> {

    pub fn unwrap(self)->UnwrappedPriorityExceptionSelect<T, P> {
        if !self.promise.result.has_started.get(){
            panic!("Tried to unwrap before joining")
        }
        UnwrappedPriorityExceptionSelect{
            priority_exception_select: self
        }
    }
}
impl<T, P: Ord> PriorityExceptionPromise<T, P> {

    pub fn unwrap(self)->UnwrappedPriorityExceptionPromise<T, P> {
        if !self.promise.result.has_started.get(){
            panic!("Tried to unwrap before joining")
        }
        UnwrappedPriorityExceptionPromise{
            exception_promise: self
        }
    }
}
struct InnerPriorityQueue<P: Ord>{
    vec_deque: RefCell<Heap<Rc<dyn PriorityTaskTrait<P>>>>,
    exception_ready: Cell<bool>,
    current: RefCell<Option<Rc<dyn PriorityTaskTrait<P>>>>,
    last_resort_exception_container: Cell<Option<Box<dyn Any>>>,
}

impl<P: Ord>  InnerPriorityQueue<P>{
    fn new(priority_style: PriorityStyle)->InnerPriorityQueue<P>{
        let is_max = match priority_style {
            PriorityStyle::Min => {
                false
            }
            PriorityStyle::Max => {
                true
            }
            PriorityStyle::Queue => {
               false
            }
        };
        let is_queue = match priority_style {
            PriorityStyle::Queue => true,
            _ => false
        };

        InnerPriorityQueue{
            vec_deque: RefCell::new(Heap::new(is_max,is_queue)),
            exception_ready: Cell::new(false),
            current: RefCell::new(None),
            last_resort_exception_container: Cell::new(None),
        }
    }
    fn push(&self, data: Rc<dyn PriorityTaskTrait<P>>){
        if data.is_now(){
            self.vec_deque.borrow_mut().push_start(data);
        }else{
            self.vec_deque.borrow_mut().push(data);
        }
    }
    pub fn pop(&self)->Option<Rc<dyn PriorityTaskTrait<P>>>{
        self.vec_deque.borrow_mut().pop()
    }
    fn eviscerate_branch(parent: &Rc<dyn PriorityTaskTrait<P>>){
        let mut stack =vec![parent.clone()];

        while let Some(value) = stack.pop(){
            value.set_killed(true);
            value.set_started(true);
            for child in value.get_children().borrow().as_ref().unwrap(){
                if let Some(child) = child.upgrade(){
                    stack.push(child.clone());
                }
            }
        }

    }
    fn run(mut self: Rc<Self>)->Option<Box<dyn Any>>{
        let rw = dummy_raw_waker();
        let w = unsafe { Waker::from_raw(rw) };
        let mut ctx = Context::from_waker(&w);

        while let Some(mut state) =  self.pop(){
            // println!("left {}",self.vec_deque.borrow().len());
            assert_eq!(0,state.get_children_count());
            if(0>state.get_children_count()){
                panic!("bro")
            }
            if state.was_killed(){
                continue
            }
            let   catch= &state.get_catch();
            if catch.is_some() && catch.as_ref().unwrap().is_throwing(){
                state.set_started(true);
                state.destroy(&self,false);
                let catch_parent = state.get_catch().unwrap().get_parent().unwrap();
                if catch_parent.get_children_count()==0{
                    self.push(catch_parent);
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

                if state.is_select(){
                    let catch_parent = state.get_catch().unwrap().get_parent().unwrap();
                    let catch_clone = catch_parent.clone();
                    let children = catch_clone.get_children().borrow();
                    for child in children.as_ref().unwrap(){
                        if let Some(child) = child.upgrade() {
                            if child.is_select(){
                                Self::eviscerate_branch(&child);
                            }
                        }
                    }
                    catch_parent.set_select_count(0);
                    if catch_parent.get_finished(){
                        catch_parent.destroy(&self,true);
                        continue
                    }
                    if catch_parent.get_children_count()==0{
                        self.push(catch_parent);
                    }
                    continue
                }

                if let None = state.get_catch() {
                    return self.last_resort_exception_container.take();
                }
                state.destroy(&self,false);
                let catch_parent = state.get_catch().unwrap().get_parent().unwrap();
                if catch_parent.get_finished(){
                    catch_parent.destroy(&self,true);
                    continue
                }
                if catch_parent.get_children_count()==0{
                    self.push(catch_parent);
                }
                continue;
            }
            if !finished_function && state.get_children_count()==0 && state.get_select_count()==0{
                self.push(state);
            }else if finished_function && state.get_children_count()==0 && state.get_select_count()==0{
                state.destroy(&self,true);
            }
        }

        return None

    }
}
pub async fn relay_internal<P:Ord>(data:Box<dyn Any>,queue: &PriorityQueue<P>, priority: Priority<P>,change_priority:bool){
    match queue.inner_priority_queue.current.borrow().as_ref().unwrap().get_catch() {
        None => {
            queue.inner_priority_queue.last_resort_exception_container.set(Some(data));

        }
        Some(catch) => {
            if change_priority{
                catch.get_parent().unwrap().set_value(priority);
            }

            catch.set_exception_store(Some(data));
            catch.set_is_throwing(true);
        }
    };

    queue.inner_priority_queue.exception_ready.set(true);
    FunctionState::Throwing.await;
}
pub async fn throw_priority_internal<P: Ord, T: 'static>(data:T,  queue: &PriorityQueue<P>, priority: Priority<P>, change_priority: bool){
    let data: Box<dyn Any>= Box::new(data);
    relay_internal(data,queue,priority,change_priority).await
}

pub async fn join_priority_internal<P: Ord>(priority: Priority<P>, priority_queue: &PriorityQueue<P>){
    priority_queue.inner_priority_queue .current.borrow().as_ref().unwrap().set_value(priority);
    FunctionState::Join.await;
}
#[macro_export]
macro_rules! throw_priority {
    ($data:expr => $queue:expr; now) => {
        $crate::entities::throw_priority_internal($data,$queue, Priority::Now, true).await;
        panic!("returned to thrown code")
    };
    ($data:expr => $queue:expr; whenever) => {
        $crate::entities::throw_priority_internal($data,$queue, Priority::Now,false).await;
        panic!("returned to thrown code")
    };
    ($data:expr => $queue:expr; with $priority:expr) => {
        $crate::entities::throw_priority_internal($data,$queue,$priority,true).await;
        panic!("returned to thrown code")
    };
}

#[macro_export]
macro_rules! relay {
    ($data:expr => $queue:expr; whenever) => {
        relay_internal($data,$queue,Priority::Now,false)
    };
    ($data:expr => $queue:expr; with $priority:expr) => {
        relay_internal($data,$queue,$priority,true)
    };
}

#[macro_export]
macro_rules! join_priority {
    ($priority:expr => $queue:expr) => {
        join_priority_internal($priority, $queue).await
    };
}
#[macro_export]
macro_rules! join_priority_now {
    ($queue:expr) => {
        join_priority_internal(Priority::Now, $queue).await
    };
}
pub use throw_priority;
pub use relay;
pub use join_priority;
pub use join_priority_now;
use crate::entities::heap::Heap;
use crate::entities::Priority::Value;
use crate::priority_fib;

pub struct PriorityQueue<P: Ord> where P: Ord{
    inner_priority_queue: Rc<InnerPriorityQueue<P>>
}
pub enum PriorityStyle{
    Min,
    Max,
    Queue
}
impl<P: Ord> PriorityQueue<P>{
    pub fn get_priority(&self)-> Priority<P>
        where P: Copy + Clone
    {
        self.inner_priority_queue.current.borrow().clone().unwrap().get_value()
    }
    pub fn clone_priority(&self)-> Priority<P>
    where P:  Clone
    {
        self.inner_priority_queue.current.borrow().clone().unwrap().clone_value()
    }
    pub fn set_priority(&self, value: Priority<P>){
        self.inner_priority_queue.current.borrow().as_ref().unwrap().set_value(value)
    }
    pub fn replace_priority(&self, value: Priority<P>)-> Priority<P>{
        let vat = self.inner_priority_queue.current.borrow().as_ref().unwrap().take_value();
        self.inner_priority_queue.current.borrow().as_ref().unwrap().set_value(value);
        vat
    }
    pub fn new(priority_style: PriorityStyle)->Self{
        PriorityQueue{
            inner_priority_queue: Rc::new(InnerPriorityQueue::new(priority_style))
        }
    }
    fn new_internal(priority_style: PriorityStyle)->Self{
        PriorityQueue{
            inner_priority_queue: Rc::new(InnerPriorityQueue::new(priority_style))
        }
    }
    pub fn current_contract(&self)->Contract<P>{
        Contract{
            priority_task: Rc::downgrade(&self.inner_priority_queue.current.borrow().clone().unwrap())
        }
    }
    pub fn run<O: 'static, F>(&mut self, start: F, priority: Priority<P>) -> PriorityExceptionPromise<O,P>
    where
        F: Future<Output=O>,

    {
        let final_answer =self.add_priority_task(start,  None, None, true,priority);
        let potential_catch = self.inner_priority_queue.clone().run();
        PriorityExceptionPromise {
            promise: final_answer,
            current: CurrentOrValue::Value(potential_catch)
        }


    }
    fn add_priority_task<O, T>(&self, future: T, parent: Option<Rc<dyn PriorityTaskTrait<P>>>, catch: Option<Rc<dyn PriorityTaskTrait<P>>>, is_head: bool, priority: Priority<P>) -> Promise<O>
    where
        T: Future<Output=O>
    {

        let queue = self.clone();
        let mut p = Promise::new();
        let pclone = p.clone();

        let boxed = Box::pin(future) as Pin<Box<dyn Future<Output=O>>>;
        let interpretation: Pin<Box<dyn Future<Output=O> + 'static>> = unsafe {transmute(boxed) };


        let task = PriorityTask {
            value: Cell::new(priority),
            is_head: Cell::new(is_head),
            future: RefCell::new(interpretation),
            promise: RefCell::new(p),
            children_count: Cell::new(0),
            parent: RefCell::new(parent.clone()),
            catch: Cell::new(catch),
            exception_store: Cell::new(None),
            is_throwing: Cell::new(false),
            finished: Cell::new(false),
            is_select: Cell::new(false),
            is_select_head: Cell::new(false),
            children: RefCell::new(Some(Vec::new())),
            select_count: Cell::new(0),
            killed: Cell::new(false),
        };
        let task: Rc<dyn PriorityTaskTrait<P>> =Rc::new(task);

        //extends lifetime
        let task =unsafe{
            transmute(task)
        };
        if parent.is_some(){
            if let Some(mut children) = parent.as_ref().unwrap().get_children().borrow_mut().as_mut(){
                children.push(Rc::downgrade(&task));
            }
        }
        queue.inner_priority_queue.vec_deque.borrow_mut().push(task);
        pclone
    }
    pub fn add_priority<O, T>(&self,future: T, value: P) -> Promise<O>
    where
        T: Future<Output=O>,
    {
        let current = self.inner_priority_queue.current.borrow();
        let unwrapped = current.as_ref().unwrap();
        unwrapped.increment_children();
        return self.add_priority_task(future,current.clone(),unwrapped.get_catch(),false, Value(value));
    }
    pub fn select_priority<O,T:Future<Output = O>>(&self, future: T, value: P) -> PriorityExceptionSelect<O,P>{

        let queue = self.clone();

        let current = queue.inner_priority_queue.current.borrow();

        let mut promise = Promise::new();

        let boxed = Box::pin(future) as Pin<Box<dyn Future<Output=O>>>;
        let interpretation: Pin<Box<dyn Future<Output=O> + 'static>> = unsafe {transmute(boxed) };

        let unwrapped_current = current.as_ref().unwrap();
        // unwrapped_current.se
        // current
        let task = PriorityTask{
            value: Cell::new(Value(value)),
            is_head: Cell::new(false),
            future: RefCell::new(interpretation),
            promise: RefCell::new(promise.clone()),
            children_count: Cell::new(0),
            parent: RefCell::new(current.clone()),
            catch: Cell::new(None),
            is_throwing: Cell::new(false),
            exception_store: Cell::new(None),
            finished: Cell::new(false),
            is_select: Cell::new(true),
            is_select_head: Cell::new(true),
            children: RefCell::new(Some(Vec::new())),
            select_count: Cell::new(0),
            killed: Cell::new(false),
        };

        let task: Rc<dyn PriorityTaskTrait<P>> =Rc::new(task);
        let task: Rc<dyn PriorityTaskTrait<P>> =unsafe{
            transmute(task)
        };
        if let Some(mut children) = current.as_ref().unwrap().get_children().borrow_mut().as_mut(){
            children.push(Rc::downgrade(&task));
        }

        task.set_catch(Some(task.clone()));
        let mut queue_result: PriorityExceptionSelect<O,P> = PriorityExceptionSelect {
            promise: promise.clone(),
            current: CurrentOrValue::Current(task.clone())
        };
        queue.inner_priority_queue.vec_deque.borrow_mut().push(task);

        let current = current.clone().unwrap();
        current.set_select_count(current.get_select_count()+1);

        queue_result
    }
    pub fn catch_priority<O,T:Future<Output = O>>(&self, future: T, value: P) -> PriorityExceptionPromise<O,P >{

        let queue = self.clone();

        let current = queue.inner_priority_queue.current.borrow();

        let mut promise = Promise::new();

        let boxed = Box::pin(future) as Pin<Box<dyn Future<Output=O>>>;
        let interpretation: Pin<Box<dyn Future<Output=O> + 'static>> = unsafe {transmute(boxed) };

        let unwrapped_current = current.as_ref().unwrap();
        // unwrapped_current.se
        // current
        let task = PriorityTask{
            value: Cell::new(Value(value)),
            is_head: Cell::new(false),
            future: RefCell::new(interpretation),
            promise: RefCell::new(promise.clone()),
            children_count: Cell::new(0),
            parent: RefCell::new(current.clone()),
            catch: Cell::new(None),
            is_throwing: Cell::new(false),
            exception_store: Cell::new(None),
            finished: Cell::new(false),
            is_select: Cell::new(false),
            is_select_head: Cell::new(false),
            children: RefCell::new(Some(Vec::new())),
            select_count: Cell::new(0),
            killed: Cell::new(false),
        };

        let task: Rc<dyn PriorityTaskTrait<P>> =Rc::new(task);
        let task: Rc<dyn PriorityTaskTrait<P>> =unsafe{
            transmute(task)
        };
        if let Some(mut children) = current.as_ref().unwrap().get_children().borrow_mut().as_mut(){
            children.push(Rc::downgrade(&task));
        }

        task.set_catch(Some(task.clone()));
        let mut queue_result: PriorityExceptionPromise<O,P> = PriorityExceptionPromise {
            promise: promise.clone(),
            current: CurrentOrValue::Current(task.clone())
        };
        queue.inner_priority_queue.vec_deque.borrow_mut().push(task);

        current.clone().unwrap().increment_children();

        queue_result
    }
}
impl<P: Ord> Clone for PriorityQueue<P>{
    fn clone(&self) -> Self{
        PriorityQueue{
            inner_priority_queue: self.inner_priority_queue.clone()
        }
    }
}