//! Provider clients reach the network through [`mangas_core::net`].

pub(crate) use mangas_core::net::{apply, proxied, transport_error};
pub use mangas_core::net::{effective, override_network, shown};
