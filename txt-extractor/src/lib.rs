use scraper::{Html};

pub async fn extract_text(bytes: &str) -> String {
    let document = Html::parse_document(bytes);

    let mut text = String::new();
    for node in document.root_element().descendants() {
        if let Some(t) = node.value().as_text() {
            text.push_str(t);
        }
    }
    String::new()
}
