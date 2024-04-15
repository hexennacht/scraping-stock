pub mod thread;
pub mod scraping;

fn main() {
    println!("Hello, world!");
    
    scraping::googlefinance::fetch_stock_price();
}
