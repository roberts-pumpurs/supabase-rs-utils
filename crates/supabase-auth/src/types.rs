use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use simd_json::OwnedValue;
use typed_builder::TypedBuilder;

pub type UserMetadata = OwnedValue;
pub type AppMetadata = OwnedValue;

/// Login credentials for authentication.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct LoginCredentials {
    #[builder(setter(strip_option), default)]
    pub email: Option<String>,
    #[builder(setter(strip_option), default)]
    pub password: Option<String>,
    #[builder(setter(strip_option), default)]
    pub phone: Option<String>,
}

/// Token request body for the `/token` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct TokenRequestBody {
    #[builder(default)]
    pub email: Option<String>,
    #[builder(default)]
    pub phone: Option<String>,
    #[builder(default)]
    pub password: Option<String>,
    #[builder(setter(strip_option), default)]
    pub refresh_token: Option<String>,
    #[builder(setter(strip_option), default)]
    pub grant_type: Option<String>,
    #[builder(setter(strip_option), default)]
    pub gotrue_meta_security: Option<GoTrueMetaSecurity>,
    #[builder(setter(strip_option), default)]
    pub code: Option<String>,
    #[builder(setter(strip_option), default)]
    pub redirect_to: Option<String>,
    #[builder(setter(strip_option), default)]
    pub scope: Option<String>,
    #[builder(setter(strip_option), default)]
    pub client_id: Option<String>,
    #[builder(setter(strip_option), default)]
    pub client_secret: Option<String>,
    #[builder(setter(strip_option), default)]
    pub id_token: Option<String>,
    #[builder(setter(strip_option), default)]
    pub nonce: Option<String>,
    #[builder(setter(strip_option), default)]
    pub invite_token: Option<String>,
    #[builder(setter(strip_option), default)]
    pub provider_token: Option<String>,
    #[builder(setter(strip_option), default)]
    pub code_verifier: Option<String>,
}

/// Payload for the `/signup` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SignupPayload {
    #[builder(setter(strip_option), default)]
    pub email: Option<String>,
    #[builder(setter(strip_option), default)]
    pub password: Option<String>,
    #[builder(setter(strip_option), default)]
    pub phone: Option<String>,
    #[builder(setter(strip_option), default)]
    pub data: Option<UserMetadata>,
    #[builder(setter(strip_option), default)]
    pub gotrue_meta_security: Option<GoTrueMetaSecurity>,
    #[builder(setter(strip_option), default)]
    pub code_challenge: Option<String>,
    #[builder(setter(strip_option), default)]
    pub code_challenge_method: Option<String>,
}

/// Response from the `/signup` endpoint.
pub type SignupResponse = AccessTokenResponseSchema;

/// Response from the `/resend` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct ResendResponse {
    /// Unique ID of the message as reported by the SMS sending provider.
    #[builder(setter(strip_option), default)]
    pub message_id: Option<String>,
}

/// Response from the `/otp` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct OtpResponse {
    /// Unique ID of the message as reported by the SMS sending provider.
    #[builder(setter(strip_option), default)]
    pub message_id: Option<String>,
}

/// Response from the `/factors` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct FactorsResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub factor_type: MFAFactorType,
    #[builder(setter(strip_option), default)]
    pub totp: Option<TotpDetails>,
    #[builder(setter(strip_option), default)]
    pub phone: Option<String>,
}

/// Details for TOTP factor.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct TotpDetails {
    #[builder(setter(strip_option), default)]
    pub qr_code: Option<String>,
    #[builder(setter(strip_option), default)]
    pub secret: Option<String>,
    #[builder(setter(strip_option), default)]
    pub uri: Option<String>,
}

/// Response from the `/factors/{factorId}/challenge` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ChallengeResponse {
    TOTPPhone(TOTPPhoneChallengeResponse),
    WebAuthn(WebAuthnChallengeResponse),
}

/// Response from the `/factors/{factorId}` DELETE endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct FactorDeleteResponse {
    pub id: String,
}

/// Response from the `/sso` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SsoResponse {
    pub url: String,
}

/// Response from the `/admin/users` GET endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AdminUsersResponse {
    #[builder(setter(strip_option), default)]
    pub aud: Option<String>, // Deprecated
    pub users: Vec<UserSchema>,
}

/// Data for updating a user's MFA factor.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct MFAFactorUpdateData {
    #[builder(setter(strip_option), default)]
    pub friendly_name: Option<String>,
    #[builder(setter(strip_option), default)]
    pub status: Option<MFAFactorStatus>,
    // Include additional fields as necessary.
}

/// Response from the `/admin/sso/providers` GET endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SsoProvidersResponse {
    pub items: Vec<SSOProviderSchema>,
}

/// Use this property to pass a CAPTCHA token only if you have enabled CAPTCHA protection.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct GoTrueMetaSecurity {
    /// The CAPTCHA token.
    #[serde(rename = "captcha_token")]
    #[builder(setter(strip_option), default)]
    pub captcha_token: Option<String>,
}

/// Error response schema.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder, thiserror::Error)]
pub struct ErrorSchema {
    /// Certain responses will contain this property with the provided values.
    ///
    /// Usually one of these:
    /// - `invalid_request`
    /// - `unauthorized_client`
    /// - `access_denied`
    /// - `server_error`
    /// - `temporarily_unavailable`
    /// - `unsupported_otp_type`
    #[serde(rename = "error")]
    #[builder(setter(strip_option), default)]
    pub error: Option<String>,

    /// Certain responses that have an `error` property may have this property which describes the
    /// error.
    #[serde(rename = "error_description")]
    #[builder(setter(strip_option), default)]
    pub error_description: Option<String>,

    /// The HTTP status code. Usually missing if `error` is present.
    #[serde(rename = "code")]
    #[builder(setter(strip_option), default)]
    pub code: Option<i32>,

    /// A basic message describing the problem with the request. Usually missing if `error` is
    /// present.
    #[serde(rename = "msg")]
    #[builder(setter(strip_option), default)]
    pub msg: Option<String>,

    /// Only returned on the `/signup` endpoint if the password used is too weak. Inspect the
    /// `reasons` and `msg` property to identify the causes.
    #[serde(rename = "weak_password")]
    #[builder(setter(strip_option), default)]
    pub weak_password: Option<WeakPassword>,
}

impl std::fmt::Display for ErrorSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Start with the main error if available
        if let Some(ref error) = self.error {
            write!(f, "Error: {}", error)?;
        }

        // Append the error description if available
        if let Some(ref description) = self.error_description {
            if self.error.is_some() {
                write!(f, " - Description: {}", description)?;
            } else {
                write!(f, "Description: {}", description)?;
            }
        }

        // Append the HTTP status code if available
        if let Some(code) = self.code {
            write!(f, " (HTTP Code: {})", code)?;
        }

        // Append the basic message if available
        if let Some(ref msg) = self.msg {
            write!(f, ". Message: {}", msg)?;
        }

        // Append weak password details if available
        if let Some(ref weak_password) = self.weak_password {
            write!(f, ". Weak Password: {:?}", weak_password)?;
        }

        Ok(())
    }
}

/// Details about why a password is considered weak.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct WeakPassword {
    /// The reasons why the password is weak.
    #[serde(rename = "reasons")]
    pub reasons: Vec<WeakPasswordReason>,
}

/// Reasons why a password is considered weak.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum WeakPasswordReason {
    Length,
    Characters,
    Pwned,
}

/// Object describing the user related to the issued access and refresh tokens.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct UserSchema {
    /// User ID.
    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,

    /// Deprecated.
    #[serde(rename = "aud")]
    #[builder(setter(strip_option), default)]
    pub aud: Option<String>,

    /// Role.
    #[serde(rename = "role")]
    #[builder(setter(strip_option), default)]
    pub role: Option<String>,

    /// User's primary contact email.
    #[serde(rename = "email")]
    #[builder(setter(strip_option), default)]
    pub email: Option<String>,

    /// Timestamp when the email was confirmed.
    #[serde(rename = "email_confirmed_at")]
    #[builder(setter(strip_option), default)]
    pub email_confirmed_at: Option<DateTime<Utc>>,

    /// User's primary contact phone number.
    #[serde(rename = "phone")]
    #[builder(setter(strip_option), default)]
    pub phone: Option<String>,

    /// Timestamp when the phone number was confirmed.
    #[serde(rename = "phone_confirmed_at")]
    #[builder(setter(strip_option), default)]
    pub phone_confirmed_at: Option<DateTime<Utc>>,

    /// Timestamp when the confirmation was sent.
    #[serde(rename = "confirmation_sent_at")]
    #[builder(setter(strip_option), default)]
    pub confirmation_sent_at: Option<DateTime<Utc>>,

    /// Timestamp when the user was confirmed.
    #[serde(rename = "confirmed_at")]
    #[builder(setter(strip_option), default)]
    pub confirmed_at: Option<DateTime<Utc>>,

    /// Timestamp when the recovery email was sent.
    #[serde(rename = "recovery_sent_at")]
    #[builder(setter(strip_option), default)]
    pub recovery_sent_at: Option<DateTime<Utc>>,

    /// New email address if the user is changing it.
    #[serde(rename = "new_email")]
    #[builder(setter(strip_option), default)]
    pub new_email: Option<String>,

    /// Timestamp when the email change was sent.
    #[serde(rename = "email_change_sent_at")]
    #[builder(setter(strip_option), default)]
    pub email_change_sent_at: Option<DateTime<Utc>>,

    /// New phone number if the user is changing it.
    #[serde(rename = "new_phone")]
    #[builder(setter(strip_option), default)]
    pub new_phone: Option<String>,

    /// Timestamp when the phone change was sent.
    #[serde(rename = "phone_change_sent_at")]
    #[builder(setter(strip_option), default)]
    pub phone_change_sent_at: Option<DateTime<Utc>>,

    /// Timestamp when reauthentication was sent.
    #[serde(rename = "reauthentication_sent_at")]
    #[builder(setter(strip_option), default)]
    pub reauthentication_sent_at: Option<DateTime<Utc>>,

    /// Timestamp of the last sign-in.
    #[serde(rename = "last_sign_in_at")]
    #[builder(setter(strip_option), default)]
    pub last_sign_in_at: Option<DateTime<Utc>>,

    /// Application-specific metadata.
    #[serde(rename = "app_metadata")]
    #[builder(setter(strip_option), default)]
    pub app_metadata: Option<OwnedValue>,

    /// User-specific metadata.
    #[serde(rename = "user_metadata")]
    #[builder(setter(strip_option), default)]
    pub user_metadata: Option<OwnedValue>,

    /// Multi-factor authentication factors.
    #[serde(rename = "factors")]
    #[builder(setter(strip_option), default)]
    pub factors: Option<Vec<MFAFactorSchema>>,

    /// External identities linked to the user.
    #[serde(rename = "identities")]
    #[builder(setter(strip_option), default)]
    pub identities: Option<Vec<IdentitySchema>>,

    /// Timestamp until which the user is banned.
    #[serde(rename = "banned_until")]
    #[builder(setter(strip_option), default)]
    pub banned_until: Option<DateTime<Utc>>,

    /// Timestamp when the user was created.
    #[serde(rename = "created_at")]
    #[builder(setter(strip_option), default)]
    pub created_at: Option<DateTime<Utc>>,

    /// Timestamp when the user was last updated.
    #[serde(rename = "updated_at")]
    #[builder(setter(strip_option), default)]
    pub updated_at: Option<DateTime<Utc>>,

    /// Timestamp when the user was deleted.
    #[serde(rename = "deleted_at")]
    #[builder(setter(strip_option), default)]
    pub deleted_at: Option<DateTime<Utc>>,

    /// Indicates if the user is anonymous.
    #[serde(rename = "is_anonymous")]
    #[builder(setter(strip_option), default)]
    pub is_anonymous: Option<bool>,
}

/// Schema for SAML attribute mapping.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SAMLAttributeMappingSchema {
    /// Mapping of SAML attributes.
    #[serde(rename = "keys")]
    #[builder(setter(strip_option), default)]
    pub keys: Option<HashMap<String, SAMLAttributeMappingKey>>,
}

/// Key-value pairs for SAML attribute mapping.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SAMLAttributeMappingKey {
    #[serde(rename = "name")]
    #[builder(setter(strip_option), default)]
    pub name: Option<String>,
    #[serde(rename = "names")]
    #[builder(setter(strip_option), default)]
    pub names: Option<Vec<String>>,
    #[serde(rename = "default")]
    #[builder(setter(strip_option), default)]
    pub default: Option<OwnedValue>,
}

/// Schema representing an SSO provider.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SSOProviderSchema {
    /// SSO provider ID.
    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,

    /// List of SSO domains.
    #[serde(rename = "sso_domains")]
    #[builder(setter(strip_option), default)]
    pub sso_domains: Option<Vec<SSODomain>>,

    /// SAML configuration details.
    #[serde(rename = "saml")]
    #[builder(setter(strip_option), default)]
    pub saml: Option<SAMLConfiguration>,
}

/// Domain associated with SSO.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SSODomain {
    /// Domain name.
    #[serde(rename = "domain")]
    #[builder(setter(strip_option), default)]
    pub domain: Option<String>,
}

/// SAML configuration details.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SAMLConfiguration {
    /// Entity ID.
    #[serde(rename = "entity_id")]
    #[builder(setter(strip_option), default)]
    pub entity_id: Option<String>,

    /// SAML metadata XML.
    #[serde(rename = "metadata_xml")]
    #[builder(setter(strip_option), default)]
    pub metadata_xml: Option<String>,

    /// SAML metadata URL.
    #[serde(rename = "metadata_url")]
    #[builder(setter(strip_option), default)]
    pub metadata_url: Option<String>,

    /// Attribute mapping configuration.
    #[serde(rename = "attribute_mapping")]
    #[builder(setter(strip_option), default)]
    pub attribute_mapping: Option<SAMLAttributeMappingSchema>,
}

/// Response schema for access and refresh tokens.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AccessTokenResponseSchema {
    /// A valid JWT that will expire in `expires_in` seconds.
    #[serde(rename = "access_token")]
    #[builder(setter(strip_option), default)]
    pub access_token: Option<String>,

    /// An opaque string that can be used once to obtain new tokens.
    #[serde(rename = "refresh_token")]
    #[builder(setter(strip_option), default)]
    pub refresh_token: Option<String>,

    /// Token type, usually `bearer`.
    #[serde(rename = "token_type")]
    #[builder(setter(strip_option), default)]
    pub token_type: Option<String>,

    /// Number of seconds until the `access_token` expires.
    #[serde(rename = "expires_in")]
    #[builder(setter(strip_option), default)]
    pub expires_in: Option<i64>,

    /// UNIX timestamp when the `access_token` expires.
    #[serde(rename = "expires_at")]
    #[builder(setter(strip_option), default)]
    pub expires_at: Option<i64>,

    /// Indicates if the password used is weak.
    #[serde(rename = "weak_password")]
    #[builder(setter(strip_option), default)]
    pub weak_password: Option<WeakPasswordResponse>,

    /// User information.
    #[serde(rename = "user")]
    #[builder(setter(strip_option), default)]
    pub user: Option<UserSchema>,
}

/// Response indicating a weak password.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct WeakPasswordResponse {
    /// Reasons why the password is weak.
    #[serde(rename = "reasons")]
    #[builder(setter(strip_option), default)]
    pub reasons: Option<Vec<WeakPasswordReason>>,

    /// Message describing the weakness.
    #[serde(rename = "message")]
    #[builder(setter(strip_option), default)]
    pub message: Option<String>,
}

/// Represents a MFA factor.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct MFAFactorSchema {
    /// Factor ID.
    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,

    /// Status of the factor.
    ///
    /// Usually one of:
    /// - `verified`
    /// - `unverified`
    #[serde(rename = "status")]
    #[builder(setter(strip_option), default)]
    pub status: Option<MFAFactorStatus>,

    /// Friendly name for the factor.
    #[serde(rename = "friendly_name")]
    #[builder(setter(strip_option), default)]
    pub friendly_name: Option<String>,

    /// Type of the factor.
    ///
    /// Usually one of:
    /// - `totp`
    /// - `phone`
    /// - `webauthn`
    #[serde(rename = "factor_type")]
    #[builder(setter(strip_option), default)]
    pub factor_type: Option<MFAFactorType>,

    /// WebAuthn credential details.
    #[serde(rename = "web_authn_credential")]
    #[builder(setter(strip_option), default)]
    pub web_authn_credential: Option<OwnedValue>,

    /// Phone number associated with the factor.
    #[serde(rename = "phone")]
    #[builder(setter(strip_option), default)]
    pub phone: Option<String>,

    /// Timestamp when the factor was created.
    #[serde(rename = "created_at")]
    #[builder(setter(strip_option), default)]
    pub created_at: Option<DateTime<Utc>>,

    /// Timestamp when the factor was last updated.
    #[serde(rename = "updated_at")]
    #[builder(setter(strip_option), default)]
    pub updated_at: Option<DateTime<Utc>>,

    /// Timestamp when the factor was last challenged.
    #[serde(rename = "last_challenged_at")]
    #[builder(setter(strip_option), default)]
    pub last_challenged_at: Option<DateTime<Utc>>,
}

/// Status of the MFA factor.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MFAFactorStatus {
    Verified,
    Unverified,
}

/// Type of the MFA factor.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MFAFactorType {
    Totp,
    Phone,
    Webauthn,
}

/// Schema representing an identity.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct IdentitySchema {
    #[serde(rename = "identity_id")]
    #[builder(setter(strip_option), default)]
    pub identity_id: Option<String>,

    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,

    #[serde(rename = "user_id")]
    #[builder(setter(strip_option), default)]
    pub user_id: Option<String>,

    #[serde(rename = "identity_data")]
    #[builder(setter(strip_option), default)]
    pub identity_data: Option<OwnedValue>,

    #[serde(rename = "provider")]
    #[builder(setter(strip_option), default)]
    pub provider: Option<String>,

    #[serde(rename = "last_sign_in_at")]
    #[builder(setter(strip_option), default)]
    pub last_sign_in_at: Option<DateTime<Utc>>,

    #[serde(rename = "created_at")]
    #[builder(setter(strip_option), default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(rename = "updated_at")]
    #[builder(setter(strip_option), default)]
    pub updated_at: Option<DateTime<Utc>>,

    #[serde(rename = "email")]
    #[builder(setter(strip_option), default)]
    pub email: Option<String>,
}

/// Response for TOTP or phone challenge.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct TOTPPhoneChallengeResponse {
    /// ID of the challenge.
    #[serde(rename = "id")]
    pub id: String,

    /// Type of the challenge.
    #[serde(rename = "type")]
    pub challenge_type: TOTPPhoneChallengeType,

    /// UNIX timestamp when the challenge expires.
    #[serde(rename = "expires_at")]
    pub expires_at: i64,
}

/// Type of the TOTP or phone challenge.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TOTPPhoneChallengeType {
    Totp,
    Phone,
}

/// Response for WebAuthn challenge.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct WebAuthnChallengeResponse {
    /// ID of the challenge.
    #[serde(rename = "id")]
    pub id: String,

    /// Type of the challenge.
    #[serde(rename = "type")]
    pub challenge_type: WebAuthnChallengeType,

    /// UNIX timestamp when the challenge expires.
    #[serde(rename = "expires_at")]
    pub expires_at: i64,

    /// Credential request options.
    #[serde(rename = "credential_request_options")]
    #[builder(setter(strip_option), default)]
    pub credential_request_options: Option<CredentialRequestOptions>,

    /// Credential creation options.
    #[serde(rename = "credential_creation_options")]
    #[builder(setter(strip_option), default)]
    pub credential_creation_options: Option<CredentialCreationOptions>,
}

/// Type of the WebAuthn challenge.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum WebAuthnChallengeType {
    Webauthn,
}

/// WebAuthn credential assertion options.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct CredentialAssertion {
    /// A random challenge generated by the server, base64url encoded.
    #[serde(rename = "challenge")]
    pub challenge: String,

    /// The relying party's identifier (usually the domain name).
    #[serde(rename = "rpId")]
    pub rp_id: String,

    /// List of credentials acceptable for this authentication.
    #[serde(rename = "allowCredentials")]
    pub allow_credentials: Vec<PublicKeyCredentialDescriptor>,

    /// Time (in milliseconds) that the user has to respond to the authentication prompt.
    #[serde(rename = "timeout")]
    pub timeout: i64,

    /// The relying party's requirements for user verification.
    #[serde(rename = "userVerification")]
    #[builder(setter(strip_option), default)]
    pub user_verification: Option<UserVerificationRequirement>,

    /// Additional parameters requesting additional processing by the client.
    #[serde(rename = "extensions")]
    #[builder(setter(strip_option), default)]
    pub extensions: Option<OwnedValue>,

    /// Status of the credential assertion.
    #[serde(rename = "status")]
    #[builder(setter(strip_option), default)]
    pub status: Option<CredentialAssertionStatus>,

    /// Error message if the assertion failed.
    #[serde(rename = "errorMessage")]
    #[builder(setter(strip_option), default)]
    pub error_message: Option<String>,

    /// User handle, base64url encoded.
    #[serde(rename = "userHandle")]
    #[builder(setter(strip_option), default)]
    pub user_handle: Option<String>,

    /// Type of authenticator to use.
    #[serde(rename = "authenticatorAttachment")]
    #[builder(setter(strip_option), default)]
    pub authenticator_attachment: Option<AuthenticatorAttachment>,
}

/// Options for requesting a credential assertion.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]

pub struct CredentialRequestOptions {
    /// A challenge to be signed by the authenticator.
    #[serde(rename = "challenge")]
    #[builder(setter(strip_option), default)]
    pub challenge: Option<String>,

    /// Time (in milliseconds) that the caller is willing to wait for the call to complete.
    #[serde(rename = "timeout")]
    #[builder(setter(strip_option), default)]
    pub timeout: Option<i64>,

    /// Relying Party ID.
    #[serde(rename = "rpId")]
    #[builder(setter(strip_option), default)]
    pub rp_id: Option<String>,

    /// List of credentials acceptable for this authentication.
    #[serde(rename = "allowCredentials")]
    #[builder(setter(strip_option), default)]
    pub allow_credentials: Option<Vec<PublicKeyCredentialDescriptor>>,

    /// User verification requirement.
    #[serde(rename = "userVerification")]
    #[builder(setter(strip_option), default)]
    pub user_verification: Option<UserVerificationRequirement>,
}

/// Options for creating a new credential.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]

pub struct CredentialCreationOptions {
    /// Relying Party information.
    #[serde(rename = "rp")]
    #[builder(setter(strip_option), default)]
    pub rp: Option<RelyingPartyInfo>,

    /// User information.
    #[serde(rename = "user")]
    #[builder(setter(strip_option), default)]
    pub user: Option<UserInfo>,

    /// A challenge to be signed by the authenticator.
    #[serde(rename = "challenge")]
    #[builder(setter(strip_option), default)]
    pub challenge: Option<String>,

    /// Public key credential parameters.
    #[serde(rename = "pubKeyCredParams")]
    #[builder(setter(strip_option), default)]
    pub pub_key_cred_params: Option<Vec<PublicKeyCredentialParameters>>,

    /// Time (in milliseconds) that the caller is willing to wait for the call to complete.
    #[serde(rename = "timeout")]
    #[builder(setter(strip_option), default)]
    pub timeout: Option<i64>,

    /// List of credentials to exclude.
    #[serde(rename = "excludeCredentials")]
    #[builder(setter(strip_option), default)]
    pub exclude_credentials: Option<Vec<PublicKeyCredentialDescriptor>>,

    /// Authenticator selection criteria.
    #[serde(rename = "authenticatorSelection")]
    #[builder(setter(strip_option), default)]
    pub authenticator_selection: Option<AuthenticatorSelectionCriteria>,

    /// Preferred attestation conveyance.
    #[serde(rename = "attestation")]
    #[builder(setter(strip_option), default)]
    pub attestation: Option<AttestationConveyancePreference>,
}

/// Information about the relying party.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct RelyingPartyInfo {
    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,
    #[serde(rename = "name")]
    #[builder(setter(strip_option), default)]
    pub name: Option<String>,
}

/// Information about the user.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct UserInfo {
    #[serde(rename = "name")]
    #[builder(setter(strip_option), default)]
    pub name: Option<String>,
    // Include other user fields as necessary
}

/// Parameters for public key credentials.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct PublicKeyCredentialParameters {
    #[serde(rename = "type")]
    #[builder(setter(strip_option), default)]
    pub cred_type: Option<String>,
    #[serde(rename = "alg")]
    #[builder(setter(strip_option), default)]
    pub alg: Option<i32>,
}

/// Descriptor for public key credentials.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct PublicKeyCredentialDescriptor {
    #[serde(rename = "type")]
    #[builder(setter(strip_option), default)]
    pub cred_type: Option<String>,
    #[serde(rename = "id")]
    #[builder(setter(strip_option), default)]
    pub id: Option<String>,
    #[serde(rename = "transports")]
    #[builder(setter(strip_option), default)]
    pub transports: Option<Vec<String>>,
}

/// Criteria for authenticator selection.
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AuthenticatorSelectionCriteria {
    #[serde(rename = "authenticatorAttachment")]
    #[builder(setter(strip_option), default)]
    pub authenticator_attachment: Option<AuthenticatorAttachment>,
    #[serde(rename = "requireResidentKey")]
    #[builder(setter(strip_option), default)]
    pub require_resident_key: Option<bool>,
    #[serde(rename = "userVerification")]
    #[builder(setter(strip_option), default)]
    pub user_verification: Option<UserVerificationRequirement>,
}

/// User verification requirement.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum UserVerificationRequirement {
    Required,
    Preferred,
    Discouraged,
}

/// Status of the credential assertion.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CredentialAssertionStatus {
    Ok,
    Failed,
}

/// Type of authenticator attachment.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum AuthenticatorAttachment {
    Platform,
    CrossPlatform,
}

/// Preferred attestation conveyance.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AttestationConveyancePreference {
    None,
    Indirect,
    Direct,
}

/// Response from admin generate link
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AdminGenerateLinkResponse {
    pub action_link: String,
    #[builder(setter(strip_option), default)]
    pub email_otp: Option<String>,
    #[builder(setter(strip_option), default)]
    pub hashed_token: Option<String>,
    #[builder(setter(strip_option), default)]
    pub verification_type: Option<String>,
    #[builder(setter(strip_option), default)]
    pub redirect_to: Option<String>,
    // Include any additional properties
}

/// Response for settings endpoint
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SettingsResponse {
    pub disable_signup: bool,
    pub mailer_autoconfirm: bool,
    pub phone_autoconfirm: bool,
    #[builder(setter(strip_option), default)]
    pub sms_provider: Option<String>,
    pub saml_enabled: bool,
    pub external: std::collections::HashMap<String, bool>,
}

/// Audit log entry
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AuditLogEntry {
    pub id: String,
    pub payload: AuditLogPayload,
    pub created_at: DateTime<Utc>,
    #[builder(setter(strip_option), default)]
    pub ip_address: Option<String>,
}

/// Audit log payload
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct AuditLogPayload {
    #[builder(setter(strip_option), default)]
    pub actor_id: Option<String>,
    #[builder(setter(strip_option), default)]
    pub actor_via_sso: Option<bool>,
    #[builder(setter(strip_option), default)]
    pub actor_username: Option<String>,
    #[builder(setter(strip_option), default)]
    pub actor_name: Option<String>,
    #[builder(setter(strip_option), default)]
    pub traits: Option<OwnedValue>,
    #[builder(setter(strip_option), default)]
    pub action: Option<String>,
    #[builder(setter(strip_option), default)]
    pub log_type: Option<String>,
}
