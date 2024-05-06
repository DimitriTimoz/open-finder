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

pub async fn extract_text_from_pdf(bytes: &[u8], txt: &mut String) {
    let text = pdf_extract::extract_text_from_mem(bytes);
    if let Ok(text) = text {
        *txt = text;
    }
}
