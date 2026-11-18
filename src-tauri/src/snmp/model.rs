use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpFeatures {
    versions: Vec<&'static str>,
    operations: Vec<&'static str>,
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SnmpVersion {
    V1,
    V2c,
    V3,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SecurityLevel {
    #[serde(rename = "noAuthNoPriv")]
    None,
    #[serde(rename = "authNoPriv")]
    Auth,
    #[serde(rename = "authPriv")]
    AuthWithPrivacy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Credentials {
    Community {
        community: SecretString,
    },
    User {
        username: String,
        security_level: SecurityLevel,
        auth_password: Option<SecretString>,
        privacy_password: Option<SecretString>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_snmp_port")]
    pub port: u16,
    pub version: SnmpVersion,
    pub credentials: Credentials,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_retries")]
    pub retries: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedProfile {
    pub name: String,
    pub endpoint: String,
    pub version: SnmpVersion,
    pub security_level: Option<SecurityLevel>,
    pub timeout_ms: u64,
    pub retries: u8,
}

#[derive(Debug, Error, PartialEq)]
pub enum ProfileValidationError {
    #[error("profile name is required")]
    MissingName,
    #[error("target host is required")]
    MissingHost,
    #[error("target host must not include a URL scheme or path")]
    InvalidHost,
    #[error("target port must be between 1 and 65535")]
    InvalidPort,
    #[error("timeout must be between 100 and 120000 milliseconds")]
    InvalidTimeout,
    #[error("retries must be between 0 and 10")]
    InvalidRetries,
    #[error("SNMP v1/v2c requires a community")]
    MissingCommunity,
    #[error("SNMP v1/v2c requires community credentials")]
    IncompatibleCommunityCredentials,
    #[error("SNMP v3 requires user credentials")]
    IncompatibleUserCredentials,
    #[error("SNMP v3 username is required")]
    MissingUsername,
    #[error("the selected SNMP v3 security level requires an authentication password")]
    MissingAuthPassword,
    #[error("authPriv requires a privacy password")]
    MissingPrivacyPassword,
}

impl ProfileInput {
    pub fn validate(self) -> Result<ValidatedProfile, ProfileValidationError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ProfileValidationError::MissingName);
        }

        let host = self.host.trim();
        if host.is_empty() {
            return Err(ProfileValidationError::MissingHost);
        }
        if host.contains("://") || host.contains('/') || host.chars().any(char::is_whitespace) {
            return Err(ProfileValidationError::InvalidHost);
        }
        if self.port == 0 {
            return Err(ProfileValidationError::InvalidPort);
        }
        if !(100..=120_000).contains(&self.timeout_ms) {
            return Err(ProfileValidationError::InvalidTimeout);
        }
        if self.retries > 10 {
            return Err(ProfileValidationError::InvalidRetries);
        }

        let security_level = match (self.version, &self.credentials) {
            (SnmpVersion::V1 | SnmpVersion::V2c, Credentials::Community { community }) => {
                if community.is_empty() {
                    return Err(ProfileValidationError::MissingCommunity);
                }
                None
            }
            (SnmpVersion::V1 | SnmpVersion::V2c, Credentials::User { .. }) => {
                return Err(ProfileValidationError::IncompatibleCommunityCredentials);
            }
            (
                SnmpVersion::V3,
                Credentials::User {
                    username,
                    security_level,
                    auth_password,
                    privacy_password,
                },
            ) => {
                if username.trim().is_empty() {
                    return Err(ProfileValidationError::MissingUsername);
                }
                if matches!(
                    security_level,
                    SecurityLevel::Auth | SecurityLevel::AuthWithPrivacy
                ) && auth_password.as_ref().is_none_or(SecretString::is_empty)
                {
                    return Err(ProfileValidationError::MissingAuthPassword);
                }
                if matches!(security_level, SecurityLevel::AuthWithPrivacy)
                    && privacy_password.as_ref().is_none_or(SecretString::is_empty)
                {
                    return Err(ProfileValidationError::MissingPrivacyPassword);
                }
                Some(*security_level)
            }
            (SnmpVersion::V3, Credentials::Community { .. }) => {
                return Err(ProfileValidationError::IncompatibleUserCredentials);
            }
        };

        let endpoint = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]:{}", self.port)
        } else {
            format!("{host}:{}", self.port)
        };

        Ok(ValidatedProfile {
            name: name.to_owned(),
            endpoint,
            version: self.version,
            security_level,
            timeout_ms: self.timeout_ms,
            retries: self.retries,
        })
    }
}

const fn default_snmp_port() -> u16 {
    161
}

const fn default_timeout_ms() -> u64 {
    2_000
}

const fn default_retries() -> u8 {
    1
}

impl SnmpFeatures {
    pub fn planned_mvp() -> Self {
        Self {
            versions: vec!["v1", "v2c", "v3"],
            operations: vec!["get", "getNext", "walk", "bulkWalk", "set"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2_profile(community: &str) -> ProfileInput {
        ProfileInput {
            name: "Lab switch".to_owned(),
            host: "192.0.2.10".to_owned(),
            port: 161,
            version: SnmpVersion::V2c,
            credentials: Credentials::Community {
                community: SecretString(community.to_owned()),
            },
            timeout_ms: 2_000,
            retries: 1,
        }
    }

    #[test]
    fn validates_v2_profile_without_exposing_community() {
        let profile = v2_profile("not-for-logs");
        let debug = format!("{profile:?}");
        assert!(!debug.contains("not-for-logs"));

        let validated = profile.validate().expect("profile should be valid");
        assert_eq!(validated.endpoint, "192.0.2.10:161");
    }

    #[test]
    fn rejects_empty_community() {
        assert_eq!(
            v2_profile(" ").validate().unwrap_err(),
            ProfileValidationError::MissingCommunity
        );
    }

    #[test]
    fn requires_both_passwords_for_auth_priv() {
        let profile = ProfileInput {
            name: "Core router".to_owned(),
            host: "2001:db8::10".to_owned(),
            port: 161,
            version: SnmpVersion::V3,
            credentials: Credentials::User {
                username: "operator".to_owned(),
                security_level: SecurityLevel::AuthWithPrivacy,
                auth_password: Some(SecretString("auth-password".to_owned())),
                privacy_password: None,
            },
            timeout_ms: 2_000,
            retries: 1,
        };

        assert_eq!(
            profile.validate().unwrap_err(),
            ProfileValidationError::MissingPrivacyPassword
        );
    }

    #[test]
    fn formats_ipv6_endpoint() {
        let profile = ProfileInput {
            host: "2001:db8::10".to_owned(),
            ..v2_profile("public")
        };

        assert_eq!(profile.validate().unwrap().endpoint, "[2001:db8::10]:161");
    }
}
