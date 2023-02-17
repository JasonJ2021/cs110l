use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    let (sender_input , receiver_input) = crossbeam_channel::unbounded();
    let (sender_output , receiver_output) = crossbeam_channel::unbounded();
    let mut threads = Vec::new();
    // spawn threads , get input from receiver_input , send output to sender_output
    for _ in 0..num_threads {
        let receiver_input = receiver_input.clone();
        let sender_output = sender_output.clone();
        threads.push(thread::spawn(move || {
            while let Ok((index , value)) = receiver_input.recv(){
                sender_output.send((index , f(value))).expect("Trying to send back f(value) , but there is no receivers");
            }
            drop(sender_output);
        }));
    }
    let mut count = 0;
    for value in input_vec {
        sender_input.send((count,value)).expect("Trying to send input , but there is no receivers");
        count += 1;
    }
    drop(sender_input);
    drop(sender_output);
    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }
    output_vec.resize_with(output_vec.capacity(), Default::default);
    // println!("output_vec with len {}" , output_vec.len());
    while let Ok((index , value)) = receiver_output.recv(){
        output_vec[index] = value;
    }
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
