pub mod thread;
pub mod scraping;

fn main() {
    scraping::googlefinance::fetch_stock_price();
}
