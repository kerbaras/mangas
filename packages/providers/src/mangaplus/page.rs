//! MANGA Plus page URLs and image decryption. A page image needs the viewer's token (sent as
//! `Plus-Vw-Token`) and comes XOR-ed with a per-page key, so both ride in the page URL's
//! fragment, which is never sent: `<image url>#key=<hex>&vw=<token>`.

use mangas_core::domain::{Error, Result};

use super::SOURCE_NAME;

/// The header carrying the viewer's token on page image requests.
pub const VIEW_TOKEN_HEADER: &str = "Plus-Vw-Token";

/// The page URL the downloader gets for `image_url`.
pub fn page_url(image_url: &str, key: &str, view_token: &str) -> String {
    let base = image_url.split('#').next().unwrap_or(image_url);
    format!("{base}#key={key}&vw={view_token}")
}

/// The viewer token in a page URL, if it has one.
pub fn view_token(url: &str) -> Option<&str> {
    fragment_value(url, "vw")
}

/// The page image at `url` decrypted with the URL's key; bytes of a URL without a key (a
/// cover) are returned as they are.
pub fn decrypt(url: &str, mut bytes: Vec<u8>) -> Result<Vec<u8>> {
    let Some(hex) = fragment_value(url, "key") else {
        return Ok(bytes);
    };
    let key = from_hex(hex).ok_or_else(|| Error::Source {
        name: SOURCE_NAME.to_string(),
        message: format!("page key {hex:?} is not hex"),
    })?;
    xor(&mut bytes, &key);
    Ok(bytes)
}

/// XOR `bytes` with `key` repeated over them.
pub fn xor(bytes: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }
    for (byte, k) in bytes.iter_mut().zip(key.iter().cycle()) {
        *byte ^= k;
    }
}

fn fragment_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    let (_, fragment) = url.split_once('#')?;
    fragment
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, v)| *k == name && !v.is_empty())
        .map(|(_, v)| v)
}

fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fragment_carries_key_and_token() {
        let url = page_url("https://cdn.test/1.jpg?hash=x&expires=9", "0aff", "tok");
        assert_eq!(
            url,
            "https://cdn.test/1.jpg?hash=x&expires=9#key=0aff&vw=tok"
        );
        assert_eq!(view_token(&url), Some("tok"));
        assert_eq!(decrypt(&url, vec![0x0a, 0x00, 0x0b]).unwrap(), [0, 0xff, 1]);
    }

    #[test]
    fn a_url_without_a_key_is_left_alone() {
        assert_eq!(
            decrypt("https://cdn.test/c.jpg", vec![1, 2]).unwrap(),
            [1, 2]
        );
        assert!(decrypt("https://cdn.test/1.jpg#key=zz", vec![1]).is_err());
    }
}
