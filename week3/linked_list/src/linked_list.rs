use std::fmt::{self, Display};
use std::option::Option;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {
            value: value,
            next: next,
        }
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node::new(self.value.clone(), self.next.clone())
    }
}
impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        return self.value == other.value;
    }
}
impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.get_size() != other.get_size() {
            return false;
        }
        let mut lhs = (&self.head).as_ref();
        let mut rhs = (&other.head).as_ref();
        while lhs.is_some() {
            let l_value = lhs.expect("l_value");
            let r_value = rhs.expect("r_value");
            if l_value.value != r_value.value {
                return false;
            }
            lhs = (&l_value.next).as_ref();
            rhs = (&r_value.next).as_ref();
        }
        return true;
    }
}
impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        LinkedList {
            head: self.head.clone(),
            size: self.size,
        }
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }

    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}

pub struct LinkedListIter<'a, T: Copy> {
    current: &'a Option<Box<Node<T>>>,
}

impl<T: Copy> Iterator for LinkedListIter<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.current {
            Some(node) => {
                self.current = &node.next;
                Some(node.value)
            }
            None => None, // YOU FILL THIS IN!
        }
    }
}

impl<T> Iterator for LinkedList<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.pop_front()
    }
}

impl<'a, T: Copy> IntoIterator for &'a LinkedList<T> {
    type Item = T;
    type IntoIter = LinkedListIter<'a, T>;
    fn into_iter(self) -> LinkedListIter<'a, T> {
        LinkedListIter {
            current: &self.head,
        }
    }
}

impl<T: Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                }
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}
