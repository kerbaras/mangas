use mangas_core::send::email::{
    book_message, check_size, explain, parse, parse_sender, Failure, MAX_ATTACHMENT_BYTES,
};
use mangas_core::send::{EmailSettings, Security};

fn settings() -> EmailSettings {
    EmailSettings {
        to: "reader@kindle.com".into(),
        host: " smtp.example.test ".into(),
        username: "me@example.test".into(),
        password: "not-a-real-password".into(),
        ..Default::default()
    }
}

#[test]
fn defaults_port_by_security_and_sender_to_username() {
    let smtp = parse(&settings()).unwrap();
    assert_eq!((smtp.host.as_str(), smtp.port), ("smtp.example.test", 587));
    assert_eq!(smtp.from.email.to_string(), "me@example.test");
    assert_eq!(smtp.to.email.to_string(), "reader@kindle.com");
    let tls = EmailSettings {
        security: Security::Tls,
        ..settings()
    };
    assert_eq!(parse(&tls).unwrap().port, 465);
    let custom = EmailSettings {
        port: Some(2525),
        from: "Me <sender@example.test>".into(),
        ..settings()
    };
    let smtp = parse(&custom).unwrap();
    assert_eq!(smtp.port, 2525);
    assert_eq!(smtp.from.email.to_string(), "sender@example.test");
}

#[test]
fn no_username_means_no_login() {
    let open = EmailSettings {
        username: String::new(),
        from: "me@example.test".into(),
        ..settings()
    };
    assert_eq!(parse(&open).unwrap().credentials, None);
    assert_eq!(
        parse(&settings()).unwrap().credentials,
        Some(("me@example.test".into(), "not-a-real-password".into()))
    );
}

#[test]
fn names_every_missing_field() {
    let err = parse(&EmailSettings::default()).unwrap_err().to_string();
    assert_eq!(
        err,
        "Email delivery isn't set up: missing Kindle address, sender address, SMTP server"
    );
    let bad = EmailSettings {
        to: "not an address".into(),
        ..settings()
    };
    assert!(parse(&bad)
        .unwrap_err()
        .to_string()
        .contains("'not an address'"));
}

#[test]
fn test_email_goes_to_the_sender_without_a_kindle_address() {
    let no_kindle = EmailSettings {
        to: String::new(),
        ..settings()
    };
    assert!(parse(&no_kindle).is_err());
    let smtp = parse_sender(&no_kindle).unwrap();
    assert_eq!(smtp.to, smtp.from);
}

#[test]
fn size_guard_at_amazons_limit() {
    assert!(check_size(MAX_ATTACHMENT_BYTES).is_ok());
    let err = check_size(62_400_000).unwrap_err().to_string();
    assert!(err.contains("62.4 MB"), "{err}");
    assert!(err.contains("up to 50 MB"), "{err}");
}

#[test]
fn book_message_attaches_the_epub() {
    let smtp = parse(&settings()).unwrap();
    let msg = book_message(&smtp, "Vol. 1", "vol-1.epub", b"EPUB-BYTES".to_vec()).unwrap();
    let raw = String::from_utf8(msg.formatted()).unwrap();
    assert!(raw.contains("Subject: Vol. 1"), "{raw}");
    assert!(raw.contains("Content-Type: application/epub+zip"), "{raw}");
    assert!(raw.contains("filename=\"vol-1.epub\""), "{raw}");
    assert!(
        raw.contains("EPUB-BYTES") || raw.contains("RVBVQi1CWVRFUw=="),
        "attachment body missing: {raw}"
    );
}

#[test]
fn attaches_pdfs_and_refuses_formats_amazon_cannot_take() {
    let smtp = parse(&settings()).unwrap();
    let pdf = book_message(&smtp, "Vol. 1", "Vol 1.PDF", b"%PDF".to_vec()).unwrap();
    assert!(String::from_utf8_lossy(&pdf.formatted()).contains("application/pdf"));
    let err = book_message(&smtp, "Vol. 1", "vol-1.cbz", b"PK".to_vec()).unwrap_err();
    assert!(err.to_string().contains("EPUB or PDF, not a CBZ"), "{err}");
}

#[test]
fn explains_common_smtp_replies() {
    let msg = |f| explain(f, "raw", "smtp.example.test", 587);
    assert!(msg(Failure::Reply(535)).contains("app password"));
    assert!(msg(Failure::Reply(552)).contains("too large"));
    assert!(msg(Failure::Reply(421)).contains("try again later"));
    assert!(msg(Failure::Connection).contains("smtp.example.test:587"));
    assert!(msg(Failure::Tls).contains("465 uses TLS"));
}

#[test]
fn refuses_to_send_a_password_without_tls_except_to_this_machine() {
    let plain = EmailSettings {
        security: Security::None,
        ..settings()
    };
    let err = parse(&plain).unwrap_err().to_string();
    assert!(err.contains("without TLS"), "{err}");
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        let local = EmailSettings {
            host: host.into(),
            ..plain.clone()
        };
        assert!(parse(&local).is_ok(), "{host}");
    }
    let anonymous = EmailSettings {
        username: String::new(),
        from: "me@example.test".into(),
        ..plain
    };
    assert!(parse(&anonymous).is_ok());
}
