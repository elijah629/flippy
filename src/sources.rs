use std::{fmt, path::PathBuf, str::FromStr};

use url::Url;

const PATH_SCHEME_PREFIX: &str = "path://";
const FILE_SCHEME: &str = "file";
const FIRMWARE_REMOTE_SCHEMES: &[&str] = &["http", "https"];
const REPOSITORY_REMOTE_SCHEMES: &[&str] = &["http", "https", "ssh", "git"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FirmwareSource {
    Preset(FirmwarePreset),
    Remote(Url),
    Local(PathBuf),
}

impl Default for FirmwareSource {
    fn default() -> Self {
        Self::Preset(FirmwarePreset::default())
    }
}

impl FromStr for FirmwareSource {
    type Err = ParseSourceError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseSourceError::empty("firmware source"));
        }

        if let Ok(preset) = FirmwarePreset::from_str(input) {
            return Ok(Self::Preset(preset));
        }

        parse_location(input, FIRMWARE_REMOTE_SCHEMES, "firmware source").map(|location| {
            match location {
                SourceLocation::Remote(url) => Self::Remote(url),
                SourceLocation::Local(path) => Self::Local(path),
            }
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FirmwarePreset {
    #[default]
    Ofw,
    Unleashed,
    Momentum,
    RogueFree,
}

impl FromStr for FirmwarePreset {
    type Err = ParseSourceError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_ascii_lowercase().as_str() {
            "ofw" => Ok(Self::Ofw),
            "unleashed" => Ok(Self::Unleashed),
            "momentum" => Ok(Self::Momentum),
            "rogue-free" => Ok(Self::RogueFree),
            _ => Err(ParseSourceError::invalid_preset(input)),
        }
    }
}

impl fmt::Display for FirmwarePreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Ofw => "ofw",
            Self::Unleashed => "unleashed",
            Self::Momentum => "momentum",
            Self::RogueFree => "rogue-free",
        })
    }
}

impl fmt::Display for FirmwareSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preset(preset) => preset.fmt(formatter),
            Self::Remote(url) => formatter.write_str(url.as_str()),
            Self::Local(path) => write!(formatter, "{PATH_SCHEME_PREFIX}{}", path.display()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositorySource {
    Remote(RepositoryRemote),
    Local(PathBuf),
}

impl FromStr for RepositorySource {
    type Err = ParseSourceError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseSourceError::empty("repository source"));
        }

        if looks_like_scp_remote(input) {
            return Ok(Self::Remote(RepositoryRemote::ScpLike(input.to_owned())));
        }

        parse_location(input, REPOSITORY_REMOTE_SCHEMES, "repository source").map(|location| {
            match location {
                SourceLocation::Remote(url) => Self::Remote(RepositoryRemote::Url(url)),
                SourceLocation::Local(path) => Self::Local(path),
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryRemote {
    Url(Url),
    ScpLike(String),
}

impl fmt::Display for RepositoryRemote {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(url) => formatter.write_str(url.as_str()),
            Self::ScpLike(value) => formatter.write_str(value),
        }
    }
}

impl fmt::Display for RepositorySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Remote(remote) => remote.fmt(formatter),
            Self::Local(path) => write!(formatter, "{PATH_SCHEME_PREFIX}{}", path.display()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseSourceError {
    EmptyInput {
        kind: &'static str,
    },
    EmptyPath {
        kind: &'static str,
    },
    InvalidPreset {
        input: String,
    },
    UnsupportedScheme {
        kind: &'static str,
        scheme: String,
    },
    InvalidFileUrl {
        kind: &'static str,
        input: String,
    },
    InvalidUrl {
        kind: &'static str,
        input: String,
        message: String,
    },
}

impl ParseSourceError {
    fn empty(kind: &'static str) -> Self {
        Self::EmptyInput { kind }
    }

    fn empty_path(kind: &'static str) -> Self {
        Self::EmptyPath { kind }
    }

    fn invalid_preset(input: &str) -> Self {
        Self::InvalidPreset {
            input: input.to_owned(),
        }
    }

    fn unsupported_scheme(kind: &'static str, scheme: &str) -> Self {
        Self::UnsupportedScheme {
            kind,
            scheme: scheme.to_owned(),
        }
    }

    fn invalid_file_url(kind: &'static str, input: &str) -> Self {
        Self::InvalidFileUrl {
            kind,
            input: input.to_owned(),
        }
    }

    fn invalid_url(kind: &'static str, input: &str, message: impl Into<String>) -> Self {
        Self::InvalidUrl {
            kind,
            input: input.to_owned(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput { kind } => write!(formatter, "{kind} cannot be empty"),
            Self::EmptyPath { kind } => write!(formatter, "{kind} path cannot be empty"),
            Self::InvalidPreset { input } => write!(
                formatter,
                "unknown firmware preset '{input}'; expected one of: ofw, unleashed, momentum, rogue-free"
            ),
            Self::UnsupportedScheme { kind, scheme } => {
                write!(formatter, "unsupported {kind} scheme '{scheme}'")
            }
            Self::InvalidFileUrl { kind, input } => {
                write!(formatter, "invalid {kind} file URL '{input}'")
            }
            Self::InvalidUrl {
                kind,
                input,
                message,
            } => write!(formatter, "invalid {kind} '{input}': {message}"),
        }
    }
}

impl std::error::Error for ParseSourceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SourceLocation {
    Remote(Url),
    Local(PathBuf),
}

fn parse_location(
    input: &str,
    remote_schemes: &[&str],
    kind: &'static str,
) -> Result<SourceLocation, ParseSourceError> {
    if let Some(path) = input.strip_prefix(PATH_SCHEME_PREFIX) {
        if path.is_empty() {
            return Err(ParseSourceError::empty_path(kind));
        }

        return Ok(SourceLocation::Local(PathBuf::from(path)));
    }

    match Url::parse(input) {
        Ok(url) if url.scheme() == FILE_SCHEME => {
            let path = url
                .to_file_path()
                .map_err(|_| ParseSourceError::invalid_file_url(kind, input))?;
            Ok(SourceLocation::Local(path))
        }
        Ok(url) if remote_schemes.contains(&url.scheme()) => Ok(SourceLocation::Remote(url)),
        Ok(url) => Err(ParseSourceError::unsupported_scheme(kind, url.scheme())),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(SourceLocation::Local(PathBuf::from(input)))
        }
        Err(error) => Err(ParseSourceError::invalid_url(
            kind,
            input,
            error.to_string(),
        )),
    }
}

fn looks_like_scp_remote(input: &str) -> bool {
    if input.contains("://")
        || input.starts_with('/')
        || input.starts_with("./")
        || input.starts_with("../")
        || input.starts_with("~/")
    {
        return false;
    }

    if is_windows_drive_path(input) {
        return false;
    }

    let Some((left, right)) = input.split_once(':') else {
        return false;
    };

    !left.is_empty() && !right.is_empty() && (left.contains('@') || left.contains('.'))
}

fn is_windows_drive_path(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        FirmwarePreset, FirmwareSource, ParseSourceError, RepositoryRemote, RepositorySource,
    };

    #[test]
    fn firmware_supports_relative_paths() {
        assert_eq!(
            "fw.tgz".parse::<FirmwareSource>().unwrap(),
            FirmwareSource::Local(PathBuf::from("fw.tgz"))
        );
    }

    #[test]
    fn firmware_supports_absolute_paths_via_path_scheme() {
        assert_eq!(
            "path:///tmp/fw.tgz".parse::<FirmwareSource>().unwrap(),
            FirmwareSource::Local(PathBuf::from("/tmp/fw.tgz"))
        );
    }

    #[test]
    fn firmware_supports_relative_paths_via_path_scheme() {
        assert_eq!(
            "path://../fw.tgz".parse::<FirmwareSource>().unwrap(),
            FirmwareSource::Local(PathBuf::from("../fw.tgz"))
        );
    }

    #[test]
    fn firmware_supports_file_urls() {
        assert_eq!(
            "file:///tmp/fw.tgz".parse::<FirmwareSource>().unwrap(),
            FirmwareSource::Local(PathBuf::from("/tmp/fw.tgz"))
        );
    }

    #[test]
    fn firmware_supports_remote_urls() {
        match "https://example.com/fw.tgz"
            .parse::<FirmwareSource>()
            .unwrap()
        {
            FirmwareSource::Remote(url) => assert_eq!(url.as_str(), "https://example.com/fw.tgz"),
            source => panic!("expected remote source, got {source:?}"),
        }
    }

    #[test]
    fn firmware_supports_rogue_free_preset() {
        assert_eq!(
            "rogue-free".parse::<FirmwareSource>().unwrap(),
            FirmwareSource::Preset(FirmwarePreset::RogueFree)
        );
    }

    #[test]
    fn repository_supports_local_paths() {
        assert_eq!(
            "../Flipper".parse::<RepositorySource>().unwrap(),
            RepositorySource::Local(PathBuf::from("../Flipper"))
        );
    }

    #[test]
    fn repository_supports_remote_urls() {
        match "https://github.com/Lucaslhm/Flipper-IRDB"
            .parse::<RepositorySource>()
            .unwrap()
        {
            RepositorySource::Remote(RepositoryRemote::Url(url)) => {
                assert_eq!(url.as_str(), "https://github.com/Lucaslhm/Flipper-IRDB")
            }
            source => panic!("expected remote source, got {source:?}"),
        }
    }

    #[test]
    fn repository_supports_scp_like_remotes() {
        assert_eq!(
            "git@github.com:Lucaslhm/Flipper-IRDB.git"
                .parse::<RepositorySource>()
                .unwrap(),
            RepositorySource::Remote(RepositoryRemote::ScpLike(
                "git@github.com:Lucaslhm/Flipper-IRDB.git".to_owned(),
            ))
        );
    }

    #[test]
    fn unsupported_firmware_scheme_returns_an_error() {
        assert_eq!(
            "ftp://example.com/fw.tgz"
                .parse::<FirmwareSource>()
                .unwrap_err(),
            ParseSourceError::UnsupportedScheme {
                kind: "firmware source",
                scheme: "ftp".to_owned(),
            }
        );
    }
}
