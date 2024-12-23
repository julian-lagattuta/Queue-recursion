use std::collections::VecDeque;
use std::mem::{replace, swap};

pub(super) struct Heap<T: Ord>{
    data: Vec<T>,
    start: Vec<T>,
    is_max: bool,
    is_queue: bool,
    vec_deque: VecDeque<T>
}
fn parent(i: usize)-> usize{
    (i+1)/2-1
}
fn left(i: usize)->usize{
    i*2+1
}
fn right(i:usize)-> usize{
    i*2+2
}
impl<T: Ord> Heap<T> {
    pub fn new(is_max:bool, is_queue: bool)->Self{
        if is_max && is_queue{
            panic!("illegal state")
        }
        Heap{
            data: Vec::new(),
            is_max,
            start: Vec::new(),
            is_queue,
            vec_deque: VecDeque::new()
        }
    }
    pub fn len(&self)->usize{
        if self.is_queue{
            self.vec_deque.len()
        }else{
            self.data.len() + self.start.len()
        }
    }
    pub fn pop(&mut self)->Option<T>{
        if self.is_queue{
            return self.vec_deque.pop_front()
        }
        if let Some(start) = self.start.pop(){
            return Some(start)
        }
        if self.data.len()==0{
            return None
        }
        if self.data.len()==1{
            return Some(self.data.pop().unwrap());
        }
        let popped = self.data.pop().unwrap();
        let value = replace(&mut self.data[0],popped);
        if self.is_max{
            let mut i = 0;
            loop {
                if left(i) >= self.data.len() {
                    break
                }

                if self.data[i]< self.data[left(i)] && right(i)>= self.data.len(){
                    self.data.swap(i,left(i));
                    i =left(i);
                    break
                }

                if right(i) < self.data.len() {
                    if self.data[right(i)]> self.data[left(i)]{
                        if self.data[i]< self.data[right(i)]{
                            self.data.swap(i,right(i));
                            i = right(i);
                            continue
                        }else if self.data[i]< self.data[left(i)]{
                            self.data.swap(i,left(i));
                            i = left(i);
                            continue
                        }
                    }else{
                        if self.data[i]< self.data[left(i)]{
                            self.data.swap(i,left(i));
                            i = left(i);
                            continue
                        }

                    }
                }
                break
            }
        }else{

            let mut i = 0;
            loop {
                if left(i) >= self.data.len() {
                    break
                }

                if self.data[i]> self.data[left(i)] && right(i)>= self.data.len(){
                    self.data.swap(i,left(i));
                    i =left(i);
                    break
                }

                if right(i) < self.data.len() {
                    if self.data[right(i)]< self.data[left(i)]{
                        if self.data[i]> self.data[right(i)]{
                            self.data.swap(i,right(i));
                            i = right(i);
                            continue
                        }else if self.data[i]> self.data[left(i)]{
                            self.data.swap(i,left(i));
                            i = left(i);
                            continue
                        }
                    }else{
                        if self.data[i]> self.data[left(i)]{
                            self.data.swap(i,left(i));
                            i = left(i);
                            continue
                        }

                    }
                }
                break
            }
        }

        Some(value)
    }
    pub fn push_start(&mut self, value: T){
        if self.is_queue{
            self.vec_deque.push_front(value);
            return
        }
        if self.start.len()>0{
            todo!("wowowow")
        }
        self.start.push(value);
    }
    pub fn push(&mut self, value: T){
        if self.is_queue{
            self.vec_deque.push_back(value);
            return
        }
        self.data.push(value);
        if self.is_max{
            let mut i = self.data.len()-1;
            while i!=0 && self.data[i]>self.data[parent(i)]{
                self.data.swap(i,parent(i));
                i = parent(i);
            }
        }else{
            let mut i = self.data.len()-1;
            while i!=0&& self.data[i]<self.data[parent(i)]{
                self.data.swap(i,parent(i));
                i = parent(i);
            }
        }
    }
}