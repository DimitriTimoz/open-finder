pub mod content;
pub mod link;
pub mod collection;
pub mod protocols;
pub mod manager;
pub mod prelude;

use std::fs::File;

use console::{style, Term};
use link::Url;

use crate::collection::UrlCollection;

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
        style("Please, enter a url to start crawling (nothing to start the scan or resume the current scan): ").green()
    );
    let urls = vec![Url::parse(String::from("https://cas.insa-rouen.fr/cas/login?service=https%3A%2F%2Fwiki.insa-rouen.fr%2Fdoku.php%3Fid%3Dinsa%3Aiti%3Amaquette%3Asemestre_6%3Astart")).unwrap()];

    let mut graph = UrlCollection::new();
    let err = if File::open("fetcheds.csv").is_ok() && File::open("to_fetch.csv").is_ok() {
        graph.load_graph().await;
        // Remove files 
        graph.fetch().await
    } else {
        // Remove files 
        let _ = std::fs::remove_file("fetcheds.csv");
        let _ = std::fs::remove_file("to_fetch.csv");        
        graph.fetch_from(urls).await
    };

    if let Err(err) = err {
        println!("{:?}", style(err).red());
    }

}
