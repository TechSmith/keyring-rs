use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use security_framework::os::macos::passwords::find_generic_password;

use crate::{Error as KeyError, ErrorCode, Platform, PlatformCredential, Result};

pub fn platform() -> Platform {
    Platform::MacOs
}

use crate::attrs::{MacCredential, MacKeychainDomain};
pub use security_framework::base::Error;

fn get_keychain(map: &MacCredential) -> Result<SecKeychain> {
    let domain = match map.domain {
        MacKeychainDomain::User => SecPreferencesDomain::User,
        MacKeychainDomain::System => SecPreferencesDomain::System,
        MacKeychainDomain::Common => SecPreferencesDomain::Common,
        MacKeychainDomain::Dynamic => SecPreferencesDomain::Dynamic,
    };
    match SecKeychain::default_for_domain(domain) {
        Ok(keychain) => Ok(keychain),
        Err(err) => Err(decode_error(err)),
    }
}

pub fn set_password(map: &PlatformCredential, password: &str) -> Result<()> {
    if let PlatformCredential::Mac(map) = map {
        get_keychain(map)?
            .set_generic_password(&map.service, &map.account, password.as_bytes())
            .map_err(decode_error)?;
        Ok(())
    } else {
        Err(ErrorCode::BadCredentialMapPlatform.into())
    }
}

pub fn get_password(map: &mut PlatformCredential) -> Result<String> {
    if let PlatformCredential::Mac(map) = map {
        let (password_bytes, _) =
            find_generic_password(Some(&[get_keychain(map)?]), &map.service, &map.account)
                .map_err(decode_error)?;
        // Mac keychain allows non-UTF8 values, passwords from 3rd parties may not be UTF-8.
        let bytes = password_bytes.to_vec();
        let password = String::from_utf8(bytes.clone())
            .map_err(|_| KeyError::new(ErrorCode::BadEncoding("password".to_string(), bytes)))?;
        Ok(password)
    } else {
        Err(ErrorCode::BadCredentialMapPlatform.into())
    }
}

pub fn delete_password(map: &PlatformCredential) -> Result<()> {
    if let PlatformCredential::Mac(map) = map {
        let (_, item) =
            find_generic_password(Some(&[get_keychain(map)?]), &map.service, &map.account)
                .map_err(decode_error)?;
        item.delete();
        Ok(())
    } else {
        Err(ErrorCode::BadCredentialMapPlatform.into())
    }
}

// The MacOS error codes used here are from:
// https://opensource.apple.com/source/libsecurity_keychain/libsecurity_keychain-78/lib/SecBase.h.auto.html
fn decode_error(err: Error) -> KeyError {
    match err.code() {
        -25291 => KeyError::new_from_platform(ErrorCode::NoStorageAccess, err), // errSecNotAvailable
        -25292 => KeyError::new_from_platform(ErrorCode::NoStorageAccess, err), // errSecReadOnly
        -25294 => KeyError::new_from_platform(ErrorCode::NoStorageAccess, err), // errSecNoSuchKeychain
        -25295 => KeyError::new_from_platform(ErrorCode::NoStorageAccess, err), // errSecInvalidKeychain
        -25300 => KeyError::new_from_platform(ErrorCode::NoEntry, err), // errSecItemNotFound
        _ => KeyError::new_from_platform(ErrorCode::PlatformFailure, err),
    }
}
