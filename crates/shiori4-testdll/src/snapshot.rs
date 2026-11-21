//! 正典イベントごとの凍結ゴールデンスナップショットを保持する静的表。
//!
//! `snapshots/<EventID>.txt`（SHIORI/3.0 応答**全文**・イベント 1 ファイル）を [`include_str!`] で
//! **コンパイル時に埋め込み**、実行時ファイル I/O をゼロにする（DLL は自分のファイル所在に依存しない）。
//! 埋込文字列は取り込み時に [`normalize_crlf`] で CRLF 行末へ正規化し、git の EOL 変換
//! （`core.autocrlf` 等）に依存しない。
//!
//! ## narrowing
//! スナップショット対象は **GET 交信のみ**。NOTIFY 発火の正典イベント（`OnInitialize`／
//! `basewareversion`／`OnClose` 等）は SHIORI ワイヤ契約上応答が破棄されるため採取不能であり、
//! replay 脳は決定論的な 204 受領で応答する（`Notify`）。
//!
//! ## 凍結セット
//! 実 emo2 `pasta.dll` からの実採取では、正典 GET は **`OnFirstBoot` 単独**である。kanade の
//! フォールスルー（`schedule/boot.rs`）は `OnFirstBoot` が Value を返すと続く `OnBoot` GET を
//! スキップするため。応答は正準 SHIORI/3.0 形式（status line＋`Charset: UTF-8`＋`Value:`＋空行終端）。
//!
//! ## PROVISIONAL マーカー
//! 実採取前の暫定応答はカスタムヘッダ `X-Areka-Snapshot: PROVISIONAL` を帯同していた
//! （`shiori-host32-host` の `parse_response` は未知ヘッダを無視するのでワイヤ上は無害）。
//! 全スナップショットが [`PROVISIONAL_MARKER`] を**帯同しない**ことが実採取完了の機械的証左となる。
#![allow(dead_code)]

use std::sync::OnceLock;

/// 埋め込み済みの生スナップショット（`(イベント ID, 応答全文)`）。行末は未正規化。
const RAW_SNAPSHOTS: &[(&str, &str)] =
    &[("OnFirstBoot", include_str!("../snapshots/OnFirstBoot.txt"))];

/// 旧暫定応答に帯同していた PROVISIONAL マーカーヘッダ（値レベルの単一権威）。
///
/// 全スナップショットが**帯同しない**ことを固定するテスト
/// （`no_snapshot_carries_provisional_marker_after_real_capture`）の値源。
pub const PROVISIONAL_MARKER: &str = "X-Areka-Snapshot: PROVISIONAL";

/// 全ての行末表現（lone `\n`・`\r\n`）を一様な `\r\n` へ正規化する純関数。
///
/// **冪等**（`normalize_crlf(&normalize_crlf(x)) == normalize_crlf(x)`）。
pub(crate) fn normalize_crlf(s: &str) -> String {
    s.replace('\r', "").replace('\n', "\r\n")
}

/// 凍結スナップショット表を CRLF 正規化済み・ID 昇順の決定論固定順で返す。
///
/// 呼ぶたびに同一参照を返す（`static` [`OnceLock`] が保持する）。
pub fn snapshots() -> &'static [(&'static str, &'static str)] {
    static TABLE: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: Vec<(&'static str, &'static str)> = RAW_SNAPSHOTS
            .iter()
            .map(|(id, raw)| {
                // leak は初期化 1 回きり・表は有限小集合なので許容する。
                let normalized: &'static str = Box::leak(normalize_crlf(raw).into_boxed_str());
                (*id, normalized)
            })
            .collect();
        table.sort_by(|a, b| a.0.cmp(b.0));
        table
    })
}

/// ID に対応する CRLF 正規化済みの凍結応答を返す（未収載は `None`）。
///
/// `&'static str` なのは `select_response` の `lookup` と `Selection::Snapshot` がそれを要求するため。
pub fn snapshot_for(id: &str) -> Option<&'static str> {
    snapshots()
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, response)| *response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalize_crlf の正規化＋冪等性檻 ----

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
        assert_eq!(normalize_crlf("a\r\nb\nc\r\n"), "a\r\nb\r\nc\r\n");
    }

    #[test]
    fn normalize_is_idempotent() {
        for raw in [
            "a\nb",
            "a\r\nb",
            "a\r\nb\nc\r\n",
            "",
            "\n",
            "\r\n",
            "no endings",
        ] {
            let once = normalize_crlf(raw);
            let twice = normalize_crlf(&once);
            assert_eq!(
                twice, once,
                "normalize_crlf は冪等であること（input={raw:?}）"
            );
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

    // ---- snapshot_for / snapshots の決定論檻 ----

    #[test]
    fn snapshot_for_onfirstboot_is_present_and_crlf_normalized() {
        let resp = snapshot_for("OnFirstBoot").expect("OnFirstBoot は収載されていること");
        assert_no_lone_lf(resp);
        assert!(resp.contains("\r\n"), "整形済み応答は CRLF 行末を持つこと");
    }

    #[test]
    fn snapshot_for_onfirstboot_is_200_with_value_line() {
        let resp = snapshot_for("OnFirstBoot").unwrap();
        assert!(
            resp.contains("200"),
            "OnFirstBoot は 200 応答であること: {resp:?}"
        );
        assert!(
            resp.contains("Value:"),
            "OnFirstBoot は Value 行を持つこと: {resp:?}"
        );
    }

    #[test]
    fn snapshot_for_onboot_is_none_after_real_capture() {
        // OnBoot は正典 GET でない（kanade のフォールスルーが OnFirstBoot の Value 返却で
        // OnBoot GET をスキップする）。
        assert_eq!(
            snapshot_for("OnBoot"),
            None,
            "OnBoot は実採取後に非収載（凍結セットは {{OnFirstBoot}}）"
        );
    }

    #[test]
    fn snapshot_for_unknown_id_is_none() {
        assert_eq!(snapshot_for("SomethingUnknown"), None);
        assert_eq!(
            snapshot_for("OnInitialize"),
            None,
            "NOTIFY イベントは非収載（narrowing）"
        );
        assert_eq!(snapshot_for(""), None);
    }

    #[test]
    fn snapshot_for_returns_same_reference_deterministically() {
        let a = snapshot_for("OnFirstBoot").unwrap();
        let b = snapshot_for("OnFirstBoot").unwrap();
        assert!(std::ptr::eq(a, b), "同一 ID は同一参照を返すこと（決定論）");
    }

    #[test]
    fn no_snapshot_carries_provisional_marker_after_real_capture() {
        // 実採取完了の機械的証左: 暫定マーカーが 1 件でも残っていれば差し替え漏れ。
        for (id, resp) in snapshots() {
            assert!(
                !resp.contains(PROVISIONAL_MARKER),
                "スナップショット {id} が PROVISIONAL マーカーを帯同している（実採取差し替え漏れ）: {resp:?}"
            );
        }
    }

    #[test]
    fn snapshots_ordering_is_deterministic_and_sorted() {
        let first = snapshots();
        let second = snapshots();
        assert!(
            std::ptr::eq(first, second),
            "snapshots() は同一参照を返すこと"
        );
        let ids: Vec<&str> = first.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(
            ids, sorted,
            "snapshots() は ID 昇順の決定論固定順であること"
        );
    }

    #[test]
    fn snapshots_covers_canonical_get_ids() {
        let ids: Vec<&str> = snapshots().iter().map(|(id, _)| *id).collect();
        assert!(
            ids.contains(&"OnFirstBoot"),
            "OnFirstBoot（挨拶・GET・200＋Value）を収載"
        );
        assert!(
            !ids.contains(&"OnBoot"),
            "OnBoot は実採取後に非収載（フォールスルーで GET スキップ）"
        );
    }

    #[test]
    fn snapshot_for_matches_table_entries() {
        for (id, resp) in snapshots() {
            assert_eq!(snapshot_for(id), Some(*resp), "{id} の 2 面が一致すること");
        }
    }
}
