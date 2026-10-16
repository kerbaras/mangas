use mangas_core::library::{ProviderSession, Settings};
use mangas_core::sources::Header;

/// Browser session a provider behind a verification wall must present: the cookies and the
/// user agent that obtained them (Cloudflare binds `cf_clearance` to the user agent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub cookies: String,
    pub user_agent: String,
}

impl Session {
    /// The session stored for `provider` in the app settings, if any.
    pub fn from_settings(settings: &Settings, provider: &str) -> Option<Self> {
        settings
            .provider_sessions
            .get(provider)
            .filter(|s| !s.cookies.is_empty())
            .map(Self::from)
    }

    /// Request headers carrying the session.
    pub fn headers(&self) -> Vec<Header> {
        let mut headers = vec![("cookie".to_string(), self.cookies.clone())];
        if !self.user_agent.is_empty() {
            headers.push(("user-agent".to_string(), self.user_agent.clone()));
        }
        headers
    }
}

impl From<&ProviderSession> for Session {
    fn from(s: &ProviderSession) -> Self {
        Self {
            cookies: s.cookies.clone(),
            user_agent: s.user_agent.clone(),
        }
    }
}
