//! `image-webp` を固定コミットから取り込んでいることの検査（spec: areka-P0-animated-image-decode 要件 7.6）。
//!
//! `[patch.crates-io]` を足しただけでは cargo は警告だけ出して公開版を使い続ける（`research.md` 9 節の実測）。
//! そこで根の `Cargo.lock` と `Cargo.toml` の文を読んで判定する。ネットは使わない。
//! 外す条件: `image-webp` 0.2.5 以上が crates.io に出たら、このファイルごと外す（`tech.md` の登記）。

/// 取り込む固定コミット（枝 `release-0.2.5` の先頭）。変えるときは根の `Cargo.toml` と一緒に変える。
const PINNED: &str = "75f810915d02ae4ff55d3f825bf6a4b07efdf994";

const LOCK: &str = include_str!("../../../Cargo.lock");
const MANIFEST: &str = include_str!("../../../Cargo.toml");

/// `Cargo.lock` と根の `Cargo.toml` の文から、取り込みが効いているかを判定する。
fn judge_pin(lock: &str, manifest: &str) -> Result<(), String> {
    if lock.lines().any(|l| l.trim_end() == "[[patch.unused]]") {
        return Err(
            "Cargo.lock に [[patch.unused]] がある（取り込みが効いていない・`cargo update -p image-webp` が要る）"
                .into(),
        );
    }
    // 項目は `[[` で始まる行ごとに区切る。
    let entries: Vec<&str> = lock
        .split("\n[[")
        .filter(|e| e.lines().any(|l| l.trim_end() == r#"name = "image-webp""#))
        .collect();
    let [entry] = entries[..] else {
        return Err(format!(
            "Cargo.lock の image-webp の項目が {} 個（1 個であるべき）",
            entries.len()
        ));
    };
    let want =
        format!(r#"source = "git+https://github.com/image-rs/image-webp?rev={PINNED}#{PINNED}""#);
    if !entry.lines().any(|l| l.trim_end() == want) {
        return Err(format!(
            "Cargo.lock の image-webp の source が固定コミットの git でない（期待: {want}）: {entry}"
        ));
    }
    let want_rev = format!(r#"rev = "{PINNED}""#);
    if !manifest
        .lines()
        .any(|l| l.starts_with("image-webp") && l.contains(&want_rev))
    {
        return Err(format!(
            "根の Cargo.toml の image-webp の rev が固定コミットでない（期待: {want_rev}）"
        ));
    }
    Ok(())
}

#[test]
fn the_repository_takes_image_webp_from_the_pinned_commit() {
    judge_pin(LOCK, MANIFEST).unwrap();
}

// ---- 較正: 赤になるべき文を渡して、赤になり、理由が合っていることを判定する ----

/// 今の `Cargo.lock` の `image-webp` の項目の頭（固定コミットの git）。
fn git_head() -> String {
    format!(
        "name = \"image-webp\"\nversion = \"0.2.5\"\nsource = \"git+https://github.com/image-rs/image-webp?rev={PINNED}#{PINNED}\""
    )
}

/// 取り込み前（task 1.1 の前のコミット）の `Cargo.lock` の `image-webp` の項目の頭（公開版）。
const PUBLIC_HEAD: &str = "name = \"image-webp\"\nversion = \"0.2.4\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"525e9ff3e1a4be2fbea1fdf0e98686a6d98b4d8f937e1bf7402245af1909e8c3\"";

/// 今の `Cargo.lock` の一部を置き換えた文。置き換えが空振りしたら較正にならないので落とす。
fn lock_with(from: &str, to: &str) -> String {
    let lock = LOCK.replace("\r\n", "\n");
    let out = lock.replace(from, to);
    assert_ne!(out, lock, "較正の置き換えが空振りした: {from}");
    out
}

fn assert_red(lock: &str, manifest: &str, reason: &str) {
    let err = judge_pin(lock, manifest).expect_err("赤になるべき文が緑と判定された");
    assert!(
        err.contains(reason),
        "理由が違う（{reason} を含まない）: {err}"
    );
}

#[test]
fn calibration_public_source_is_red() {
    assert_red(&lock_with(&git_head(), PUBLIC_HEAD), MANIFEST, "source");
}

#[test]
fn calibration_patch_unused_is_red() {
    // `[patch.crates-io]` を足しただけで `cargo update` をしなかった状態の実物（`research.md` 9 節）:
    // 項目は公開版のまま、末尾に `[[patch.unused]]` が付く。
    let unused = format!(
        "\n[[patch.unused]]\nname = \"image-webp\"\nversion = \"0.2.5\"\nsource = \"git+https://github.com/image-rs/image-webp?rev={PINNED}#{PINNED}\"\n"
    );
    let lock = lock_with(&git_head(), PUBLIC_HEAD) + &unused;
    assert_red(&lock, MANIFEST, "[[patch.unused]]");
}

#[test]
fn calibration_other_commit_in_lock_is_red() {
    let other = "0000000000000000000000000000000000000000";
    assert_red(&lock_with(PINNED, other), MANIFEST, "source");
}

#[test]
fn calibration_two_entries_is_red() {
    // 公開版と固定コミットの git が並んで在る（どちらかが別の依存から引かれている）状態。
    let both = format!("{PUBLIC_HEAD}\n\n[[package]]\n{}", git_head());
    assert_red(&lock_with(&git_head(), &both), MANIFEST, "1 個であるべき");
}

#[test]
fn calibration_other_commit_in_manifest_is_red() {
    let other = "0000000000000000000000000000000000000000";
    let manifest = MANIFEST.replace(PINNED, other);
    assert_ne!(manifest, MANIFEST, "較正の置き換えが空振りした");
    assert_red(LOCK, &manifest, "根の Cargo.toml");
}
