//! 正典イベントごとの凍結ゴールデンスナップショットを保持する静的表（task 1.3・design.md
//! "SnapshotTable"）。
//!
//! 本モジュールは `snapshots/<EventID>.txt`（SHIORI/3.0 応答**全文**・イベント 1 ファイル）を
//! [`include_str!`] で**コンパイル時に埋め込み**、実行時ファイル I/O をゼロにする（要件 1.5
//! 「実行時 I/O ゼロ」＝DLL は自分のファイル所在に依存しない決定論）。埋込文字列は取り込み時に
//! [`normalize_crlf`] で CRLF 行末へ正規化し、git の EOL 変換（`core.autocrlf` 等）に対する免疫を
//! 得る（要件 2.5 の忠実搬送／research.md §7.3）——on-disk の行末表現に一切依存しない。
//!
//! ## narrowing（design.md "SnapshotTable" の明文化）
//! スナップショット対象は **GET 交信のみ**。NOTIFY 発火の正典イベント（`OnInitialize`／
//! `basewareversion`／`OnClose` 等）は SHIORI ワイヤ契約上応答が破棄されるため採取不能であり、
//! replay 脳は決定論的な 204 受領で応答する（task 1.4 の `Notify`）。ゆえに本表は GET イベント
//! （`OnFirstBoot`＝204 で OnBoot 挨拶経路を駆動／`OnBoot`＝200＋挨拶さくらスクリプト）を鍵とする。
//!
//! ## PROVISIONAL マーカー規約（暫定データの明示・research.md §7.3）
//! 実 emo2 `pasta.dll` からの実採取（task 6.2）前の暫定応答は、SSP 寛容なカスタムヘッダ
//! `X-Areka-Snapshot: PROVISIONAL` を各ファイルへ帯同させ、**暫定であることを明示かつ機械検出可能**に
//! する。`parse_response`（`shiori-host32-host`）は未知ヘッダを無視するため、このマーカーはワイヤ上
//! inert（無害）であり、task 6.2 が実採取応答（マーカー無し）へ差し替える確定点となる。暫定応答は
//! 正準 SHIORI/3.0 形式（status line＋`Charset: UTF-8`（200 時）＋`Value:`＋空行終端）を保つ。
//!
//! 本モジュールの項目は crate 内消費（`pub(crate)`）。task 1.4 が [`snapshot_for`] を
//! `request::select_response` の `lookup` クロージャへ無改修で差し込む（`Fn(&str) -> Option<&'static str>`
//! を充足する）。本 task では `shiori_factory`／`ReplayBrain` へ未結線ゆえ、非テストビルドでは
//! 未使用となる（結線までの間 `dead_code` を意図的に許容する）。
#![allow(dead_code)]

use std::sync::OnceLock;

/// 埋め込み済みの生スナップショット（`(イベント ID, 応答全文)`・コンパイル時 `include_str!`）。
///
/// 実行時ファイル I/O を持たない（要件 1.5）。行末は取り込み時に [`normalize_crlf`] で正規化するため、
/// ここでの各 `include_str!` の on-disk 行末表現には依存しない。GET 交信のみを対象とする
/// （narrowing・上記モジュール doc）。
const RAW_SNAPSHOTS: &[(&str, &str)] = &[
    ("OnBoot", include_str!("../snapshots/OnBoot.txt")),
    ("OnFirstBoot", include_str!("../snapshots/OnFirstBoot.txt")),
];

/// 暫定応答に帯同する PROVISIONAL マーカーヘッダ（値レベルの単一権威）。
///
/// task 6.2 の実採取差し替えでこのヘッダは消える（実採取応答はマーカーを持たない）。
pub(crate) const PROVISIONAL_MARKER: &str = "X-Areka-Snapshot: PROVISIONAL";

/// 全ての行末表現（lone `\n`・`\r\n`）を一様な `\r\n` へ正規化する純関数。
///
/// **冪等**（`normalize_crlf(&normalize_crlf(x)) == normalize_crlf(x)`）であり、git の EOL 変換に
/// 対する免疫を与える（要件 2.5／research.md §7.3）——on-disk 行末に依存しない。
///
/// 実装は「全 `\r` を除去 → 全 `\n` を `\r\n` へ」の 2 段。既に CRLF の入力は 1 段目で LF のみへ、
/// 2 段目で CRLF へ戻るため不変（冪等）。空文字列は空文字列。
pub(crate) fn normalize_crlf(s: &str) -> String {
    // 「全 \r を除去 → 全 \n を \r\n へ」の 2 段。
    // 既に CRLF の入力は 1 段目で LF のみへ落ち、2 段目で CRLF へ戻るため不変＝冪等。
    s.replace('\r', "").replace('\n', "\r\n")
}

/// 凍結スナップショット表を CRLF 正規化済み・ID 昇順の決定論固定順で返す（要件 1.5）。
///
/// 参照は `static` [`OnceLock`] が保持する正規化済み文字列を指すため `'static`。同一入力（＝呼出）に
/// 常に同一参照を返す（事後条件）。
pub(crate) fn snapshots() -> &'static [(&'static str, &'static str)] {
    /// CRLF 正規化済みの表を一度だけ構築し保持する（`static` ゆえ参照は `'static`）。
    static TABLE: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: Vec<(&'static str, &'static str)> = RAW_SNAPSHOTS
            .iter()
            .map(|(id, raw)| {
                // 埋込文字列を CRLF 正規化し、所有 String を一度だけ leak して `'static` 化する
                // （表は有限小集合・初期化 1 回きり＝決定論・I/O ゼロ）。
                let normalized: &'static str = Box::leak(normalize_crlf(raw).into_boxed_str());
                (*id, normalized)
            })
            .collect();
        // ID 昇順の決定論固定順（返却スライスの順序不変・要件 1.5）。
        table.sort_by(|a, b| a.0.cmp(b.0));
        table
    })
}

/// ID に対応する CRLF 正規化済みの凍結応答を返す（未収載は `None`・要件 2.2）。
///
/// 返り値は `&'static str`（task 1.4 が `select_response` の `lookup: Fn(&str) -> Option<&'static str>`
/// へ無改修で差し込むため／`Selection::Snapshot(&'static str)` が保持するため）。
pub(crate) fn snapshot_for(id: &str) -> Option<&'static str> {
    snapshots()
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, response)| *response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalize_crlf の正規化＋冪等性檻（要件 2.5／research.md §7.3） ------------------------

    #[test]
    fn normalize_lf_only_becomes_crlf() {
        assert_eq!(normalize_crlf("a\nb\nc"), "a\r\nb\r\nc");
    }

    #[test]
    fn normalize_already_crlf_is_unchanged() {
        let already = "a\r\nb\r\nc";
        assert_eq!(normalize_crlf(already), already);
    }

    #[test]
    fn normalize_mixed_endings_unified() {
        // LF と CRLF の混在 → すべて CRLF に統一。
        assert_eq!(normalize_crlf("a\r\nb\nc\r\n"), "a\r\nb\r\nc\r\n");
    }

    #[test]
    fn normalize_is_idempotent() {
        for raw in ["a\nb", "a\r\nb", "a\r\nb\nc\r\n", "", "\n", "\r\n", "no endings"] {
            let once = normalize_crlf(raw);
            let twice = normalize_crlf(&once);
            assert_eq!(twice, once, "normalize_crlf は冪等であること（input={raw:?}）");
        }
    }

    #[test]
    fn normalize_empty_string_is_empty() {
        assert_eq!(normalize_crlf(""), "");
    }

    /// 正規化結果に lone `\n`（`\r` を伴わない `\n`）が残らないこと。
    fn assert_no_lone_lf(s: &str) {
        let bytes = s.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                assert!(
                    i > 0 && bytes[i - 1] == b'\r',
                    "位置 {i} の \\n が \\r を伴っていない（lone LF が残存）: {s:?}"
                );
            }
        }
    }

    // ---- snapshot_for / snapshots の決定論檻（要件 1.5／2.2） ------------------------------------

    #[test]
    fn snapshot_for_onboot_is_present_and_crlf_normalized() {
        let resp = snapshot_for("OnBoot").expect("OnBoot は収載されていること");
        // 全行末が CRLF（lone LF なし）。
        assert_no_lone_lf(resp);
        assert!(resp.contains("\r\n"), "整形済み応答は CRLF 行末を持つこと");
    }

    #[test]
    fn snapshot_for_onboot_is_200_with_value_line() {
        let resp = snapshot_for("OnBoot").unwrap();
        assert!(resp.contains("200"), "OnBoot は 200 応答であること: {resp:?}");
        assert!(resp.contains("Value:"), "OnBoot は Value 行を持つこと: {resp:?}");
    }

    #[test]
    fn snapshot_for_onfirstboot_is_204() {
        let resp = snapshot_for("OnFirstBoot").expect("OnFirstBoot は収載されていること");
        assert!(resp.contains("204"), "OnFirstBoot は 204 応答であること: {resp:?}");
        assert_no_lone_lf(resp);
    }

    #[test]
    fn snapshot_for_unknown_id_is_none() {
        assert_eq!(snapshot_for("SomethingUnknown"), None);
        assert_eq!(snapshot_for("OnInitialize"), None, "NOTIFY イベントは非収載（narrowing）");
        assert_eq!(snapshot_for(""), None);
    }

    #[test]
    fn snapshot_for_returns_same_reference_deterministically() {
        // 同一入力 → 同一参照（要件 1.5）。
        let a = snapshot_for("OnBoot").unwrap();
        let b = snapshot_for("OnBoot").unwrap();
        assert!(std::ptr::eq(a, b), "同一 ID は同一参照を返すこと（決定論）");
    }

    #[test]
    fn all_provisional_snapshots_carry_marker() {
        // 暫定であることが明示・機械検出可能であること（task 6.2 が消す確定点）。
        for (id, resp) in snapshots() {
            assert!(
                resp.contains(PROVISIONAL_MARKER),
                "暫定スナップショット {id} は PROVISIONAL マーカーを帯同すること: {resp:?}"
            );
        }
    }

    #[test]
    fn snapshots_ordering_is_deterministic_and_sorted() {
        let first = snapshots();
        let second = snapshots();
        // 同一参照（OnceLock）＝呼び出し間で不変。
        assert!(std::ptr::eq(first, second), "snapshots() は同一参照を返すこと");
        // ID 昇順の固定順（決定論）。
        let ids: Vec<&str> = first.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "snapshots() は ID 昇順の決定論固定順であること");
    }

    #[test]
    fn snapshots_covers_canonical_get_ids() {
        let ids: Vec<&str> = snapshots().iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"OnBoot"), "OnBoot（挨拶・GET）を収載");
        assert!(ids.contains(&"OnFirstBoot"), "OnFirstBoot（GET・204 で OnBoot 経路駆動）を収載");
    }

    #[test]
    fn snapshot_for_matches_table_entries() {
        // snapshots() の各エントリと snapshot_for() の解決が一致（単一表の 2 面）。
        for (id, resp) in snapshots() {
            assert_eq!(snapshot_for(id), Some(*resp), "{id} の 2 面が一致すること");
        }
    }
}
