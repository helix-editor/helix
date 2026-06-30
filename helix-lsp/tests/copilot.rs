//! End-to-end protocol test against a real `copilot-language-server`.
//!
//! This test is ignored by default because it spawns an external process. Run
//! it explicitly with the path to the binary:
//!
//! ```sh
//! HELIX_COPILOT_BIN=/path/to/copilot-language-server \
//!     cargo test -p helix-lsp --test copilot -- --ignored --nocapture
//! ```
//!
//! It verifies the full unauthenticated handshake but deliberately stops at the
//! device-flow prompt, so it never completes an irreversible sign-in.

use helix_lsp::copilot::{Client, FormattingOptions};
use helix_lsp::{lsp, Error};

fn binary() -> Option<String> {
    std::env::var("HELIX_COPILOT_BIN").ok()
}

#[tokio::test]
#[ignore = "spawns the real copilot-language-server; set HELIX_COPILOT_BIN"]
async fn copilot_protocol_e2e() {
    let Some(bin) = binary() else {
        eprintln!("HELIX_COPILOT_BIN not set; skipping");
        return;
    };

    let client = Client::start(&bin, &["--stdio".to_string()]).expect("failed to spawn copilot");

    // 1. initialize -> the server advertises an inline completion provider.
    let caps = client
        .initialize("Helix", "25.07.0", "0.1.0", None)
        .await
        .expect("initialize failed");
    assert!(
        caps.inline_completion_provider.is_some(),
        "expected inlineCompletionProvider in capabilities, got: {caps:?}"
    );
    println!(
        "[e2e] inlineCompletionProvider present: {:?}",
        caps.inline_completion_provider
    );

    // 2. checkStatus -> NotSignedIn for a fresh, unauthenticated server.
    let status = client.check_status().await.expect("checkStatus failed");
    println!("[e2e] checkStatus -> {status:?}");
    assert_eq!(status.status, "NotSignedIn");

    // 3. inlineCompletion before sign-in -> JSON-RPC error 1000 (NotSignedIn).
    let uri = lsp::Url::parse("file:///tmp/helix-copilot-e2e.py").unwrap();
    client.did_open(&uri, 1, "python", "def add(a, b):\n    ");
    let err = client
        .inline_completion(&uri, 1, 1, 4, FormattingOptions::default())
        .await
        .expect_err("expected NotSignedIn error before sign-in");
    match &err {
        Error::Rpc(rpc) => {
            println!(
                "[e2e] inlineCompletion pre-signin -> error {}: {}",
                rpc.code.code(),
                rpc.message
            );
            assert_eq!(rpc.code.code(), 1000, "expected error code 1000");
        }
        other => panic!("expected an RPC error, got: {other:?}"),
    }

    // 4. signIn -> a real device code the user could enter at the verification
    //    URI. We stop here and never finish the device flow.
    let sign_in = client.sign_in().await.expect("signIn failed");
    println!("[e2e] signIn -> {sign_in:?}");
    assert_eq!(sign_in.status, "PromptUserDeviceFlow");
    assert!(sign_in.user_code.is_some(), "expected a device user code");
    assert!(
        sign_in.verification_uri.is_some(),
        "expected a verification URI"
    );
    let command = sign_in
        .command
        .expect("expected a finishDeviceFlow command");
    assert_eq!(command.command, "github.copilot.finishDeviceFlow");
    println!(
        "[e2e] device code {} at {} (NOT completing sign-in)",
        sign_in.user_code.unwrap(),
        sign_in.verification_uri.unwrap()
    );
}
