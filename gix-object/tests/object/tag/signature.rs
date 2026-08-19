use bstr::ByteSlice;
use gix_object::{
    Kind, Tag, TagRef, TagRefIter, WriteTo,
    signature::{
        Format,
        sign::Options,
        verify::{Options as VerifyOptions, Outcome, TrustLevel},
    },
};
use gix_testtools::signature;

use crate::Result;

#[test]
fn ssh_and_tag_ref_api() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (_key_home, key) = signature::ssh_private_key()?;
    let unsigned = tag(gix_hash::Kind::Sha1);
    let mut data = Vec::new();
    unsigned.write_to(&mut data)?;
    let signed = TagRef::from_bytes(&data, gix_hash::Kind::Sha1)?.sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    assert!(verify_ssh(&signed)?.is_valid(), "the generated SSH signature is valid");
    Ok(())
}

#[test]
fn openpgp() -> Result {
    if !signature::program_available("gpg") {
        return Ok(());
    }
    let home = signature::openpgp_home()?;
    let command_home = signature::path_for_command(home.path()).into_os_string();
    let signed = tag(gix_hash::Kind::Sha1).sign(Options {
        format: Format::OpenPgp,
        program: "gpg".into(),
        program_arguments: vec!["--pinentry-mode=error".into()],
        signing_key: signature::IDENTITY.into(),
        environment: vec![("GNUPGHOME".into(), command_home.clone())],
    })?;
    assert!(
        verify(
            &signed,
            VerifyOptions::OpenPgp {
                program: "gpg".into(),
                program_arguments: Vec::new(),
                environment: vec![("GNUPGHOME".into(), command_home.clone())],
                minimum_trust: TrustLevel::Undefined,
            },
        )?
        .is_valid(),
        "the generated OpenPGP signature is valid"
    );
    assert!(
        git_verifies_openpgp(&signed, &command_home)?,
        "Git accepts the same generated annotated-tag signature"
    );
    Ok(())
}

#[test]
fn x509() -> Result {
    if !signature::program_available("gpgsm") {
        return Ok(());
    }
    let home = signature::x509_home()?;
    let command_home = signature::path_for_command(home.path()).into_os_string();
    let signed = tag(gix_hash::Kind::Sha1).sign(Options {
        format: Format::X509,
        program: "gpgsm".into(),
        program_arguments: Vec::new(),
        signing_key: signature::IDENTITY.into(),
        environment: vec![("GNUPGHOME".into(), command_home.clone())],
    })?;
    let outcome = verify(
        &signed,
        VerifyOptions::X509 {
            program: "gpgsm".into(),
            program_arguments: Vec::new(),
            environment: vec![("GNUPGHOME".into(), command_home)],
            minimum_trust: TrustLevel::Undefined,
        },
    )?;
    assert!(
        outcome.is_valid(),
        "the generated X.509 signature is valid: {outcome:?}"
    );
    Ok(())
}

#[test]
fn replaces_the_active_signature_and_tampering_is_invalid() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (key_home, key) = signature::ssh_private_key()?;
    let mut tag = tag(gix_hash::Kind::Sha1);
    tag.signature = Some("-----BEGIN PGP SIGNATURE-----\nold".into());
    let signed = tag.sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    let mut data = Vec::new();
    signed.write_to(&mut data)?;
    assert!(
        !data.contains_str(b"-----BEGIN PGP SIGNATURE-----"),
        "the previous in-body PGP signature is removed"
    );
    assert!(
        data.contains_str(b"-----BEGIN SSH SIGNATURE-----"),
        "exactly one replacement SSH signature is written"
    );
    assert!(verify_ssh(&signed)?.is_valid(), "the replacement signature is valid");

    let message_start = data
        .windows(b"signed tag".len())
        .position(|window| window == b"signed tag")
        .expect("the signed message is present");
    data[message_start] = b'S';
    let (signature, signed_data) = TagRefIter::signature(&data).expect("the signature remains discoverable");
    let outcome = signed_data.verify(
        signature.data,
        ssh_verify_options(gix_date::Time::new(1_700_000_000, 0)),
    )?;
    assert!(
        !outcome.is_valid(),
        "changing any signed tag byte invalidates the signature"
    );
    drop(key_home);
    Ok(())
}

#[test]
#[cfg(feature = "sha256")]
fn native_sha256_has_one_in_body_signature_and_no_compatibility_header() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (_key_home, key) = signature::ssh_private_key()?;
    let signed = tag(gix_hash::Kind::Sha256).sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    let mut data = Vec::new();
    signed.write_to(&mut data)?;
    assert!(!data.contains_str(b"gpgsig"), "tags have no signature headers");
    assert!(
        data.contains_str(b"-----BEGIN SSH SIGNATURE-----"),
        "the tag has an in-body SSH signature"
    );
    assert!(
        verify_ssh(&signed)?.is_valid(),
        "the native SHA-256 tag signature is valid"
    );
    Ok(())
}

fn tag(hash_kind: gix_hash::Kind) -> Tag {
    Tag {
        target: gix_hash::ObjectId::empty_tree(hash_kind),
        target_kind: Kind::Tree,
        name: "v1.0.0".into(),
        tagger: Some(gix_actor::Signature {
            name: "Gitoxide Signing Fixture".into(),
            email: signature::IDENTITY.into(),
            time: gix_date::Time::new(1_700_000_000, 0),
        }),
        message: "signed tag".into(),
        signature: None,
    }
}

fn git_verifies_openpgp(tag: &Tag, command_home: &std::ffi::OsStr) -> Result<bool> {
    use std::{io::Write as _, process::Stdio};

    let repo = gix_testtools::tempfile::TempDir::new()?;
    let init = std::process::Command::new(gix_path::env::exe_invocation())
        .args(["init", "--bare"])
        .arg(repo.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !init.success() {
        return Ok(false);
    }

    let mut data = Vec::new();
    tag.write_to(&mut data)?;
    let mut hash = std::process::Command::new(gix_path::env::exe_invocation())
        .arg("-C")
        .arg(repo.path())
        .args(["hash-object", "-t", "tag", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    hash.stdin
        .take()
        .expect("stdin was configured as piped")
        .write_all(&data)?;
    let hash = hash.wait_with_output()?;
    if !hash.status.success() {
        return Ok(false);
    }
    let tag_id = String::from_utf8(hash.stdout)?;
    Ok(std::process::Command::new(gix_path::env::exe_invocation())
        .arg("-C")
        .arg(repo.path())
        .args(["-c", "gpg.format=openpgp", "-c", "gpg.program=gpg", "verify-tag"])
        .arg(tag_id.trim())
        .env("GNUPGHOME", command_home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success())
}

fn verify(tag: &Tag, options: VerifyOptions) -> Result<Outcome> {
    let mut data = Vec::new();
    tag.write_to(&mut data)?;
    let (signature, signed) = TagRefIter::signature(&data).expect("the tag was just signed");
    Ok(signed.verify(signature.data, options)?)
}

fn verify_ssh(tag: &Tag) -> Result<Outcome> {
    verify(
        tag,
        ssh_verify_options(tag.tagger.as_ref().expect("the fixture has a tagger").time),
    )
}

fn ssh_verify_options(verify_time: gix_date::Time) -> VerifyOptions {
    VerifyOptions::Ssh {
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        environment: Vec::new(),
        allowed_signers: signature::fixture("ssh-allowed-signers"),
        revocation_file: None,
        verify_time,
        minimum_trust: TrustLevel::Undefined,
    }
}
