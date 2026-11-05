use super::*;

use std::path::PathBuf;
use std::sync::Arc;

use areka_emo_atlas::MemoryDecoder;
use temp_path_kit::TempPath;

use crate::balloon_marker::MarkerImage;

// ── テスト用一時ディレクトリ（`balloon`・`balloon_marker`・`balloon_definitions` のテストで 1 つ）──
// 置き場・一意化・後始末は共有の窓口 [`TempPath`]（ワークツリーの `target\test-roots` の下）に任せる。

/// 破棄時に自身を再帰削除する一時ディレクトリ。
pub(crate) struct TempDir(TempPath);

impl TempDir {
    pub(crate) fn new() -> Self {
        TempDir(TempPath::new("emo-present-balloon"))
    }

    pub(crate) fn path(&self) -> &Path {
        self.0.path()
    }

    /// 空のプレースホルダファイルを作る（MemoryDecoder 経路ゆえ中身は不問・列挙対象のため名前のみ要）。
    pub(crate) fn touch(&self, name: &str) {
        std::fs::File::create(self.0.child(name)).expect("プレースホルダ作成");
    }

    /// テキストファイルを UTF-8 で書く（記述ファイル 2 層の合成 fixture 用）。
    pub(crate) fn write(&self, name: &str, content: &str) {
        std::fs::write(self.0.child(name), content).expect("記述ファイル作成");
    }

    /// 名前だけのファイルを置き、偽の読み手に `w`×`h` の絵を登録する。
    pub(crate) fn image(&self, dec: &mut MemoryDecoder, name: &str, w: u32, h: u32) {
        self.touch(name);
        let bgra = vec![0x80; (w * h * 4) as usize];
        dec.insert(self.0.child(name), w, h, w * 4, bgra, true);
    }
}

/// 自前の α を使う読みの設定（印と定義のテストで共通）。
pub(crate) const ALPHA: AlphaParams = AlphaParams {
    use_self_alpha: UseSelfAlpha::On,
};

/// 印の絵の寸法（無ければ `None`）。
pub(crate) fn size(img: &Option<Arc<MarkerImage>>) -> Option<(u32, u32)> {
    img.as_ref().map(|i| (i.width, i.height))
}

// ── ログの捕捉 ──────────────────────────────────────────────────────────────────────
//
// 捕捉は共有 crate の [`log_capture_kit::capture`] を各檻がそのまま呼ぶ（硬化の機序は同 crate の
// crate doc にあり、ここには写さない）。

pub(crate) use log_capture_kit::CapturedEvent;

/// フィールド値を**引用符抜き**で引く拡張（移行前の `CapturedEvent::field` と同一規則）。
///
/// `%`（Display）記録は素の文字列・`?`（Debug）記録は引用符付きになり得るため、両表記に
/// 依存しない比較にする。値は必ず Debug 表現（[`CapturedEvent::field`]）から取る——
/// [`CapturedEvent::field_str`] は `record_str` 経路の生値だけを返すので、本ファイルの檻が
/// 見る `%expr` のシジル形では `None` になり、判定が黙って空振りする。
///
/// メソッド名を `field` にしないのは、[`CapturedEvent`] の固有メソッド `field`（引用符付きの
/// Debug 表現）が拡張トレイトより優先され、規則が黙って入れ替わるためである。
pub(crate) trait FieldUnquoted {
    /// フィールド値を引用符抜きで引く（欠落は `None`）。
    fn field_unquoted(&self, name: &str) -> Option<&str>;
}

impl FieldUnquoted for CapturedEvent {
    fn field_unquoted(&self, name: &str) -> Option<&str> {
        self.field(name).map(|v| v.trim_matches('"'))
    }
}

// ── emo2 のバルーンのフォルダ ─────────────────────────────────────────────────────────

/// emo2 が同時にインストールするバルーンのフォルダを窓口から得る（自分でパスを継ぎ足さない）。
///
/// 本 fixture は実資産である——`descript.txt`（基層）・`balloons0s.txt`（本体側の面別上書き層）・
/// `balloonk0s.txt`（相方側の面別上書き層）に加え、面画像 `balloons0.png` / `balloonk0.png` を
/// 併せ持つため、scope 0 / 1 が**別の面を採用し別の上書き層へ辿り着く**ことを実データで固定できる。
pub(super) fn emo2_balloon_root() -> PathBuf {
    crate::shell_target::test_support::EMO2
        .balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}
