pub mod content;
pub mod link;
pub mod page;

use crate::page::Page;
#[tokio::main]
async fn main() {
    let url = link::Url::parse(&"https://insagenda.fr").unwrap();
    let page = Page::new(url).await.unwrap();
    println!("{:?}", page);
}
