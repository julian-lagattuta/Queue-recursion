# Queue-Based/Breadth-First Recursion
This project reimagines recursion by replacing "the Stack" with "the Queue". Before, recursion was a paradigm restricted only to depth-first algorithms. This is no longer the case with queue-based recursion.
Written in Rust and implemented via a custom async executor, it enables breadth-first algorithms to be written as intuitively as depth-first ones, maintaining clean, readable logic without requiring loops for queue management.

### Main Features
+ **Breadth-first recursion**: Function calls execute in first-in, first-out order.
+ **Priority based recursion**: Function calls can be placed into priority queues.
+ **Queue exceptions**:  The most unique feature, allowing information to be easily propagated through the Queue.

# Example Code

### Print Binary Tree Breadth-First
```rust
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
                //calls print_tree_helper on left node by adding to queue
                let left =queue.add_priority(print_tree_helper(&n.left, queue),());
            }
            if(n.right.is_some()){
                //calls print_tree_helper on right node by adding to queue
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


```

## Recursive Implementation of Dijkstra's
```rust
pub fn dijkstras(&self,a: usize, b: usize)-> Option<Vec<usize>>{
    let mut visited = vec![false; self.num];

    //Create queue. Prioritize SMALLER(Min) values.
    let mut queue = PriorityQueue::new(PriorityStyle::Min);

    //This runs dijkstras_helper breadth-first around a "try-catch"
    match queue.run(self.dijkstras_helper(a, b, &mut visited, &queue.clone()),Priority::Value(0)). //Priority::Value(0) means that we are starting with zero cost at the starting node
        unwrap(). //unwrap never fails here
        consume_type::<Vec<usize>>(). //The time of an exception can by anything, so I assume the type is a Vec<usize>
        unwrap() //unwrap fails if theh exception type isn't Vec<usize>
        {
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

    //I create a separate branch for each connected node
    let mut promises = Vec::new();
    for i in 0..self.num{

        //ignore visited nodes
        if i == a || visited_nodes[i] || self.matrix[a][i].is_none(){
            continue
        }
        
        //get cost to go from a to i
        let value = self.matrix[a][i].unwrap();
        if i == b{
            //When the end node is found, we raise a queue-exception, throwing a Vec<usize>, representing the path
            //This exception will propagate upwards, with each function adding its current node to the Vec
            throw_priority!(vec![b,a] => priority_queue; now); //I use the keyword "now" because it is important, in order to not waste time, for the catcher to catch the code immediately, rather than being put in the back of the line. 
        }
        let new_distance =previous_priority+value;

        //this is the recursive call.  Notice the term "select". This means that an exception will cancel the rest of the children function calls
        //This is useful because we don't want to keep searching once the end node is found
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
    //If no exceptions were found, no exceptions are thrown. This implies that the end node was not found

}

```

#  Full Explanation/Documentation
### PriorityStyle Enum
#### Min
Prioritize smaller values
#### Max
Prioritize larger values
#### Queue
Not priority based. Everytime a priority is accepted as an argument, pass `Priority::Value(())` or `Priority::Now` instead.

### PriorityQueue Struct
#### New
```rust
 fn new(PriorityStyle style)->PriorityQueue;
 ```
Creates a new priority queue

#### Priority functions
```rust
fn get_priority(&self)-> Priority<P>;
fn clone_priority(&self)-> Priority<P>;
fn replace_priority(&self, value: Priority<P>)-> Priority<P>;
fn set_priority(&self, value: Priority<P>);
```
These all modify/read the current function's priority. Reminder that modifying the priority ONLY matters when, in that same function, you call `throw_priority!(value => queue; whenever);` with `whenever`, otherwise it will be overwritten.

#### Run
```rust

fn run<O: 'static, F>(&mut self, start: F, priority: Priority<P>) -> PriorityExceptionPromise<O,P>
where
    F: Future<Output=O>
```
Runs the given function. The priority is the priority for the starting function. For example, in Dijkstra's, you would use zero since the starting cost is zero.
### Recursive Calls
#### Add_priority

```rust 
fn add_priority<O, T>(&self,future: T, value: P) -> Promise<O>;
```
This is the main way you call a function.

Adds the given function to the call queue. It gets placed in the call queue based on its given value/priority.
If you are using ``PriorityStyle::Queue``, then put `()` as your value. 


If the function below it raises an exception, that exception will continue up the call stack, canceling all functions below the add_priority.

### Joining
The name "join" comes from threading. Joining threads means that those threads will have finished. Likewise, in queue-based recursion, joining causes the functions it called to run and finish by pausing the current function  and switching contexts by popping from the queue.

There are two ways to join.
```rust
join_priority!([priority]=>[&queue]);
```
This will call join. Once all the function's children finish executing, the function will be added back to the queue. The priority when it is added back is the priority you pass as an argument.


Reminder: the priority has type Priority. Therefore you may have to input `Priority::Value(priority)` as  the priority.
```rust
join_priority_now!([&queue]);
```
This is a special case of the first one and is equivalent to `join_priority!(Priority::Now => [&queue])`;

For both join functions, the priority can be overwritten by throwing exceptions. Please view that section for more info. 
### Priority Exceptions
Let's say you want to search an unsorted binary tree for a specific value, breadth-first. Once you find the value you are looking for, how can you communicate to the other function calls that there is no more need to search? This is done through exceptions.

When you throw an exception normally, I like to think of them as a sort of "super return" statement that skips over every function "above it", until it gets "caught" by a `catch`. 
In queue based recursion, there isn't just "above", there also exists functions which exists to the function's left and right, in the same way that tree nodes have a left and a right.

There are a few ways to throw exceptions 


#### Exception Throwing
```rust
throw_priority!([exception value] => [&queue]; with [priority]);
```
Throws an exception. Exceptions can be any type. The priority represents the priority for the "catcher" (the function that catches the error). If you are using `PriorityStyle::Queue`, putting `()` as the priority  will place the catcher at the end of the queue. This will override the priority set by any of the "join" functions.

Reminder: the priority has type Priority. Therefore you may have to input `Priority::Value(priority)` as  the priority.
```rust
throw_priority!([exception value] => [&queue]; now);
```
Throws an exception. Exceptions can be any type. The "now" means that the "catcher" will run immediately. This way to throw an exception is probably the most common to use. This will override the priority set by any of the "join" functions.
```rust
throw_priority!([exception value] => [&queue]; whenever);
```
Throws an exception. Exceptions can be any type. The "whenever" means that the priority of the "catcher" remains unchanged from what is was set to from its called to `join_priority`.



#### Catch_priority
```rust
fn catch_priority<O,T:Future<Output = O>>(&self, future: T, value: P) -> PriorityExceptionPromise<O,P >; 
```
This can be thought of as `add_priority` except that it is surrounded by `try/catch` statement. Once you unwrap the return value, you can `unwrap_ok()` or check to see if there was exception using `consume_type<T>()` or `consume_any()`. 

#### Select_priority
```rust
fn select_priority<O,T:Future<Output = O>>(&self, future: T, value: P) -> PriorityExceptionSelect<O,P>;
```
More useful version of `catch_priority`, although it has the limitation that it cannot be used with `add_priority` or `catch_priority` in the same "join".
`select_priority` allows you to put multiple function calls in the same "catch" block, metaphorically speaking.
For example:

```rust
let a_catch = queue.select_priority(a(),());
let b_catch = queue.select_priority(b(),());
let c_catch = queue.select_priority(c(),());
join_priority_now!(queue);
```

This is analogous to 
```java
try{
    let a_value = a();
    let b_value = b();
    let c_value = c();
}catch(Exception e){
    ...
}
```
It will run the functions `a`,`b`,`c`, until one of them raises an exception. Once one of them raises an exception, the rest stop running.
You can match over the return value. View my Dijkstra's example for a working example.  In that example, I use `select_priority` so that the search does not continue after the end node has been found.
## Bonus Info

If you want to throw a caught exception, you can use `relay([Box<dyn Any>]=> [&PriorityQueue];with/whenever [priority])`

You can create "Contracts", which allow functions to modify other function's priority. You can access a contract via `queue.current_contract();`

## Try it out!
Download the repo and modify main.rs. You can test out things there. In the future I will make it a library.

## Is this idea original?
I'm pretty sure some of it is. But if you use this idea anywhere, credit me.