use std::{collections::HashMap, error::Error, fmt::{self, Debug}};
use structopt::StructOpt;


#[derive(StructOpt, Debug, Clone)]
struct CLI {
    #[structopt(short, long)]
    verbose: bool,

    #[structopt(short, long, default_value = "AAPL:NASDAQ,BBCA:IDX,TLKM:IDX")]
    codes: String,

    #[structopt(short, long, default_value = "10")]
    interval: u64,
}

#[derive(Debug, Clone)]
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

    determine_stock_status(args);
}

fn determine_stock_status(args: CLI) {
    let mut past_data: HashMap<String, Stock> = HashMap::new();

    loop {
        args.clone().codes.split(",").for_each(|stock| {
            let html_content = fetch_from_google_finance(stock).unwrap();
    
            let new_stock = parse_stock_value(html_content, stock).unwrap();
    
            let stock = past_data.get(new_stock.clone().symbol.as_str())
                .map(|past_stock| {
                    let mut nstock = new_stock.clone();
    
                    if nstock.price > past_stock.price {
                        nstock.status = "up".to_string();
                    } else if nstock.price < past_stock.price {
                        nstock.status = "down".to_string();
                    } else {
                        nstock.status = "same".to_string();
                    }
    
                    return nstock;
                })
                .unwrap_or(new_stock.clone());
    
            past_data.insert(new_stock.clone().symbol.clone(), new_stock);
    
            println!("{:?}", stock);
        });

        std::thread::sleep(std::time::Duration::from_secs(args.interval));
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