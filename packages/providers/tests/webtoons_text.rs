use mangas_core::domain::MangaId;
use mangas_providers::webtoons::parse_title_page;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/webtoons");

#[test]
fn spanish_title_pages_decode_named_entities() {
    let id = MangaId("es-webtoon-1386".into());
    let page = r#"<meta property="og:title" content="La Torre de Dios" />
<meta property="og:description" content="&iquest;Qu&eacute; desear&iacute;as? Bam &aacute;rbol &mdash; fin" />"#;
    let manga = parse_title_page(&id, page).unwrap();
    assert_eq!(manga.description, "¿Qué desearías? Bam árbol — fin");
}

#[test]
fn double_encoded_meta_tags_decode_fully() {
    let page = std::fs::read_to_string(format!("{FIXTURES}/title_es_double_encoded.html")).unwrap();
    let manga = parse_title_page(&MangaId("es-webtoon-7337".into()), &page).unwrap();
    assert_eq!(manga.title, "Enamorándome de mi mejor amigo");
    assert_eq!(
        manga.description,
        "¿Y si el amor siempre estuvo a tu lado? Una historia de José — 'todo' cambia & nada vuelve."
    );
}
