use std::{fmt::Display, str::FromStr};

use serde_with::{DeserializeFromStr, SerializeDisplay};

/// The invalid part of a package name
#[derive(Debug)]
pub enum ErrorReason {
    /// The scope of the package name is invalid
    Scope,
    /// The name of the package name is invalid
    Name,
}

impl Display for ErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorReason::Scope => write!(f, "scope"),
            ErrorReason::Name => write!(f, "name"),
        }
    }
}

/// A pesde package name
#[derive(
    Debug, DeserializeFromStr, SerializeDisplay, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct PackageName(String, String);

impl FromStr for PackageName {
    type Err = errors::PackageNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (scope, name) = s
            .split_once('/')
            .ok_or(Self::Err::InvalidFormat(s.to_string()))?;

        for (reason, part) in [(ErrorReason::Scope, scope), (ErrorReason::Name, name)] {
            if part.len() < 3 || part.len() > 32 {
                return Err(Self::Err::InvalidLength(reason, part.to_string()));
            }

            if part.chars().all(|c| c.is_ascii_digit()) {
                return Err(Self::Err::OnlyDigits(reason, part.to_string()));
            }

            if part.starts_with('_') || part.ends_with('_') {
                return Err(Self::Err::PrePostfixUnderscore(reason, part.to_string()));
            }

            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(Self::Err::InvalidCharacters(reason, part.to_string()));
            }
        }

        Ok(Self(scope.to_string(), name.to_string()))
    }
}

impl Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl PackageName {
    /// Returns the parts of the package name
    pub fn as_str(&self) -> (&str, &str) {
        (&self.0, &self.1)
    }

    /// Returns the package name as a string suitable for use in the filesystem
    pub fn escaped(&self) -> String {
        format!("{}+{}", self.0, self.1)
    }
}

/// All possible package names
#[derive(
    Debug, DeserializeFromStr, SerializeDisplay, Clone, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum PackageNames {
    /// A pesde package name
    Pesde(PackageName),
    /// A Wally package name
    #[cfg(feature = "wally-compat")]
    Wally(wally::WallyPackageName),
}

impl PackageNames {
    /// Returns the parts of the package name
    pub fn as_str(&self) -> (&str, &str) {
        match self {
            PackageNames::Pesde(name) => name.as_str(),
            #[cfg(feature = "wally-compat")]
            PackageNames::Wally(name) => name.as_str(),
        }
    }

    /// Returns the package name as a string suitable for use in the filesystem
    pub fn escaped(&self) -> String {
        match self {
            PackageNames::Pesde(name) => name.escaped(),
            #[cfg(feature = "wally-compat")]
            PackageNames::Wally(name) => name.escaped(),
        }
    }

    /// The reverse of `escaped`
    pub fn from_escaped(s: &str) -> Result<Self, errors::PackageNamesError> {
        PackageNames::from_str(s.replacen('+', "/", 1).as_str())
    }
}

impl Display for PackageNames {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageNames::Pesde(name) => write!(f, "{name}"),
            #[cfg(feature = "wally-compat")]
            PackageNames::Wally(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for PackageNames {
    type Err = errors::PackageNamesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(feature = "wally-compat")]
        if let Some(wally_name) = s
            .strip_prefix("wally#")
            .or_else(|| if s.contains('-') { Some(s) } else { None })
            .and_then(|s| wally::WallyPackageName::from_str(s).ok())
        {
            return Ok(PackageNames::Wally(wally_name));
        }

        if let Ok(name) = PackageName::from_str(s) {
            Ok(PackageNames::Pesde(name))
        } else {
            Err(errors::PackageNamesError::InvalidPackageName(s.to_string()))
        }
    }
}

/// Wally package names
#[cfg(feature = "wally-compat")]
pub mod wally {
    use std::{fmt::Display, str::FromStr};

    use serde_with::{DeserializeFromStr, SerializeDisplay};

    use crate::names::{errors, ErrorReason};

    /// A Wally package name
    #[derive(
        Debug, DeserializeFromStr, SerializeDisplay, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
    )]
    pub struct WallyPackageName(String, String);

    impl FromStr for WallyPackageName {
        type Err = errors::WallyPackageNameError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (scope, name) = s
                .strip_prefix("wally#")
                .unwrap_or(s)
                .split_once('/')
                .ok_or(Self::Err::InvalidFormat(s.to_string()))?;

            for (reason, part) in [(ErrorReason::Scope, scope), (ErrorReason::Name, name)] {
                if part.is_empty() || part.len() > 64 {
                    return Err(Self::Err::InvalidLength(reason, part.to_string()));
                }

                if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(Self::Err::InvalidCharacters(reason, part.to_string()));
                }
            }

            Ok(Self(scope.to_string(), name.to_string()))
        }
    }

    impl Display for WallyPackageName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "wally#{}/{}", self.0, self.1)
        }
    }

    impl WallyPackageName {
        /// Returns the parts of the package name
        pub fn as_str(&self) -> (&str, &str) {
            (&self.0, &self.1)
        }

        /// Returns the package name as a string suitable for use in the filesystem
        pub fn escaped(&self) -> String {
            format!("wally#{}+{}", self.0, self.1)
        }
    }
}

/// Errors that can occur when working with package names
pub mod errors {
    use thiserror::Error;

    use crate::names::ErrorReason;

    /// Errors that can occur when working with pesde package names
    #[derive(Debug, Error)]
    pub enum PackageNameError {
        /// The package name is not in the format `scope/name`
        #[error("package name `{0}` is not in the format `scope/name`")]
        InvalidFormat(String),

        /// The package name is outside the allowed characters: a-z, 0-9, and _
        #[error("package {0} `{1}` contains characters outside a-z, 0-9, and _")]
        InvalidCharacters(ErrorReason, String),

        /// The package name contains only digits
        #[error("package {0} `{1}` contains only digits")]
        OnlyDigits(ErrorReason, String),

        /// The package name starts or ends with an underscore
        #[error("package {0} `{1}` starts or ends with an underscore")]
        PrePostfixUnderscore(ErrorReason, String),

        /// The package name is not within 3-32 characters long
        #[error("package {0} `{1}` is not within 3-32 characters long")]
        InvalidLength(ErrorReason, String),
    }

    /// Errors that can occur when working with Wally package names
    #[cfg(feature = "wally-compat")]
    #[derive(Debug, Error)]
    pub enum WallyPackageNameError {
        /// The package name is not in the format `scope/name`
        #[error("wally package name `{0}` is not in the format `scope/name`")]
        InvalidFormat(String),

        /// The package name is outside the allowed characters: a-z, 0-9, and -
        #[error("wally package {0} `{1}` contains characters outside a-z, 0-9, and -")]
        InvalidCharacters(ErrorReason, String),

        /// The package name is not within 1-64 characters long
        #[error("wally package {0} `{1}` is not within 1-64 characters long")]
        InvalidLength(ErrorReason, String),
    }

    /// Errors that can occur when working with package names
    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum PackageNamesError {
        /// The package name is invalid
        #[error("invalid package name {0}")]
        InvalidPackageName(String),
    }
}
