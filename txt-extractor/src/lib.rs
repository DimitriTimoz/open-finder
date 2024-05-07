use html2text::from_read;

pub async fn extract_text(bytes: &str, txt: &mut String)  {
    
    let bytes = bytes.as_bytes();
    *txt = from_read(bytes, 1000);
}

pub async fn extract_text_from_pdf(bytes: &[u8], txt: &mut String) {
    let text = pdf_extract::extract_text_from_mem(bytes);
    // Write it to the file
    if let Ok(text) = text {
        *txt = text;
    } else {
        println!("Error: {:?}", text);
        txt.clear();
    }
}
