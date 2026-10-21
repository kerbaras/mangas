//! An error with its causes on one line, each said once.

/// `e` and its sources joined by `: `, like the CLI's error line: a cause the text before it
/// already ends with is skipped, so "refused: Connection refused" never doubles.
pub fn with_causes(e: &dyn std::error::Error) -> String {
    let mut line = e.to_string();
    let mut parent = line.clone();
    let mut source = e.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !parent.ends_with(&text) {
            line.push_str(": ");
            line.push_str(&text);
        }
        parent = text;
        source = cause.source();
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("{text}")]
    struct Layer {
        text: &'static str,
        #[source]
        source: Option<Box<Layer>>,
    }

    fn chain(texts: &[&'static str]) -> Layer {
        let mut iter = texts.iter().rev();
        let mut layer = Layer {
            text: iter.next().copied().unwrap_or(""),
            source: None,
        };
        for text in iter {
            layer = Layer {
                text,
                source: Some(Box::new(layer)),
            };
        }
        layer
    }

    #[test]
    fn joins_every_cause_once() {
        let e = chain(&[
            "sending request",
            "client error (Connect)",
            "Connection refused",
        ]);
        assert_eq!(
            with_causes(&e),
            "sending request: client error (Connect): Connection refused"
        );
        let repeated = chain(&["dns error: no such host", "no such host"]);
        assert_eq!(with_causes(&repeated), "dns error: no such host");
        assert_eq!(with_causes(&chain(&["alone"])), "alone");
    }
}
