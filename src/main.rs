use anyhow::{bail, Context, Result};
use caps::{CapSet, Capability};
use clap::Parser;
use nix::unistd::{Gid, Uid};
use pam::Client;
use zeroize::Zeroizing;

#[derive(Clone, Parser)]
struct Opts {
    #[clap(short, long, default_value = "example-rust-pam")]
    service: String,
    username: String,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    println!("using pam service: {}", opts.service);

    // stash original uid/gid - note;; not restoring suppl groups in this example
    let uid = Uid::effective();
    let gid = Gid::effective();

    // read password before elevating
    let password: Zeroizing<String> = Zeroizing::new(
        rpassword::prompt_password(format!("Enter the password for user {}: ", opts.username))
            .context("failed to read password from terminal")?,
    );

    // prepare the PAM client before elevating
    let mut auth = Client::with_password(&opts.service)
        .context("pam_start() failed -- is the service name valid? (check /etc/pam.d/)")?;
    auth.conversation_mut()
        .set_credentials(&opts.username, password.as_str());

    // ensure setuid / gid caps are permitted
    caps::has_cap(None, CapSet::Permitted, Capability::CAP_SETUID)?;
    caps::has_cap(None, CapSet::Permitted, Capability::CAP_SETGID)?;

    // activate caps
    caps::raise(None, CapSet::Effective, Capability::CAP_SETUID)?;
    caps::raise(None, CapSet::Effective, Capability::CAP_SETGID)?;

    //
    // elevate
    //
    nix::unistd::seteuid(Uid::from_raw(0)).context("seteuid 0 failed")?;
    nix::unistd::setegid(Gid::from_raw(0)).context("setegid 0 failed")?;
    nix::unistd::setgroups(&[]).context("clear suppl groups failed")?;

    // pam_authenticate() is the only call that actually needs root
    // it opens /etc/shadow (via pam_unix.so) to verify the password hash
    let auth_result = auth.authenticate();

    // ensure calling pam_end() while uid=0
    drop(auth);

    //
    // switch back to original user and drop caps
    //
    nix::unistd::seteuid(uid).context("restore setuid failed")?;
    nix::unistd::setegid(gid).context("restore setgid failed")?;
    caps::clear(None, CapSet::Effective)?;

    // zero the password now that auth is complete
    drop(password);

    match auth_result {
        Ok(()) => println!("✓ Authentication succeeded for '{}'.", opts.username),
        Err(e) => bail!("✗ Authentication failed for '{}': {e}", opts.username),
    }

    Ok(())
}
