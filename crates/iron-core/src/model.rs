//! The typed project model. `validate::check` builds it while validating.

use iron_domain::alarm::AlarmDef;
use iron_domain::expr::Expr;
use iron_domain::TagType;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Tag {
    pub ty: TagType,
    pub derived: Option<Expr>,
    pub alarms: Vec<(String, AlarmDef)>,
}

#[derive(Debug, Clone, Default)]
pub struct Project {
    pub tags: BTreeMap<String, Tag>,
    /// Each derived tag after everything it depends on.
    pub derived_order: Vec<String>,
}
