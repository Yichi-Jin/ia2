//! Subcommand implementations. `quartet` is the generic resource
//! surface (`ls/get/set/rm`), `api` the raw escape hatch; the rest are
//! the domain verbs that keep their own semantics.

pub(crate) mod actions;
pub(crate) mod agent;
pub(crate) mod analysis;
pub(crate) mod api;
pub(crate) mod edge;
pub(crate) mod project;
pub(crate) mod quartet;
pub(crate) mod runtime;
pub(crate) mod sim;
