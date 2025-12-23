/// The error returned by [`Hasher::try_finalize()`](crate::Hasher::try_finalize()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Detected SHA-1 collision attack with digest {digest}")]
    CollisionAttack { digest: crate::ObjectId },
}

pub(super) mod _impl {
    use sha1_checked::{CollisionResult, Digest};

    use crate::hasher::Error;

    /// Hash implementations that can be used once.
    #[derive(Clone)]
    pub enum Hasher {
        /// An implementation of the SHA1 hash.
        ///
        /// We use [`sha1_checked`] to implement the same collision detection algorithm as Git.
        Sha1(sha1_checked::Sha1),
        /// An implementation of the SHA256 hash.
        #[cfg(feature = "sha256")]
        Sha256(sha2::Sha256),
    }

    impl Hasher {
        /// Let's not make this public to force people to go through [`hasher()`].
        fn new_sha1() -> Self {
            // This matches the configuration used by Git, which only uses
            // the collision detection to bail out, rather than computing
            // alternate “safe hashes” for inputs where a collision attack
            // was detected.
            Self::Sha1(sha1_checked::Builder::default().safe_hash(false).build())
        }

        /// Let's not make this public to force people to go through [`hasher()`].
        #[cfg(feature = "sha256")]
        fn new_sha256() -> Self {
            Self::Sha256(sha2::Sha256::new())
        }
    }

    impl Hasher {
        /// Digest the given `bytes`.
        pub fn update(&mut self, bytes: &[u8]) {
            match self {
                Hasher::Sha1(sha1) => sha1.update(bytes),
                #[cfg(feature = "sha256")]
                Hasher::Sha256(sha256) => sha256.update(bytes),
            }
        }

        /// Finalize the hash and produce an object id.
        ///
        /// Returns [`Error`] if a collision attack is detected.
        ///
        /// TODO: Since SHA256 has an infallible `finalize`, it might be worth investigating
        /// turning the return type into `Result<crate::ObjectId, Infallible>` when this crate is
        /// compiled with SHA256 support only.
        #[inline]
        pub fn try_finalize(self) -> Result<crate::ObjectId, Error> {
            match self {
                Hasher::Sha1(sha1) => match sha1.try_finalize() {
                    CollisionResult::Ok(digest) => Ok(crate::ObjectId::Sha1(digest.into())),
                    CollisionResult::Mitigated(_) => {
                        // SAFETY: `CollisionResult::Mitigated` is only
                        // returned when `safe_hash()` is on. `Hasher`’s field
                        // is private, and we only construct it in the
                        // `Default` instance, which turns `safe_hash()` off.
                        //
                        // As of Rust 1.84.1, the compiler can’t figure out
                        // this function cannot panic without this.
                        #[allow(unsafe_code)]
                        unsafe {
                            std::hint::unreachable_unchecked()
                        }
                    }
                    CollisionResult::Collision(digest) => Err(Error::CollisionAttack {
                        digest: crate::ObjectId::Sha1(digest.into()),
                    }),
                },
                #[cfg(feature = "sha256")]
                Hasher::Sha256(sha256) => Ok(crate::ObjectId::Sha256(sha256.finalize().into())),
            }
        }
    }

    /// Produce a hasher suitable for the given `kind` of hash.
    #[inline]
    pub fn hasher(kind: crate::Kind) -> Hasher {
        match kind {
            crate::Kind::Sha1 => Hasher::new_sha1(),
            #[cfg(feature = "sha256")]
            crate::Kind::Sha256 => Hasher::new_sha256(),
        }
    }
}
