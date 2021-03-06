extern crate dkregistry;
extern crate futures;
extern crate tokio_core;

use self::dkregistry::mediatypes::MediaTypes;
use self::futures::stream::Stream;
use self::tokio_core::reactor::Core;

static REGISTRY: &'static str = "quay.io";

fn get_env() -> Option<(String, String)> {
    let user = ::std::env::var("DKREG_QUAY_USER");
    let password = ::std::env::var("DKREG_QUAY_PASSWD");
    match (user, password) {
        (Ok(u), Ok(t)) => Some((u, t)),
        _ => None,
    }
}

#[test]
fn test_dockerio_getenv() {
    if get_env().is_none() {
        println!(
            "[WARN] {}: missing DKREG_QUAY_USER / DKREG_QUAY_PASSWD",
            REGISTRY
        );
    }
}

#[test]
fn test_quayio_base() {
    let (user, password) = match get_env() {
        Some(t) => t,
        None => return,
    };

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(Some(user))
        .password(Some(password))
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_quayio_insecure() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);
}

#[test]
fn test_quayio_get_tags_simple() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, None);
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_limit() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, Some(10));
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_pagination() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/flannel";
    let fut_tags = dclient.get_tags(image, Some(20));
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "v0.10.0");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_has_manifest() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "latest";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = tcore.run(fut).unwrap();

    assert_eq!(has_manifest, Some(MediaTypes::ManifestV2S1Signed));
}

#[test]
fn test_quayio_has_no_manifest() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "clearly_bogus";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = tcore.run(fut).unwrap();

    assert_eq!(has_manifest, None);
}
