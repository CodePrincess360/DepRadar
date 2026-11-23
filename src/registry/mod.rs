//! Registry metadata lookups for individual packages.
//!
//! * [`npm`] — npm registry metadata: maintainers, publish times,
//!   repository, latest version (#94).
//! * [`crates`] — crates.io registry metadata: yanked versions and
//!   license information (#178, #179).

pub mod crates;
pub mod npm;

use crate::model::Ecosystem;
use crate::pipeline::AdvisoryError;

/// Registry metadata about a deprecated or yanked version.
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    pub message: Option<String>,
    pub yanked: bool,
}

/// Metadata source used by the lockfile scanner.
pub trait RegistrySource {
    fn deprecation(
        &self,
        ecosystem: Ecosystem,
        name: &str,
        version: &str,
    ) -> Result<Option<DeprecationInfo>, AdvisoryError>;
}
