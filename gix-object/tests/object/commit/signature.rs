use gix_object::{
    Commit, WriteTo,
    signature::{
        Format,
        sign::Options,
        verify::{Options as VerifyOptions, TrustLevel},
    },
};
use gix_testtools::signature;

use crate::Result;

#[test]
fn ssh() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (key_home, key) = signature::ssh_private_key()?;
    let signed = commit().sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    assert!(
        verify_ssh(&signed)?.is_valid(),
        "the plumbing verifier accepts the generated SSH signature"
    );
    drop(key_home);
    Ok(())
}

#[test]
fn openpgp() -> Result {
    if !signature::program_available("gpg") {
        return Ok(());
    }
    let home = signature::openpgp_home()?;
    let command_home = signature::path_for_command(home.path()).into_os_string();
    let signed = commit().sign(Options {
        format: Format::OpenPgp,
        program: "gpg".into(),
        // The fixture key is unprotected; fail instead of opening pinentry if that ever changes.
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
                environment: vec![("GNUPGHOME".into(), command_home)],
                minimum_trust: TrustLevel::Undefined,
            },
        )?
        .is_valid(),
        "the plumbing verifier accepts the generated OpenPGP signature"
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
    let signed = commit().sign(Options {
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
        "the plumbing verifier accepts the generated X.509 signature: {outcome:?}"
    );
    Ok(())
}

#[test]
fn replaces_the_active_signature() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (key_home, key) = signature::ssh_private_key()?;
    let mut commit = commit();
    commit.extra_headers.push(("before".into(), "one".into()));
    commit.extra_headers.push(("gpgsig".into(), "old".into()));
    commit.extra_headers.push(("after".into(), "two".into()));
    let signed = commit.sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    assert_eq!(
        signed
            .extra_headers
            .iter()
            .map(|(name, _)| name.as_slice())
            .collect::<Vec<_>>(),
        [b"before".as_slice(), b"after".as_slice(), b"gpgsig".as_slice()],
        "the old active signature is removed and its replacement is appended like Git"
    );
    assert!(
        verify_ssh(&signed)?.is_valid(),
        "the plumbing verifier accepts the replacement signature"
    );
    drop(key_home);
    Ok(())
}

#[test]
fn sha256_uses_its_git_signature_header() -> Result {
    if !signature::program_available("ssh-keygen") {
        return Ok(());
    }
    let (key_home, key) = signature::ssh_private_key()?;
    let mut commit = commit();
    commit.tree = gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha256);
    let signed = commit.sign(Options {
        format: Format::Ssh,
        program: "ssh-keygen".into(),
        program_arguments: Vec::new(),
        signing_key: key.into_os_string(),
        environment: Vec::new(),
    })?;
    assert_eq!(signed.extra_headers[0].0, "gpgsig-sha256");
    assert!(
        verify_ssh(&signed)?.is_valid(),
        "the plumbing verifier accepts the SHA-256 signature"
    );
    drop(key_home);
    Ok(())
}

fn commit() -> Commit {
    let actor = gix_actor::Signature {
        name: "Gitoxide Signing Fixture".into(),
        email: signature::IDENTITY.into(),
        time: gix_date::Time::new(1_700_000_000, 0),
    };
    Commit {
        tree: gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1),
        parents: Default::default(),
        author: actor.clone(),
        committer: actor,
        encoding: None,
        message: "signed commit\n".into(),
        extra_headers: Vec::new(),
    }
}

fn verify(commit: &Commit, options: VerifyOptions) -> Result<gix_object::signature::verify::Outcome> {
    let mut data = Vec::new();
    commit.write_to(&mut data)?;
    let (signature, signed) =
        gix_object::CommitRefIter::signature(&data, commit.tree.kind())?.expect("the commit was just signed");
    Ok(signed.verify(&signature, options)?)
}

fn verify_ssh(commit: &Commit) -> Result<gix_object::signature::verify::Outcome> {
    verify(
        commit,
        VerifyOptions::Ssh {
            program: "ssh-keygen".into(),
            program_arguments: Vec::new(),
            environment: Vec::new(),
            allowed_signers: signature::fixture("ssh-allowed-signers"),
            revocation_file: None,
            verify_time: commit.committer.time,
            minimum_trust: TrustLevel::Undefined,
        },
    )
}
