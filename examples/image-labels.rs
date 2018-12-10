extern crate dirs;
extern crate dkregistry;
extern crate futures;
extern crate serde_json;
extern crate tokio_core;

use dkregistry::reference;
use futures::prelude::*;
use std::result::Result;
use std::str::FromStr;
use std::{env, fs, io};
use tokio_core::reactor::Core;

mod common;

fn main() {
    let dkr_ref = match std::env::args().nth(1) {
        Some(ref x) => reference::Reference::from_str(x),
        None => reference::Reference::from_str("quay.io/steveej/cincinnati-test-labels:0.0.0"),
    }
    .unwrap();
    let registry = dkr_ref.registry();

    println!("[{}] downloading image {}", registry, dkr_ref);

    let mut user = None;
    let mut password = None;
    let home = dirs::home_dir().unwrap();
    let cfg = fs::File::open(home.join(".docker/config.json"));
    if let Ok(fp) = cfg {
        let creds = dkregistry::get_credentials(io::BufReader::new(fp), &registry);
        if let Ok(user_pass) = creds {
            user = user_pass.0;
            password = user_pass.1;
        } else {
            println!("[{}] no credentials found in config.json", registry);
        }
    } else {
        user = env::var("DKREG_USER").ok();
        if user.is_none() {
            println!("[{}] no $DKREG_USER for login user", registry);
        }
        password = env::var("DKREG_PASSWD").ok();
        if password.is_none() {
            println!("[{}] no $DKREG_PASSWD for login password", registry);
        }
    };

    let res = run(&dkr_ref, user, password);

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

fn run(
    dkr_ref: &reference::Reference,
    user: Option<String>,
    passwd: Option<String>,
) -> Result<(), dkregistry::errors::Error> {
    let mut tcore = Core::new()?;

    let mut client = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(&dkr_ref.registry())
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let image = dkr_ref.repository();
    let login_scope = format!("repository:{}:pull", image);
    let version = dkr_ref.version();

    let futures = common::authenticate_client(&mut client, &login_scope)
        .and_then(|dclient| {
            dclient
                .has_manifest(&image, &version, None)
                .and_then(move |manifest_option| Ok((dclient, manifest_option)))
                .and_then(|(dclient, manifest_option)| match manifest_option {
                    None => Err(format!("{}:{} doesn't have a manifest", &image, &version).into()),

                    Some(manifest_kind) => Ok((dclient, manifest_kind)),
                })
        })
        .and_then(|(dclient, manifest_kind)| {
            let image = image.clone();
            dclient
                .get_manifest(&image, &version)
                .and_then(move |manifest_body| {
                    let m: Result<_, _> = match manifest_kind {
                        dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed => {
                            let m = serde_json::from_slice::<
                                dkregistry::v2::manifest::ManifestSchema1Signed,
                            >(manifest_body.as_slice())?;
                            Ok(m)
                        }
                        dkregistry::mediatypes::MediaTypes::ManifestV2S2 => {
                            let _ = serde_json::from_slice::<
                                dkregistry::v2::manifest::ManifestSchema2,
                            >(manifest_body.as_slice())?;
                            Err("manifest V2.1 not implemented".into())
                        }
                        _ => Err("unknown format".into()),
                    };
                    m
                })
        });

    let manifest = match tcore.run(futures) {
        Ok(manifest) => Ok(manifest),
        Err(e) => Err(format!("Got error {}", e)),
    }?;

    match manifest.get_labels(0) {
        Some(labels) => {
            println!("got labels: {:#?}", labels);
            println!("channel label: {:#?}", labels.get("channel"));
        }
        None => println!("got no labels"),
    }

    Ok(())
}
