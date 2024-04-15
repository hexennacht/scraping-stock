use std::thread;

pub fn run_simple_thread() {


    thread::spawn(move || {
        println!("Hello from a thread!");
    });

    thread::spawn(move || {
        println!("Hello from another thread!");
    });

    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("Hello from main thread!")
}