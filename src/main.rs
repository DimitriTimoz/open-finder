pub mod content;
pub mod link;
pub mod page;
pub mod prtocols;
use console::{style, Term};

use crate::page::PagesGraph;

const NAME_ASCII_ART: &str = r#"
 ___  ____  _____ _   _          _____ ___ _   _ ____  _____ ____  
/ _ \|  _ \| ____| \ | |        |  ___|_ _| \ | |  _ \| ____|  _ \ 
| | | | |_) |  _||  \| |        | |_   | ||  \| | | | |  _| | |_) |
| |_| |  __/| |__| |\  |        |  _|  | || |\  | |_| | |___|  _ < 
\___/|_|   |_____|_| \_|        |_|   |___|_| \_|____/|_____|_| \_\
"#;
#[tokio::main]
async fn main() {
    let term = Term::stdout();
    term.clear_screen().unwrap();
    println!("{}", style(NAME_ASCII_ART).green());
    println!("\n");
    println!(
        "{} {} !",
        style("Welcome to").green(),
        style("Open Finder").red()
    );
    println!(
        "{}",
        style("Please, enter a url to start crawling: ").green()
    );
    let url = term.read_line().unwrap();
    let url = link::Url::parse(&url).unwrap();

    println!(
        "{}",
        style("Please, the maximum distance from the root: ").green()
    );
    let distance = term.read_line().unwrap();
    let distance = distance.parse::<u8>().unwrap();
    let mut graph = PagesGraph::new();
    let _ = graph.fetch_form(url, distance).await;
    let links = graph.get_links();
    for link in &links {
        println!("- {}", link);
    }
    println!("{}", style(links.len()).green());
}
