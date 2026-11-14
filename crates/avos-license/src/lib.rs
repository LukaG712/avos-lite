use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Embedded AVOS Ed25519 public verifying key (replace with production keypair at release).
pub const DEFAULT_AVOS_PUBLIC_KEY: [u8; 32] = [
    0x3b, 0x6a, 0x27, 0xbc, 0xce, 0xb6, 0xa4, 0x2d,
    0x62, 0xa3, 0xa8, 0xd0, 0x2a, 0x6f, 0x0d, 0x73,
    0x65, 0x32, 0x15, 0x77, 0x1d, 0xe2, 0x43, 0xa6,
    0x3a, 0xc0, 0x48, 0xa1, 0x8b, 0x59, 0xda, 0x29,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseEdition {
    CommunityFree,
    PersonalPerpetual,
    CommercialPerpetual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub edition: LicenseEdition,
    pub issued_to: String,
    pub issued_at_unix: u64,
    pub expires_at_unix: Option<u64>,
    pub max_seats: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseError {
    EmptyKey,
    MalformedFormat,
    InvalidBase64,
    InvalidPublicKey,
    InvalidSignature,
    InvalidClaimsJson,
    Expired { expires_at_unix: u64, now_unix: u64 },
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Signs a `LicenseClaims` payload into `<base64url_payload>.<base64url_signature>`.
pub fn sign_license(claims: &LicenseClaims, signing_key: &SigningKey) -> Result<String, LicenseError> {
    let payload_bytes = serde_json::to_vec(claims).map_err(|_| LicenseError::InvalidClaimsJson)?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_bytes);
    let signature = signing_key.sign(payload_b64.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());
    Ok(format!("{payload_b64}.{sig_b64}"))
}

/// Verifies an offline `license.avoskey` against a specific 32-byte Ed25519 public key and timestamp.
pub fn verify_offline_key_with_pubkey(
    key_bytes: &[u8],
    pubkey_bytes: &[u8; 32],
    now_unix: u64,
) -> Result<LicenseClaims, LicenseError> {
    let raw = std::str::from_utf8(key_bytes)
        .map_err(|_| LicenseError::MalformedFormat)?
        .trim();

    if raw.is_empty() {
        return Err(LicenseError::EmptyKey);
    }

    let (payload_b64, sig_b64) = raw.split_once('.').ok_or(LicenseError::MalformedFormat)?;
    if payload_b64.is_empty() || sig_b64.is_empty() || sig_b64.contains('.') {
        return Err(LicenseError::MalformedFormat);
    }

    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| LicenseError::InvalidBase64)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| LicenseError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_arr);

    let verifying_key =
        VerifyingKey::from_bytes(pubkey_bytes).map_err(|_| LicenseError::InvalidPublicKey)?;

    verifying_key
        .verify(payload_b64.as_bytes(), &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| LicenseError::InvalidBase64)?;
    let claims: LicenseClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| LicenseError::InvalidClaimsJson)?;

    if let Some(exp) = claims.expires_at_unix
        && now_unix > exp
    {
        return Err(LicenseError::Expired {
            expires_at_unix: exp,
            now_unix,
        });
    }

    Ok(claims)
}

/// Verifies an offline `license.avoskey` using the compiled-in AVOS public key, falling back to `CommunityFree`.
pub fn verify_offline_key(key_bytes: &[u8]) -> LicenseEdition {
    match verify_offline_key_with_pubkey(
        key_bytes,
        &DEFAULT_AVOS_PUBLIC_KEY,
        current_unix_timestamp(),
    ) {
        Ok(claims) => claims.edition,
        Err(_) => LicenseEdition::CommunityFree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn verifies_valid_commercial_license() {
        let sk = test_signing_key();
        let pk = sk.verifying_key().to_bytes();
        let claims = LicenseClaims {
            edition: LicenseEdition::CommercialPerpetual,
            issued_to: "luka@avos.sh".into(),
            issued_at_unix: 1_700_000_000,
            expires_at_unix: None,
            max_seats: 5,
        };

        let token = sign_license(&claims, &sk).unwrap();
        let verified =
            verify_offline_key_with_pubkey(token.as_bytes(), &pk, 1_750_000_000).unwrap();
        assert_eq!(verified, claims);
    }

    #[test]
    fn rejects_tampered_payload() {
        let sk = test_signing_key();
        let pk = sk.verifying_key().to_bytes();
        let claims = LicenseClaims {
            edition: LicenseEdition::PersonalPerpetual,
            issued_to: "indie@example.com".into(),
            issued_at_unix: 1_700_000_000,
            expires_at_unix: None,
            max_seats: 1,
        };

        let token = sign_license(&claims, &sk).unwrap();
        let (_, sig) = token.split_once('.').unwrap();
        let fake_payload = URL_SAFE_NO_PAD.encode(
            br#"{"edition":"COMMERCIAL_PERPETUAL","issued_to":"evil","issued_at_unix":1,"expires_at_unix":null,"max_seats":99}"#,
        );
        let tampered = format!("{fake_payload}.{sig}");

        assert_eq!(
            verify_offline_key_with_pubkey(tampered.as_bytes(), &pk, 1_750_000_000),
            Err(LicenseError::InvalidSignature)
        );
    }

    #[test]
    fn rejects_expired_license() {
        let sk = test_signing_key();
        let pk = sk.verifying_key().to_bytes();
        let claims = LicenseClaims {
            edition: LicenseEdition::PersonalPerpetual,
            issued_to: "trial@example.com".into(),
            issued_at_unix: 1_700_000_000,
            expires_at_unix: Some(1_700_100_000),
            max_seats: 1,
        };

        let token = sign_license(&claims, &sk).unwrap();
        assert_eq!(
            verify_offline_key_with_pubkey(token.as_bytes(), &pk, 1_700_200_000),
            Err(LicenseError::Expired {
                expires_at_unix: 1_700_100_000,
                now_unix: 1_700_200_000,
            })
        );
    }

    #[test]
    fn empty_key_falls_back_to_community_free() {
        assert_eq!(verify_offline_key(b""), LicenseEdition::CommunityFree);
        assert_eq!(verify_offline_key(b"AVOS-COMM-FAKE"), LicenseEdition::CommunityFree);
    }
}
