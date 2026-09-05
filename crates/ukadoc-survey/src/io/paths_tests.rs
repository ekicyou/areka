//! `io/paths.rs` の在中テスト。
//!
//! 守るのは 2 つ。⑴ ワークスペース根が「この crate の manifest の 2 段上」で実際に
//! repo の根を指していること。⑵ **綴りが逐語で正しいこと**——場所の関数は「何本あるか」
//! ではなく「どこを指すか」が本体なので、7 本すべてについて根からの相対パスを
//! 逐語で釘付けにする。ドメイン 4 種の綴り（`sakura-script` の横棒を含む）も 1 本ずつ
//! 対で並べる。件数だけを数えると 1 文字の入れ違いが素通りする。
//!
//! ファイルは 1 つも作らない。存在を確かめる読み取り（`is_file`）だけを行う
//! （設計 File Structure Plan）。

use super::*;
use crate::model::Domain;

/// ワークスペース根からの相対パスを `/` 区切りの文字列で返す。
fn rel(path: &Path) -> String {
    let root = workspace_root();
    let stripped = path
        .strip_prefix(&root)
        .unwrap_or_else(|_| panic!("根の下に無い: {} (根 {})", path.display(), root.display()));
    stripped.to_string_lossy().replace('\\', "/")
}

// ---- ワークスペース根 ----

/// 2 段上りが本当に repo の根を指していること。根の直下に自分自身の manifest がある。
#[test]
fn workspace_root_holds_this_crate_manifest() {
    let manifest = workspace_root()
        .join("crates")
        .join("ukadoc-survey")
        .join("Cargo.toml");
    assert!(
        manifest.is_file(),
        "ワークスペース根の求め方が違う: {} が無い",
        manifest.display()
    );
}

/// 根の直下にワークスペースの manifest がある（`crates/` の 1 段上ではない）。
#[test]
fn workspace_root_holds_the_workspace_manifest() {
    let root = workspace_root();
    assert!(
        root.join("Cargo.toml").is_file(),
        "ワークスペースの manifest が無い: {}",
        root.display()
    );
    assert!(
        root.join("crates").is_dir(),
        "crates/ が無い: {}",
        root.display()
    );
}

#[test]
fn workspace_root_is_absolute() {
    assert!(
        workspace_root().is_absolute(),
        "根は絶対パスでなければならない: {}",
        workspace_root().display()
    );
}

// ---- 7 本の場所を逐語で釘付けにする ----

#[test]
fn coverage_dir_is_verbatim() {
    assert_eq!(rel(&coverage_dir()), "doc/ukadoc-coverage");
}

#[test]
fn catalog_path_is_verbatim() {
    assert_eq!(rel(&catalog_path()), "doc/ukadoc-coverage/catalog.toml");
}

#[test]
fn values_path_is_verbatim() {
    assert_eq!(rel(&values_path()), "doc/ukadoc-coverage/values.md");
}

#[test]
fn summary_report_path_is_verbatim() {
    assert_eq!(
        rel(&summary_report_path()),
        "doc/ukadoc-coverage/report/summary.md"
    );
}

/// 台帳 4 本の綴りを 1 本ずつ対で並べる（`sakura-script` は横棒・拡張子は `.toml`）。
#[test]
fn ledger_paths_are_verbatim_for_every_domain() {
    let expected = [
        (Domain::Shiori, "doc/ukadoc-coverage/ledger/shiori.toml"),
        (Domain::Assets, "doc/ukadoc-coverage/ledger/assets.toml"),
        (
            Domain::SakuraScript,
            "doc/ukadoc-coverage/ledger/sakura-script.toml",
        ),
        (Domain::Property, "doc/ukadoc-coverage/ledger/property.toml"),
    ];
    for (domain, want) in expected {
        assert_eq!(rel(&ledger_path(domain)), want, "台帳の場所: {domain:?}");
    }
    assert_eq!(
        expected.len(),
        Domain::ALL.len(),
        "4 ドメインを網羅していない"
    );
}

/// ドメイン別報告 4 本の綴りを 1 本ずつ対で並べる（拡張子は `.md`）。
#[test]
fn domain_report_paths_are_verbatim_for_every_domain() {
    let expected = [
        (Domain::Shiori, "doc/ukadoc-coverage/report/shiori.md"),
        (Domain::Assets, "doc/ukadoc-coverage/report/assets.md"),
        (
            Domain::SakuraScript,
            "doc/ukadoc-coverage/report/sakura-script.md",
        ),
        (Domain::Property, "doc/ukadoc-coverage/report/property.md"),
    ];
    for (domain, want) in expected {
        assert_eq!(
            rel(&domain_report_path(domain)),
            want,
            "報告の場所: {domain:?}"
        );
    }
    assert_eq!(
        expected.len(),
        Domain::ALL.len(),
        "4 ドメインを網羅していない"
    );
}

// ---- 取り違えないこと ----

/// 4 ドメインの台帳・報告が互いに別の場所を指す（同じ場所を返す実装を弾く）。
#[test]
fn every_domain_gets_its_own_files() {
    let mut seen: Vec<String> = Vec::new();
    for domain in Domain::ALL {
        seen.push(rel(&ledger_path(domain)));
        seen.push(rel(&domain_report_path(domain)));
    }
    let mut sorted = seen.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), seen.len(), "場所が重なっている: {seen:?}");
}

/// 全体報告はドメイン別報告のどれとも別（要件 7.6 で常時検査の対象外なので取り違えない）。
#[test]
fn summary_report_is_not_a_domain_report() {
    for domain in Domain::ALL {
        assert_ne!(
            summary_report_path(),
            domain_report_path(domain),
            "全体報告がドメイン別報告と同じ場所: {domain:?}"
        );
    }
}

/// 7 本すべてがカタログ置き場の下にある。
#[test]
fn every_path_lives_under_the_coverage_dir() {
    let dir = coverage_dir();
    let mut paths = vec![catalog_path(), values_path(), summary_report_path()];
    for domain in Domain::ALL {
        paths.push(ledger_path(domain));
        paths.push(domain_report_path(domain));
    }
    for path in paths {
        assert!(
            path.starts_with(&dir),
            "カタログ置き場の外を指している: {}",
            path.display()
        );
    }
}
