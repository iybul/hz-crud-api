// This lib.rs file re-exports parts of main.rs for testing purposes
pub use crud_hz_api_main::*;

#[doc(hidden)]
pub mod crud_hz_api_main {
    include!("main.rs");
}