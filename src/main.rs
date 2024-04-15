pub mod thread;
pub mod scraping;

fn main() {
    println!("Hello, world!");

    thread::thread::run_simple_thread();

    scraping::googlefinance::fetch_stock_price();
}
