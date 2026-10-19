use std::net::TcpListener;

use super::*;

/// One test: the claim and the Cancel flag are process-wide.
#[tokio::test]
async fn cancel_hits_only_the_running_download() {
    let moving = claim_for_move().expect("free at start");
    assert!(
        claim(Holder::Download).is_none(),
        "a move and a download never overlap"
    );
    cancel_detectors_download();
    assert!(
        !CANCEL.load(Ordering::SeqCst),
        "a Cancel during a move is ignored"
    );
    drop(moving);
    cancel_detectors_download();
    assert!(
        !CANCEL.load(Ordering::SeqCst),
        "a Cancel when idle is ignored"
    );

    // A server that accepts and never answers: the Cancel comes before the first chunk.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/ort.tgz", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let _held: Vec<_> = listener.incoming().take(1).collect();
        std::thread::sleep(Duration::from_secs(30));
    });
    let busy = claim(Holder::Download).expect("free after the move");
    let run = tokio::spawn(async move {
        let mut ignore = |_, _| {};
        ml::fetch::fetch(&url, &Default::default(), &mut ignore, &CANCEL).await
    });
    cancel_detectors_download();
    let err = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .expect("Cancel right after start stops the download")
        .unwrap()
        .unwrap_err();
    assert!(err.to_string().contains(ml::fetch::CANCELLED), "{err}");
    drop(busy);
    assert!(
        !CANCEL.load(Ordering::SeqCst),
        "the next download is not pre-cancelled"
    );
    assert!(claim(Holder::Download).is_some(), "freed when dropped");
}

/// A fresh launch with a runtime that won't load reports the failure, not "Running on cpu".
#[cfg(feature = "ml")]
#[tokio::test]
async fn status_reports_a_runtime_that_wont_load() {
    let dir = tempfile::tempdir().unwrap();
    let spec = ml::runtime::current().unwrap();
    let lib = ml::runtime::lib_path(dir.path(), &spec);
    std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
    std::fs::write(&lib, b"not a library").unwrap();
    for m in &ml::MODELS {
        std::fs::write(ml::cache::model_path(dir.path(), m), b"not a model").unwrap();
    }
    let status = status_in(dir.path().to_path_buf()).await.unwrap();
    assert!(!status.downloaded, "the switch stays off");
    match status.runtime {
        ml::RuntimeStatus::Failed { reason, .. } => assert!(!reason.is_empty()),
        other => panic!("{other:?}"),
    }
}

/// A Remove refused by another holder, or failing half-way, leaves the lease free (#159/#163).
#[tokio::test]
async fn remove_releases_the_lease_on_error() {
    use ml::lease::{acquire, Purpose};
    let dir = tempfile::tempdir().unwrap();
    let models = dir.path().to_path_buf();
    let model = ml::cache::model_path(&models, &ml::MODELS[0]);
    std::fs::create_dir_all(model.parent().unwrap()).unwrap();
    std::fs::write(&model, b"model").unwrap();

    let other = acquire(&models, Purpose::Download).unwrap();
    let err = remove_in(models.clone()).await.unwrap_err();
    assert!(model.exists(), "refused, nothing deleted: {err}");
    drop(other);

    // A model path that is a folder makes `remove_file` fail half-way.
    std::fs::remove_file(&model).unwrap();
    std::fs::create_dir(&model).unwrap();
    std::fs::write(model.join("x"), b"x").unwrap();
    remove_in(models.clone()).await.unwrap_err();
    drop(acquire(&models, Purpose::Remove).expect("released after the error"));

    std::fs::remove_dir_all(&model).unwrap();
    std::fs::write(&model, b"model").unwrap();
    assert_eq!(remove_in(models.clone()).await.unwrap(), 5);
    assert!(!model.exists());
    drop(acquire(&models, Purpose::Remove).expect("released after the delete"));
}
