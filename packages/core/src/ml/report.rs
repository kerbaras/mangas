//! What an export says about the detectors: used, skipped (asked for but couldn't run) or off
//! (not asked for). The CLI's human lines and its `--json` report both come from here.

use serde::Serialize;

use super::Penalizer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorState {
    Used,
    Skipped,
    Off,
}

/// The detectors' part of an export report: `{"state": "used"|"skipped"|"off", "reason": ...}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectorReport {
    pub state: DetectorState,
    /// Why they were skipped (the runtime's error); `None` unless skipped.
    pub reason: Option<String>,
}

impl DetectorReport {
    /// Detectors not asked for.
    pub fn off() -> Self {
        Self {
            state: DetectorState::Off,
            reason: None,
        }
    }

    /// Detectors asked for: used, unless `error` says why they couldn't run.
    pub fn requested(error: Option<&str>) -> Self {
        match error {
            Some(e) => Self {
                state: DetectorState::Skipped,
                reason: Some(e.to_string()),
            },
            None => Self {
                state: DetectorState::Used,
                reason: None,
            },
        }
    }

    /// The line an export prints about the detectors, if any. `provider` is the execution
    /// provider they run on ("cpu", "coreml", ...).
    pub fn message(&self, provider: &str) -> Option<String> {
        match (&self.state, &self.reason) {
            (DetectorState::Used, _) => Some(format!("detectors: {provider}")),
            (DetectorState::Skipped, Some(e)) => {
                Some(format!("detectors off, cutting without them: {e}"))
            }
            _ => None,
        }
    }
}

impl Penalizer {
    /// The report for detectors that were asked for and set up as this.
    pub fn report(&self) -> DetectorReport {
        DetectorReport::requested(self.error.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_state_serialises_and_prints_as_the_cli_shows_it() {
        let json = |r: &DetectorReport| serde_json::to_string(r).unwrap();
        let skipped = DetectorReport::requested(Some("dlopen failed"));
        assert_eq!(
            json(&skipped),
            r#"{"state":"skipped","reason":"dlopen failed"}"#
        );
        assert_eq!(
            skipped.message("cpu").unwrap(),
            "detectors off, cutting without them: dlopen failed"
        );
        let used = DetectorReport::requested(None);
        assert_eq!(json(&used), r#"{"state":"used","reason":null}"#);
        assert_eq!(used.message("cpu").unwrap(), "detectors: cpu");
        assert_eq!(
            json(&DetectorReport::off()),
            r#"{"state":"off","reason":null}"#
        );
        assert_eq!(DetectorReport::off().message("cpu"), None);
    }
}
