//! Registry metadata lookups for individual packages.
//!
//! * [`npm`] — npm registry metadata: maintainers, publish times,
//!   repository, latest version (#94).
//! * [`crates`] — crates.io registry metadata: yanked versions and
//!   license information (#178, #179).
//! * [`confusion`] — private package names that also exist on the public
//!   registry (#332).

pub mod confusion;
pub mod crates;
pub mod npm;