use scraper::Html;

pub async fn extract_text(bytes: &str, txt: &mut String)  {
    let document = Html::parse_document(bytes);

    txt.clear();
    for node in document.root_element().descendants() {
        // Skip script and style tags
        if let Some(t) = node.value().as_text() {
            txt.push_str(t);
        }
    }
}
