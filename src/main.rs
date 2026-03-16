use anyhow::{bail, Context, Result};
use clap::Parser;
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
    let orig_uid = unsafe { libc::getuid() as i32 };
    let orig_gid = unsafe { libc::getgid() as i32 };

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

    //
    // elevate
    //
    capng::get_caps_process()?;
    capng::change_id(0, 0, capng::Flags::DROP_SUPP_GRP)
        .context("capng::change_id(0,0) failed -- is cap_setuid,cap_setgid=ep set on binary?")?;

    // pam_authenticate() is the only call that actually needs root
    // it opens /etc/shadow (via pam_unix.so) to verify the password hash
    let auth_result = auth.authenticate();

    // ensure calling pam_end() while uid=0
    drop(auth);

    //
    // drop privileges and switch back to original user
    //
    capng::get_caps_process()?;
    capng::change_id(orig_uid, orig_gid, capng::Flags::DROP_SUPP_GRP)
        .context("capng::change_id(orig) failed while dropping privileges")?;
    capng::clear(capng::Set::all());
    capng::apply(capng::Set::all()).context("failed to apply cleared capability sets")?;

    // zero the password now that PAM is done with it.
    drop(password);

    match auth_result {
        Ok(()) => println!("✓ Authentication succeeded for '{}'.", opts.username),
        Err(e) => bail!("✗ Authentication failed for '{}': {e}", opts.username),
    }

    Ok(())
}
