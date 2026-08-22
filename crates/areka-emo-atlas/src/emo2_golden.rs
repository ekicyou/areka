//! emo2 fixture の bake 出力に対する決定性（golden）テスト（task 4.1・要件 **5.5**）。
//!
//! 既に構築済みの bake パイプライン（マニフェスト導出 → デコード → 正規化 → トリム →
//! packing → 焼付）を、実 fixture emo2 の shell surface 一式で駆動し、その配置結果
//! （各キーの `(page, uv_rect, trim_offset, original)`）が
//!
//! 1. **同一入力を複数回 bake しても常に同一**（run-to-run 決定性・5.5 中核）
//! 2. **固定 golden スナップショットに一致**（以降のリグレッションを検出）
//!
//! であることを検証する。production コードは変更せず（本モジュールは `#[cfg(test)]`）、
//! WIC 腕（COM 隔離）越しに実 PNG を復号する。
//!
//! ## golden が機械非依存で安定する理由（「継続的実行環境で安定して再現」）
//! - 画像の寸法・α は **PNG のコンテンツから決まる**（WIC はバイト列を忠実に復号する
//!   だけで、寸法/画素は WIC 内部の非決定的状態に依存しない）。
//! - α-bbox トリムは復号画素に対する決定的計算。
//! - `rectangle-pack` の配置は入力が整列済みなら決定的（`Packer` は最終 `ElementId`
//!   ＝ manifest 昇順で採番済みの安定入力を渡す）。
//! - マニフェスト採番は `SetId` 昇順・rel_path 昇順で決定的（D7）。
//!
//! ゆえに run-to-run 等値と固定 golden スナップショットの双方が、実行環境に依らず
//! 安定・再現する。
//!
//! ## golden の再生成手順
//! 配置アルゴリズムを意図的に変更した場合など golden を更新する必要が生じたら、
//! 無視指定の `record_golden` テストを明示的に実行する:
//!
//! ```text
//! cargo test -p areka-emo-atlas emo2_golden::record_golden -- --ignored --nocapture
//! ```
//!
//! これが `src/testdata/emo2_shell_golden.txt` を上書きする。通常の `cargo test` では
//! `#[ignore]` ゆえ実行されず、CI が golden を書き換えることはない。

use std::path::PathBuf;

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::{
    AlphaParams, AtlasTable, ElementId, PackConfig, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};

/// COM 初期化下でクロージャを実行するヘルパ（WIC は CPU-only だが COM init 必須）。
/// `emo2_e2e.rs` の同名ヘルパと同一パターン（MTA・RPC_E_CHANGED_MODE 許容）。
fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        // 既に初期化済みでも RPC_E_CHANGED_MODE を許容し、続行する。
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// emo2 fixture 実資産のパスを組む。
/// `CARGO_MANIFEST_DIR` = `crates/areka-emo-atlas`。fixtures はパイロット crate 配下。
fn emo2(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
        .join(rel)
}

/// emo2 の shell/master 基準 dir（surfaces.txt の element 相対パス起点）。
fn shell_master_dir() -> PathBuf {
    emo2("shell/master")
}

/// AlphaParams { use_self_alpha: On }（emo2 の descript は seriko.use_self_alpha,1）。
fn on_params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// emo2 shell surfaces.txt をパースし、`SurfaceSet` として **1 度** bake した結果を返す。
///
/// COM は呼び出し側で初期化済みであること（本関数は WIC 腕を生成し bake するのみ）。
/// 毎回この関数を呼ぶことで「新規構築した同一入力」を得られ、run-to-run 決定性の
/// 検証に用いる。
fn bake_emo2_shell() -> crate::BakeResult {
    let content = std::fs::read_to_string(emo2("shell/master/surfaces.txt"))
        .expect("emo2 surfaces.txt must be readable (UTF-8)");
    let shell = areka_parsers::shell::parse(&content);
    assert!(
        !shell.surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );

    let base = shell_master_dir();
    let dec = WicDecoderArm::new().expect("WIC factory creates under COM init");
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &base,
        alpha_params: on_params(),
    };
    // freshly-constructed identical inputs + PackConfig::default()。
    bake(std::slice::from_ref(&set), &dec, PackConfig::default())
}

/// `&AtlasTable` の全エントリを **`AtlasKey`（set → rel_path）昇順**で並べた決定的
/// スナップショット文字列を生成する。
///
/// 1 エントリ 1 行、タブ区切りで
/// `<set>\t<rel_path>\t<placement>` を出力する。`<placement>` は
/// - `entry.placement == None`（全透明）→ `EMPTY orig=<ow>x<oh>`
/// - `Some(Placement)` → `page=<p> uv=<x>,<y>,<w>,<h> off=<ox>,<oy> orig=<ow>x<oh>`
///
/// これにより各キーの `(page, uv_rect, trim_offset, original)` を design 要求どおり
/// 捕捉する。human-diffable な安定書式を保つ。
fn snapshot_table(table: &AtlasTable) -> String {
    // 全 ElementId を列挙し (key, entry) を収集。
    let mut rows: Vec<(u32, String, String)> = Vec::with_capacity(table.len());
    for i in 0..table.len() {
        let id = ElementId(i as u32);
        let key = table.key(id);
        let entry = table.entry(id);
        let orig = entry.original;
        let placement = match &entry.placement {
            None => format!("EMPTY orig={}x{}", orig.w, orig.h),
            Some(p) => format!(
                "page={} uv={},{},{},{} off={},{} orig={}x{}",
                p.page,
                p.uv_rect.x,
                p.uv_rect.y,
                p.uv_rect.w,
                p.uv_rect.h,
                p.trim_offset.x,
                p.trim_offset.y,
                orig.w,
                orig.h,
            ),
        };
        rows.push((key.set.0, key.rel_path.clone(), placement));
    }
    // AtlasKey 昇順 = (set, rel_path) 昇順で整列（安定順序・golden 決定性）。
    rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));

    let mut out = String::new();
    for (i, (set, rel, placement)) in rows.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{set}\t{rel}\t{placement}"));
    }
    out
}

/// エラー集合の決定的スナップショット（run-to-run 比較用）。`Debug` を行区切りで並べ、
/// 安定比較のため整列する。
fn snapshot_errors(errors: &[crate::error::BakeError]) -> String {
    let mut lines: Vec<String> = errors.iter().map(|e| format!("{e:?}")).collect();
    lines.sort();
    lines.join("\n")
}

/// テスト①: run-to-run 決定性（5.5 中核・golden ファイル非依存で完全に堅牢）。
///
/// 同一 emo2 shell 入力を **2 回** 独立に bake し、配置結果スナップショットと
/// エラー集合が**バイト等値**であることを確認する。ハードコード座標に依存せず、
/// 2 つの独立実行の等値のみで決定性（5.5）を実証する。
#[test]
fn emo2_shell_bake_is_deterministic() {
    with_com_initialized(|| {
        let a = bake_emo2_shell();
        let b = bake_emo2_shell();

        let snap_a = snapshot_table(&a.table);
        let snap_b = snapshot_table(&b.table);
        assert_eq!(
            snap_a, snap_b,
            "same emo2 shell input must bake to an identical placement snapshot (5.5)"
        );

        // 失敗集合も同一（同一の null.png 正規化 seam が両実行で 1 件だけ）。
        let err_a = snapshot_errors(&a.errors);
        let err_b = snapshot_errors(&b.errors);
        assert_eq!(
            err_a, err_b,
            "same emo2 shell input must yield an identical error set run-to-run"
        );

        // 空でない実配置が存在する（スナップショットが空文字列でない＝空虚な等値でない）。
        assert!(
            !snap_a.is_empty(),
            "snapshot must contain entries (non-vacuous determinism)"
        );
    });
}

/// テスト②: golden スナップショット・リグレッション検出。
///
/// emo2 shell を 1 度 bake し、その配置スナップショットをコミット済み golden
/// （`src/testdata/emo2_shell_golden.txt`）と比較する。不一致（配置アルゴリズムの
/// 意図しない変更）は `assert_eq!` が actual/expected を並べて検出する。
///
/// golden の再生成が必要な場合はモジュール doc の手順（無視指定 `record_golden` を
/// `--ignored` 付きで実行）に従い、生成物をコミットする。
#[test]
fn emo2_shell_matches_golden() {
    let expected = include_str!("testdata/emo2_shell_golden.txt");
    // golden は git の autocrlf でチェックアウト時に CRLF 化されうる（この repo は
    // Cargo.lock を含め text を CRLF 正規化する）。snapshot_table は常に LF ゆえ、
    // 行内改行を LF へ正規化＋末尾改行除去してから比較し、配置値の等価性のみを見る。
    // これがないと autocrlf 有効な新規クローン（Windows）で改行差だけで誤検出する。
    let expected = expected.replace("\r\n", "\n");
    let expected = expected.trim_end_matches('\n');
    with_com_initialized(|| {
        let result = bake_emo2_shell();
        let actual = snapshot_table(&result.table);
        assert_eq!(
            actual, expected,
            "emo2 shell placement snapshot regressed against committed golden \
             (src/testdata/emo2_shell_golden.txt); regenerate intentionally via the \
             #[ignore] record_golden test if the change is expected"
        );
    });
}

/// golden 再生成用の無視指定テスト。通常の `cargo test` では `#[ignore]` ゆえ実行
/// されず、CI が golden を書き換えることはない。意図的な再生成時のみ
/// `-- --ignored --nocapture` で明示実行する。
///
/// 実行すると emo2 shell を bake し、配置スナップショットを
/// `src/testdata/emo2_shell_golden.txt` へ書き込む（末尾改行 1 つ付き）。
#[test]
#[ignore = "golden regeneration only; run explicitly with --ignored"]
fn record_golden() {
    with_com_initialized(|| {
        let result = bake_emo2_shell();
        let snapshot = snapshot_table(&result.table);
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/testdata/emo2_shell_golden.txt"
        );
        // 末尾改行 1 つを付けて POSIX テキストとして書き込む。
        std::fs::write(path, format!("{snapshot}\n")).expect("write golden snapshot");
        eprintln!("wrote golden ({} bytes) to {path}", snapshot.len());
    });
}
