pub struct Node<T: PartialOrd> {
    pub(crate) left: Option<Box<Node<T>>>,
    pub(crate) right: Option<Box<Node<T>>>,
    pub(crate) value: T,
}

pub struct Tree<T: PartialOrd> {
    pub(crate) head: Option<Box<Node<T>>>,
}

impl<T: PartialOrd> Tree<T> {
    pub fn new() -> Self {
        Tree {
            head: None
        }
    }
    pub fn add(&mut self, value: T) {
        let node = Some(Box::new(Node {
            left: None,
            right: None,
            value,
        }));
        if self.head.is_none() {
            self.head = node;
            return;
        }


        let mut curr = self.head.as_mut().unwrap();
        loop {
            if curr.value > node.as_ref().unwrap().value {
                if curr.left.is_none() {
                    curr.left = node;
                    return;
                }
                curr = curr.left.as_mut().unwrap();
            } else {
                if curr.right.is_none() {
                    curr.right = node;
                    return;
                }
                curr = curr.right.as_mut().unwrap();
            }
        }
    }
}