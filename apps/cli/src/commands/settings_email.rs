//! `mangas settings email`: show or change the Send to Kindle email settings.

use clap::Args as ClapArgs;
use mangas_core::library;
use mangas_core::send::password::{self, Storage};
use mangas_core::send::{EmailSettings, Security};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct EmailArgs {
    /// Send to Kindle address (name@kindle.com)
    #[arg(long)]
    to: Option<String>,
    /// Sender address approved in your Amazon account [default: --username]
    #[arg(long)]
    from: Option<String>,
    /// SMTP server
    #[arg(long)]
    host: Option<String>,
    /// SMTP port; 0 resets to the usual one for --security
    #[arg(long)]
    port: Option<u16>,
    #[arg(long, value_enum)]
    security: Option<SecurityArg>,
    #[arg(long)]
    username: Option<String>,
    /// Read the SMTP password from the first line of stdin (kept in the system keychain, or
    /// unencrypted in the app settings when there is none)
    #[arg(long)]
    password_stdin: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum SecurityArg {
    Starttls,
    Tls,
    None,
}

impl From<SecurityArg> for Security {
    fn from(s: SecurityArg) -> Self {
        match s {
            SecurityArg::Starttls => Security::Starttls,
            SecurityArg::Tls => Security::Tls,
            SecurityArg::None => Security::None,
        }
    }
}

pub fn run(args: EmailArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let store = password::system_store();
    let mut settings = password::load_settings(&lib, &*store)?;
    let before = settings.clone();
    let typed = if args.password_stdin {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Some(line.trim_end_matches(['\r', '\n']).to_string())
    } else {
        None
    };
    apply(&mut settings.kindle_email, args, typed);
    if settings != before {
        let saved = password::save_settings(&lib, &*store, Some(&before), &settings)?;
        if let Some(why) = saved.fallback {
            warn_unencrypted(&why);
        }
        settings = saved.settings;
    }
    print(&settings.kindle_email, password::storage(&lib, &*store)?)
}

/// Says once per run that the password sits unencrypted in the app settings, and why.
pub fn warn_unencrypted(why: &str) {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
        eprintln!("warning: the SMTP password is stored unencrypted in the app settings ({why})");
    });
}

fn apply(e: &mut EmailSettings, args: EmailArgs, password: Option<String>) {
    let set = |field: &mut String, value: Option<String>| {
        if let Some(v) = value {
            *field = v.trim().to_string();
        }
    };
    set(&mut e.to, args.to);
    set(&mut e.from, args.from);
    set(&mut e.host, args.host);
    set(&mut e.username, args.username);
    if let Some(s) = args.security {
        e.security = s.into();
    }
    if let Some(port) = args.port {
        e.port = (port != 0).then_some(port);
    }
    if let Some(p) = password {
        e.password = p;
    }
}

fn print(e: &EmailSettings, storage: Storage) -> anyhow::Result<()> {
    let port = e.port.unwrap_or(e.security.default_port());
    let security = serde_json::to_value(e.security)?;
    outln!("to        {}", e.to)?;
    outln!(
        "from      {}",
        if e.from.is_empty() {
            &e.username
        } else {
            &e.from
        }
    )?;
    outln!(
        "server    {}:{port} ({})",
        e.host,
        security.as_str().unwrap_or("")
    )?;
    outln!("username  {}", e.username)?;
    outln!(
        "password  {}",
        match (e.password.is_empty(), storage) {
            (true, _) => "(not set)",
            (false, Storage::Keychain) => "(set, in the system keychain)",
            (false, Storage::File) => "(set, unencrypted in the app settings)",
        }
    )?;
    Ok(())
}
