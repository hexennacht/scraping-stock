use std::{collections::HashMap, error::Error, fmt::{self, Debug}, sync::{Arc, RwLock}};
use structopt::StructOpt;


#[derive(StructOpt, Debug, Clone)]
struct CLI {
    #[structopt(short, long, default_value = "AAPL:NASDAQ,BBCA:IDX,TLKM:IDX")]
    codes: String,

    #[structopt(short, long, default_value = "10")]
    interval: u64,

    #[structopt(short, long)]
    use_async: bool,
}

#[derive(Debug, Clone, Default)]
struct Stock {
    symbol: String,
    company_name: String,
    price: f64,
    status: String,
}

impl Stock {
    fn new(symbol: String, company_name: String, price: f64, status: String) -> Self {
        Self { symbol, company_name, price, status }
    }
}

impl fmt::Display for Stock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}: ${} ({})", self.symbol, self.company_name, self.price, self.status)
    }
}

#[derive(Debug)]
struct StockError {
    code: String,
    message: String,
}

impl fmt::Display for StockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl StockError {
    fn new(code: String, message: String) -> Self {
        Self { code, message }
    }
}

impl Error for StockError {}

pub fn fetch_stock_price() {
    let args = CLI::from_args();

    match args.use_async {
        true => determine_stock_status(args),
        false => async_determine_stock_status(&args),
    }
}

fn async_determine_stock_status(args: &CLI) {
    let mut data: Arc<RwLock<HashMap<String, Stock>>> = Arc::new(RwLock::new(HashMap::new()));

    loop {
        let cloned_args: CLI = args.clone();
        let codes = cloned_args.codes.split(",").collect::<Vec<&str>>().into_iter().map(|code| {
            code.to_string()
        }).collect::<Vec<String>>();

        for share_code in codes {
            let local_data = Arc::clone(&data);

            std::thread::spawn(move || {
                let html_content = fetch_from_google_finance(share_code.as_str()).unwrap();
                let mut new_stock = parse_stock_value(html_content, share_code.as_str()).unwrap();
                let default = &Stock::new("".to_string(), "".to_string(), 0f64, "".to_string());

                let past_stock = local_data.clone()
                    .read()
                    .unwrap()
                    .get(share_code.as_str())
                    .unwrap_or(default)
                    .clone();

                new_stock.status = get_stock_valuation_status(&new_stock.clone(), &past_stock);
                
                local_data.write().unwrap().insert(share_code, new_stock.clone());

                println!("New status = {:?}", new_stock.clone());

                new_stock
            });
        }

        std::thread::sleep(std::time::Duration::from_secs(args.interval));
    }

}

fn determine_stock_status(args: CLI) {
    let mut past_data: HashMap<String, Stock> = HashMap::new();

    loop {
        args.clone().codes.split(",").for_each(|share_code| {
            let html_content = fetch_from_google_finance(share_code).unwrap();
            let new_stock = parse_stock_value(html_content, share_code).unwrap();
    
            let stock = past_data.get(new_stock.symbol.as_str())
                .map(|past_stock| {
                    let mut nstock = new_stock.clone();
                    
                    nstock.status = get_stock_valuation_status(&nstock, past_stock);

                    println!("{:?}", nstock);

                    nstock
                })
                .unwrap_or(new_stock);
    
            past_data.insert(share_code.to_string(), stock.clone());    
        });

        std::thread::sleep(std::time::Duration::from_secs(args.interval));
    }
}

fn get_stock_valuation_status(nstock: &Stock, past_stock: &Stock) -> String {
    match nstock.price.partial_cmp(&past_stock.price) {
        Some(std::cmp::Ordering::Greater) => "up".to_string(),
        Some(std::cmp::Ordering::Less) => "down".to_string(),
        _ => "same".to_string(),
    }
}

fn parse_stock_value(html_content: String, stock: &str) -> Result<Stock, StockError> {
    let html_selector = scraper::Html::parse_document(&html_content);

    let company_selector = scraper::Selector::parse(".zzDege")
        .map_err(|err| {
            StockError::new("SELECTOR_FAILED".to_string(), err.to_string())
        })?;

    let stock_value_selector = scraper::Selector::parse(".YMlKec.fxKbKc")
        .map_err(|err| {
            StockError::new("SELECTOR_FAILED".to_string(), err.to_string())
        })?;

    let company_name = html_selector.select(&company_selector)
        .next()
        .map(|value| {
            value.text().next().unwrap_or("N/A").to_string()
        })
        .unwrap_or("N/A".to_string());

    let stock_value = html_selector.select(&stock_value_selector)
        .next()
        .map(|value| {
            let v = value.text().next()
                .unwrap_or("0,0")
                .replace("$", "")
                .replace("Rp\u{a0}", "")
                .replace(",","");
            
            v.parse::<f64>().unwrap_or(0f64)
        })
        .unwrap_or(0f64);


    let stock_code = stock
        .to_uppercase()
        .split(":")
        .nth(0)
        .unwrap_or(stock)
        .to_string();
    
    Ok(Stock::new(stock_code, company_name, stock_value, "up".to_string()))
}

fn fetch_from_google_finance(stock: &str) -> Result<String, StockError> {
    let base_url = "https://www.google.com/finance/quote/";

    let url = url::Url::parse(&format!("{}{}", base_url, stock))
        .map_err(move |err| {
            println!("{:?}", err.clone());
            StockError::new("PARSE_URL_FAILED".to_string(), err.to_string())
        })?;

    
    let client = reqwest::blocking::Client::new();
    
    let res = client.get(url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .map_err(|err| {
            StockError::new("REQUEST_FAILED".to_string(), err.to_string())
        })?;

    if !res.status().is_success() {
        return Err(StockError::new("RESPONSE_FAILED".to_string(), res.status().to_string()));
    }

    let html_content = res.text()
        .map_err(|err| {
            StockError::new("RESPONSE_BODY_FAILED".to_string(), err.to_string())
        })?;

    Ok(html_content)
}