pub mod content;
pub mod link;
pub mod collection;
pub mod protocols;
pub mod manager;
pub mod prelude;

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
    let urls = vec![Url::parse(String::from("https://wiki.insa-rouen.fr/doku.php?id=insa:iti:maquette:semestre_6:start")).unwrap()];

    let mut graph = UrlCollection::new();
    let err = if urls.is_empty() {
        graph.load_graph().await;
        // Remove files 
        let _ = std::fs::remove_file("edges.csv");
        let _ = std::fs::remove_file("nodes.csv");        
        graph.fetch().await
    } else {
        // Remove files 
        let _ = std::fs::remove_file("edges.csv");
        let _ = std::fs::remove_file("nodes.csv");        
        graph.fetch_from(urls).await
    };

    if let Err(err) = err {
        println!("{:?}", style(err).red());
    }

}
