use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display

    // If you implement iterator trait:
    //for val in &list {
    //    println!("{}", val);
    //}
    // Clone traits
    println!("list now is {}" , list);
    println!("After Clone======================"); 
    let list_clone = list.clone();
    println!("list_clone is {}", list_clone);

    // PartialEq traits
    println!("list = list_clone ? {} " , list == list_clone);

    for i in list{
        println!("{}" , i);
    }
}
