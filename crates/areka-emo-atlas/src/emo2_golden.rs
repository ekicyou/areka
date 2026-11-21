//! emo2 fixture の bake 出力に対する決定性（golden）テスト。
//!
//! 実 fixture emo2 の shell surface 一式を bake し、その配置結果
//! （各キーの `(page, uv_rect, trim_offset, original)`）が
//! **固定 golden スナップショットに一致**することを検証する。
//! 毎回の実行が固定の golden と一致すれば run-to-run 決定性も成り立つので、
//! 2 回焼いて比べる別のテストは置かない。失敗 0 件は `emo2_e2e.rs` の最初のテストが見る。
//!
//! 本物の焼きはこのファイルで 1 回だけ。WIC 腕越しに実 PNG を復号する。
//!
//! ## golden が機械非依存で安定する理由
//! - 画像の寸法・α は **PNG のコンテンツから決まる**（WIC はバイト列を忠実に復号する
//!   だけで、寸法/画素は WIC 内部の非決定的状態に依存しない）。
//! - α-bbox トリムは復号画素に対する決定的計算。
//! - `rectangle-pack` の配置は入力が整列済みなら決定的（`Packer` は最終 `ElementId`
//!   ＝ manifest 昇順で採番済みの安定入力を渡す）。
//! - マニフェスト採番は `SetId` 昇順・rel_path 昇順で決定的。
//!
//! ## golden の再生成手順
//! 配置アルゴリズムを意図的に変更して golden を更新するときは、
//! 無視指定の `record_golden` テストを明示的に実行する:
//!
//! ```text
//! cargo test -p areka-emo-atlas emo2_golden::record_golden -- --ignored --nocapture
//! ```
//!
//! これが `src/testdata/emo2_shell_golden.txt` を上書きする。通常の `cargo test` では
//! `#[ignore]` ゆえ実行されず、CI が golden を書き換えることはない。

use crate::test_support::{emo2_shell, set, with_arm};
use crate::{AtlasTable, ElementId, PackConfig, WicDecoderArm, bake};

/// emo2 shell surfaces.txt をパースし、`SurfaceSet` として **1 度** bake した結果を返す。
///
/// 読み手は呼び出し側が COM を張った下で組んで渡す（`with_arm`）。
fn bake_emo2_shell(dec: &WicDecoderArm) -> crate::BakeResult {
    let (surfaces, base) = emo2_shell();
    bake(&[set(&surfaces, &base)], dec, PackConfig::default())
}

/// `&AtlasTable` の全エントリを **`AtlasKey`（set → rel_path）昇順**で並べた決定的
/// スナップショット文字列を生成する。
///
/// 1 エントリ 1 行、タブ区切りで
/// `<set>\t<rel_path>\t<placement>` を出力する。`<placement>` は
/// - `entry.placement == None`（全透明）→ `EMPTY orig=<ow>x<oh>`
/// - `Some(Placement)` → `page=<p> uv=<x>,<y>,<w>,<h> off=<ox>,<oy> orig=<ow>x<oh>`
fn snapshot_table(table: &AtlasTable) -> String {
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

/// golden スナップショット・リグレッション検出（決定性も兼ねる）。
///
/// emo2 shell を 1 度 bake し、その配置スナップショットをコミット済み golden
/// （`src/testdata/emo2_shell_golden.txt`）と比較する。
///
/// golden は 55 行＝参照される全 element を持つ。`purple/a/null.png` は番号ごとの透明度を
/// 持つパレットの絵（番号 1 が透明度 0）として α の道を通り、全画素が透明になるため `0\tpurple/a/null.png\tEMPTY orig=382x547` の
/// 行として載る（脱落ではない）。残る 54 行は α チャンネルの腕を通った絵で、
/// 抜き色の腕の追加によって 1 文字も変わらない。
///
/// golden の再生成はモジュール doc の手順に従い、生成物をコミットする。
#[test]
fn emo2_shell_matches_golden() {
    let expected = include_str!("testdata/emo2_shell_golden.txt");
    // golden は git の autocrlf でチェックアウト時に CRLF 化されうる（この repo は
    // Cargo.lock を含め text を CRLF 正規化する）。snapshot_table は常に LF ゆえ、
    // 行内改行を LF へ正規化＋末尾改行除去してから比較し、配置値の等価性のみを見る。
    // これがないと autocrlf 有効な新規クローン（Windows）で改行差だけで誤検出する。
    let expected = expected.replace("\r\n", "\n");
    let expected = expected.trim_end_matches('\n');
    with_arm(|dec| {
        let result = bake_emo2_shell(dec);
        let actual = snapshot_table(&result.table);
        assert_eq!(
            actual, expected,
            "emo2 shell placement snapshot regressed against committed golden \
             (src/testdata/emo2_shell_golden.txt); regenerate intentionally via the \
             #[ignore] record_golden test if the change is expected"
        );
    });
}

/// golden 再生成用の無視指定テスト。意図的な再生成時のみ `-- --ignored --nocapture` で明示実行する。
///
/// 実行すると emo2 shell を bake し、配置スナップショットを
/// `src/testdata/emo2_shell_golden.txt` へ書き込む（末尾改行 1 つ付き）。
#[test]
#[ignore = "golden regeneration only; run explicitly with --ignored"]
fn record_golden() {
    with_arm(|dec| {
        let result = bake_emo2_shell(dec);
        let snapshot = snapshot_table(&result.table);
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/testdata/emo2_shell_golden.txt"
        );
        std::fs::write(path, format!("{snapshot}\n")).expect("write golden snapshot");
        eprintln!("wrote golden ({} bytes) to {path}", snapshot.len());
    });
}
