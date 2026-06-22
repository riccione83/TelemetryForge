use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

pub fn hash_password(password: &str) -> Result<String, String> {
    if password.len() < 8 {
        return Err("Password must contain at least 8 characters".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashes_are_not_plaintext_and_verify() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(!hash.contains("correct horse battery staple"));
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong password", &hash));
    }
}
