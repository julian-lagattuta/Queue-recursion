use std::any::Any;
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt::{Display};
use std::future::Future;
use std::marker::PhantomPinned;
use std::ops::DerefMut;
use std::rc::Rc;
use thiserror::__private::AsDynError;

use crate::tree::Node;
use crate::entities::*;

mod tree;
mod entities;


// async fn fib(n: i32, ctx: &Queue) -> i32 {
//     println!("{}", n);
//     if n < 2 {
//         return 1;
//     }
//     let mut a = add(fib(n - 1, ctx), ctx);
//     let mut b = add(fib(n - 2, ctx), ctx);
//     join!();
//     return a.unwrap() + b.unwrap();
// }
//
// pub async fn print_tree<T>(node: &Option<Box<Node<T>>>, queue: &Queue)
// where
//     T: Display + PartialOrd,
// {
//     match node {
//         None => { return }
//         Some(n) => {
//             println!("{}", n.value);
//             add(print_tree(&n.left, queue), queue);
//             add(print_tree(&n.right, queue), queue);
//         }
//     }
// }



trait Generic<T>{
    fn get(self)->T;
}
struct Hold<T,P>{
    value: T,
    val: Rc<dyn Generic<T>>,
    k: P
}
impl<T,P> Generic<T> for Hold<T,P>{
    fn get(self) -> T {
        self.value
    }
}
// struct A<T>{
//     hold: Box<dyn Generic<T>>
// }
#[derive(Copy, Clone)]
struct Distance{
    value: i32
}
impl Distance{
    pub fn new(value: i32)->Self{
        Distance{
            value
        }
    }
}
impl Eq for Distance {}

impl PartialEq<Self> for Distance {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl PartialOrd<Self> for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&-other.value)
    }
}

impl Ord for Distance{
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&-other.value)
    }
}
struct Graph{
    matrix: Vec<Vec<Option<i32>>>,
    num: usize
}

impl Graph{
    pub fn new(num: usize)->Self{
        Graph{
            matrix: vec![vec![None;num];num],
            num
        }
    }
    pub fn add_edge(&mut self, a: usize, b: usize, weight: i32){
        self.matrix[a][b] = Some(weight);
        self.matrix[b][a] = Some(weight);
    }
    pub fn dijkstras(&self,a: usize, b: usize)-> Option<Vec<usize>>{
        let mut visited = vec![false; self.num];

        //Create queue. Prioritize SMALLER(Min) values.
        let mut queue = PriorityQueue::new(PriorityStyle::Min);

        //This runs dijkstras_helper breadth-first around a "try-catch"
        match queue.run(self.dijkstras_helper(a, b, &mut visited, &queue.clone()),Priority::Value(0)).unwrap().consume_type::<Vec<usize>>().unwrap() {
            ResultMatch::Ok(_) => {
                //If nothing is "caught", then there is no path from a to b
                None
            }
            ResultMatch::Exception(e) => {
                //I caught the path, I just neeed to reverse the path order
                Some(e.iter().copied().rev().collect())
            }
        }
    }
    async fn dijkstras_helper(&self, a: usize, b: usize, visited_nodes: &mut Vec<bool>, priority_queue: &PriorityQueue<i32>){
        visited_nodes[a] = true;
        //This gets the current function call's priority. In other words, the current "cost"
        let previous_priority = priority_queue.get_priority().unwrap();

        //We create a separate branch for each connected node
        let mut promises = Vec::new();
        for i in 0..self.num{

            //continue if node visited
            if i == a || visited_nodes[i] || self.matrix[a][i].is_none(){
                continue
            }

            let value = self.matrix[a][i].unwrap();
            if i == b{
                //When the node is found, we raise a queue-exception
                throw_priority!(vec![b,a] => priority_queue; now);
            }
            let new_distance =previous_priority+value;

            //this is the recursive call.  Notice the term "select". This means that an exception will cancel the rest of the children function calls
            let promise =priority_queue.select_priority(self.dijkstras_helper(i,b,visited_nodes,priority_queue),new_distance);
            promises.push(promise);
        }
        //This executes all the child function calls until it hits an exception
        join_priority_now!(priority_queue);

        //Search for an exception
        for promise in promises{
            if let SelectMatch::Exception(mut path) = promise.unwrap().consume_type::<Vec<usize>>().unwrap(){
                //Add this node to the path
                path.push(a);
                //throw the path further up the call queue
                throw_priority!(path=> priority_queue; now);
            }
        }

    }
}
// fn is_in_grammar(s: &String, grammar: &Vec<String>) -> bool{
//     return grammar
//         .iter()
//         .any(|rule| s.contains(rule))
// }
// fn is_rule_in_data(s: &String, data: &Vec<String>)->bool{
//     return data.iter().any(|string| string.contains(s))
// }
// async fn bufia_helper(s: String, data: &Vec<String>, alphabet: &str, k: usize, rules: &mut Vec<String>, queue: &Queue){
//     println!("{:?}",s);
//     if is_in_grammar(&s, rules){
//         return
//     }
//     if is_rule_in_data(&s,&data){
//         if s.len()==k{
//             return
//         }
//         for letter in alphabet.chars(){
//             let word =format!("{}{}",s.as_str(),letter);
//             add(bufia_helper(word,data,alphabet,k,rules,queue),queue);
//         }
//         return
//     }
//
//     rules.push(s);
// }
// fn bufia(data: Vec<String>,alphabet: &str)->Vec<String>{
//     let max_len = data.iter().map(|x| x.len()).max().expect("gave empty data array");
//
//     let mut rules = Vec::new();
//     let mut q = Queue::new();
//     q.run(bufia_helper("".to_string(),&data, alphabet, max_len, &mut rules, &q.clone()));
//     rules
//
// }
async fn priority_fib(i: i32, priority_queue: &PriorityQueue<i32>)->i32{
    if i <2{
        return 1;
    }
    let a= priority_queue.add_priority(priority_fib(i-1,priority_queue),i*100);
    let b= priority_queue.add_priority(priority_fib(i-2,priority_queue),i*100);
    join_priority!(Priority::Value(100) => priority_queue);
    a.unwrap()+b.unwrap()
}

pub fn print_tree<T: PartialOrd + Display>(node:&Option<Box<Node<T>>>)
{
    //Create priority queue with PriorityStyle::Queue. This means it is first-in first-out
    let mut queue:PriorityQueue<()> = PriorityQueue::new(PriorityStyle::Queue);
    //Run print_tree_helper breadth first. Priority::Now means nothing in this situation
    queue.run(print_tree_helper(node,&queue.clone()),Priority::Now) ;
}
pub async fn print_tree_helper<T: PartialOrd + Display>(node: &Option<Box<Node<T>>>, queue: &PriorityQueue<()>)

{
    match node {
        None => { return }
        Some(n) => {
            println!("{}", n.value);
            if(n.left.is_some()){
                //adds left node to queue
                let left =queue.add_priority(print_tree_helper(&n.left, queue),());
            }
            if(n.right.is_some()){
                //adds right node to queue
                let right =queue.add_priority(print_tree_helper(&n.right, queue),());
            }
            //join pops from the queue
            //when it is running, all its child function calls run
            join_priority_now!(queue);
            //after the join, all the child function calls will have finished.
            //In other words, all tree nodes below (and including) the current node will have been printed.
            println!("printed all nodes below {}",n.value);
        }
    }
}


// async fn nothing(){}
// async fn unsendable(){
//     async_send_test(unsendable()).await;
// }
fn main() {
    // async_send_test(unsendable());
    let mut graph = Graph::new(20);
    graph.add_edge(0,1,10);
    graph.add_edge(1,2,10);
    graph.add_edge(2,4,10);
    graph.add_edge(4,3,10);
    graph.add_edge(1,8,2);
    graph.add_edge(8,2,1);
    graph.add_edge(0,10,1);
    graph.add_edge(10,11,11);
    graph.add_edge(11,12,11);
    graph.add_edge(12,13,1);
    graph.add_edge(13,14,1);
    graph.add_edge(15,14,1);
    // graph.add_edge(10,4,1);
    let  a= graph.dijkstras(0,4);
    println!("{:?}",a);

    let mut t = tree::Tree::new();
    t.add(100);
    t.add(20);
    t.add(200);
    t.add(300);
    t.add(150);
    t.add(9);

    t.add(6);
    t.add(5);
    t.add(5);
    t.add(61);
    print_tree(&t.head);

    return;

}
