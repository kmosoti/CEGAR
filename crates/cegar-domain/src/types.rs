//! Typed, namespace-separated identities for the experimental artifact domain.
//!
//! Every value here is a plain immutable datum: no paths, names or unqualified
//! hex strings are accepted in place of a typed identity. Content identities
//! compare by `(algorithm, digest)` exactly; symbolic identities compare by
//! their canonical bytes. Coding and ordering are fixed by this module and are
//! asserted by unit tests, never derived from hash-map iteration or arrival
//! order.

use core::fmt;

/// Version string bound into every experimental receipt.
pub const DOMAIN_SEMANTICS: &str = "cegar.artifact-domain.v0.1-experimental";

pub(crate) fn encode_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&(len as u32).to_le_bytes());
}

pub(crate) fn encode_str(value: &str, out: &mut Vec<u8>) {
    encode_len(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

/// Digest algorithms this slice is able to represent.
///
/// Selecting an algorithm is an owner decision recorded in ADR 0007/0008; this
/// slice supports exactly one so an unsupported algorithm is rejected rather
/// than silently coerced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigestAlgorithm {
    Sha256,
}

impl DigestAlgorithm {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }

    pub const fn byte_len(self) -> usize {
        match self {
            Self::Sha256 => 32,
        }
    }
}

/// A content digest. Equality is typed equality of algorithm and bytes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest {
    algorithm: DigestAlgorithm,
    bytes: [u8; 32],
}

impl Digest {
    pub const fn sha256(bytes: [u8; 32]) -> Self {
        Self {
            algorithm: DigestAlgorithm::Sha256,
            bytes,
        }
    }

    pub const fn algorithm(self) -> DigestAlgorithm {
        self.algorithm
    }

    pub const fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        encode_str(self.algorithm.id(), out);
        out.extend_from_slice(&self.bytes);
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.algorithm.id())?;
        f.write_str(":")?;
        for byte in self.bytes {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

macro_rules! symbolic_identity {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn encode(&self, out: &mut Vec<u8>) {
                encode_str(crate::types::tag(stringify!($name)), out);
                encode_str(&self.0, out);
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

symbolic_identity!(
    /// Authenticated, scoped principal identity. Never a plan-supplied string
    /// that happens to look authorized.
    PrincipalId
);
symbolic_identity!(
    /// Tenant boundary used by the scoped evidence checks.
    TenantId
);
symbolic_identity!(
    /// Intended audience of an attestation or publication.
    AudienceId
);
symbolic_identity!(
    /// Publication destination.
    DestinationId
);
symbolic_identity!(
    /// Identity of one test run, used to detect contradictory records.
    RunId
);
symbolic_identity!(
    /// Registered test-suite identifier.
    SuiteName
);
symbolic_identity!(
    /// Registered operation identifier.
    OperationName
);

pub(crate) const fn tag(name: &'static str) -> &'static str {
    name
}

/// Typed artifact role. Two roles with equal bytes are still different typed
/// identities and never interchangeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FormatId {
    Base,
    Patch,
    Candidate,
}

impl FormatId {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Base => "format.base",
            Self::Patch => "format.patch",
            Self::Candidate => "format.candidate",
        }
    }
}

/// Exact bytes plus format role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactId {
    pub digest: Digest,
    pub format: FormatId,
}

impl ArtifactId {
    pub const fn new(digest: Digest, format: FormatId) -> Self {
        Self { digest, format }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        self.digest.encode(out);
        encode_str(self.format.tag(), out);
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.digest)?;
        write!(f, " [{}]", self.format.tag())
    }
}

/// Versioned operation contract identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationRef {
    pub name: OperationName,
    pub version: u32,
    pub contract: Digest,
}

impl OperationRef {
    pub fn new(name: OperationName, version: u32, contract: Digest) -> Self {
        Self {
            name,
            version,
            contract,
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        self.name.encode(out);
        out.extend_from_slice(&self.version.to_le_bytes());
        self.contract.encode(out);
    }
}

/// Versioned registered suite identity, including its definition digest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SuiteRef {
    pub name: SuiteName,
    pub version: u32,
    pub definition: Digest,
}

impl SuiteRef {
    pub fn new(name: SuiteName, version: u32, definition: Digest) -> Self {
        Self {
            name,
            version,
            definition,
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        self.name.encode(out);
        out.extend_from_slice(&self.version.to_le_bytes());
        self.definition.encode(out);
    }
}

/// Versioned policy identity. A version mismatch never inherits authority.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyRef {
    pub version: u32,
    pub digest: Digest,
}

impl PolicyRef {
    pub const fn new(version: u32, digest: Digest) -> Self {
        Self { version, digest }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.version.to_le_bytes());
        self.digest.encode(out);
    }
}

/// Versioned snapshot identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SnapshotRef {
    pub generation: u64,
    pub digest: Digest,
}

impl SnapshotRef {
    pub const fn new(generation: u64, digest: Digest) -> Self {
        Self { generation, digest }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.generation.to_le_bytes());
        self.digest.encode(out);
    }
}

/// Deterministic synthetic digest helper for fixtures and tests only.
pub fn synthetic_digest(seed: u8) -> Digest {
    let mut bytes = [0u8; 32];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = seed.wrapping_mul(31).wrapping_add(index as u8 ^ seed);
    }
    Digest::sha256(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_typed_identities_never_collide() {
        let principal = PrincipalId::new("same");
        let tenant = TenantId::new("same");
        let mut left = Vec::new();
        let mut right = Vec::new();
        principal.encode(&mut left);
        tenant.encode(&mut right);
        assert_ne!(left, right);
        // Display text may coincide; only the typed encoding separates them.
        assert_eq!(principal.to_string(), tenant.to_string());
    }

    #[test]
    fn digest_display_and_encoding_are_stable() {
        let digest = Digest::sha256([7u8; 32]);
        assert_eq!(digest.to_string(), format!("sha256:{}", "07".repeat(32)),);
        let mut encoded = Vec::new();
        digest.encode(&mut encoded);
        // Length-prefixed algorithm tag followed by the 32 raw bytes.
        assert_eq!(encoded.len(), 4 + 6 + 32);
        assert_eq!(&encoded[4..10], b"sha256");
    }

    #[test]
    fn artifact_role_is_part_of_identity() {
        let digest = synthetic_digest(3);
        let base = ArtifactId::new(digest, FormatId::Base);
        let patch = ArtifactId::new(digest, FormatId::Patch);
        assert_eq!(base.digest, patch.digest);
        assert_ne!(base, patch);
    }

    #[test]
    fn string_encoding_is_length_prefixed_and_unambiguous() {
        let mut joined = Vec::new();
        encode_str("ab", &mut joined);
        encode_str("c", &mut joined);
        let mut split = Vec::new();
        encode_str("a", &mut split);
        encode_str("bc", &mut split);
        assert_ne!(joined, split);
    }
}
