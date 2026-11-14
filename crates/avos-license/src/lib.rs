#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseEdition {
    CommunityFree,
    PersonalPerpetual,
    CommercialPerpetual,
}

pub fn verify_offline_key(key_bytes: &[u8]) -> LicenseEdition {
    if key_bytes.starts_with(b"AVOS-COMM-") {
        LicenseEdition::CommercialPerpetual
    } else if key_bytes.starts_with(b"AVOS-PERS-") {
        LicenseEdition::PersonalPerpetual
    } else {
        LicenseEdition::CommunityFree
    }
}
