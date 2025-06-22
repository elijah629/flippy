use anyhow::{Error, anyhow};
use cliclack::Validate;
use url::Url;

pub struct URLValidator;

impl Validate<String> for URLValidator {
    type Err = Error;

    fn validate(&self, input: &String) -> Result<(), Self::Err> {
        Ok(Url::parse(input).map(|_| ())?)
    }
}

pub struct ProjectNameValidator;

impl Validate<String> for ProjectNameValidator {
    type Err = Error;

    fn validate(&self, input: &String) -> Result<(), Self::Err> {
        validate_project_name(input)
    }
}

pub fn validate_project_name(input: &str) -> anyhow::Result<()> {
    if input.is_empty() {
        return Err(anyhow!("Project name cannot be empty"));
    }

    if input.len() > 64 {
        return Err(anyhow!("Project name must be at most 64 characters long"));
    }

    let mut chars = input.chars();

    // First character must be a letter
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return Err(anyhow!("Project name must start with a letter (a-z, A-Z)")),
    }

    // Remaining characters must be valid
    if !input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Project name must only contain letters, numbers, '-', or '_'"
        ));
    }

    Ok(())
}
