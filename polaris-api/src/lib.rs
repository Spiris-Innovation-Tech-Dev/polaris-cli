#[allow(unused_imports, clippy::all)]
pub mod generated {
    pub mod issue_query_v1 {
        include!(concat!(env!("OUT_DIR"), "/issue_query_v1.rs"));
    }
    pub mod triage_command_v1 {
        include!(concat!(env!("OUT_DIR"), "/triage_command_v1.rs"));
    }
    pub mod triage_query_v1 {
        include!(concat!(env!("OUT_DIR"), "/triage_query_v1.rs"));
    }
}

pub mod auth;
pub mod common;
pub mod client;
pub mod error;

