use super::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

// ── テスト用一時ディレクトリ（新規 dev-dep を避け std のみで構成）─────────────
// `std::env::temp_dir()` 配下へプロセス id ＋単調カウンタで一意なサブディレクトリを作り、
// Drop で後始末する（tempfile 相当の最小実装）。

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Drop 時に自身を再帰削除する一時ディレクトリ。
pub(super) struct TempDir {
    pub(super) path: PathBuf,
}

impl TempDir {
    pub(super) fn new() -> Self {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "areka-emo-present-balloon-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&path).expect("一時ディレクトリ作成");
        TempDir { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// 空のプレースホルダファイルを作る（MemoryDecoder 経路ゆえ中身は不問・列挙対象のため名前のみ要）。
    pub(super) fn touch(&self, name: &str) {
        std::fs::File::create(self.path.join(name)).expect("プレースホルダ作成");
    }

    /// テキストファイルを UTF-8 で書く（記述ファイル 2 層の合成 fixture 用）。
    pub(super) fn write(&self, name: &str, content: &str) {
        std::fs::write(self.path.join(name), content).expect("記述ファイル作成");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

// ── 檻 7: R6.2 warn の**発火条件**（`scope >= 1` ∧ `tier == Default`・R6.2/R7.1）───────
//
// tier の割当自体は檻 1〜3 が固定しているが、warn を出すか出さないかの判断分岐そのものは
// 実ログを観測しなければ固定できない（両述語を同時に反転しても tier 檻は全緑のまま通る）。
// ここでは共有機構の捕捉窓でイベントを捕らえ、発火条件を直に押さえる。
//
// # 硬化機構は 1 箇所にしかない（「スレッドローカルゆえ安全」は誤り）
//
// 捕捉窓そのものは共有 crate `log-capture-kit` へ委譲する（spec:
// areka-P0-test-cage-determinism・要件 1.5／2.2）。以前ここに在った常駐 probe と最小
// subscriber は同 crate `scale.rs` の同型を本ファイル境界内へ写し取ったもので、写し損ねた
// 側だけが静かに嘘をつく形だったため撤去した。
//
// 「`with_default` はスレッドローカルだから並行実行でも干渉しない」は**誤り**である。
// 差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める
// callsite の interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点をプロセス内で
// 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッドの既定は `NoSubscriber` で
// 判定は「不要」なので、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を
// 差していても以後そのイベントは早期 return で捨てられる——つまり混入ではなく**取りこぼし**が
// 起きる。結果、不在の主張は捕捉 0 件のまま静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で
// 確率的に赤になる（偽陽性）。
//
// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での interest 再計算 ⑶ 窓の内側で発火する
// 対照イベント（番兵）による空振り検出、の 3 点でこれを塞ぐ。番兵は返却前に取り除かれるので
// 呼出側の件数・主張は変わらない。捕捉されるのは呼出スレッドで同期的に発火したイベントだけで
// ある点は移行前と同じ。機序の逐条解説（`tracing-core` の実コード引用つき）は
// `log_capture_kit` の crate doc と同 crate の `src/probe.rs` にある。

pub(super) use log_capture_kit::CapturedEvent;

/// フィールド値を**引用符抜き**で引く拡張（移行前の `CapturedEvent::field` と同一規則）。
///
/// `%`（Display）記録は素の文字列・`?`（Debug）記録は引用符付きになり得るため、両表記に
/// 依存しない比較にする。値は必ず Debug 表現（[`CapturedEvent::field`]）から取る——
/// [`CapturedEvent::field_str`] は `record_str` 経路の生値だけを返すので、本ファイルの檻が
/// 見る `%expr` のシジル形では `None` になり、判定が黙って空振りする。
///
/// メソッド名を `field` にしないのは、[`CapturedEvent`] の固有メソッド `field`（引用符付きの
/// Debug 表現）が拡張トレイトより優先され、規則が黙って入れ替わるためである。
pub(super) trait FieldUnquoted {
    /// フィールド値を引用符抜きで引く（欠落は `None`）。
    fn field_unquoted(&self, name: &str) -> Option<&str>;
}

impl FieldUnquoted for CapturedEvent {
    fn field_unquoted(&self, name: &str) -> Option<&str> {
        self.field(name).map(|v| v.trim_matches('"'))
    }
}

/// `f` の実行中に出たイベントを捕捉して `(戻り値, イベント列)` を返す。
///
/// 捕捉と硬化は硬化機構の唯一の定義元 [`log_capture_kit::capture`] が行う。捕捉が働いて
/// いなければ空の結果を静かに返さず panic する。
pub(super) fn capture_events<T>(f: impl FnOnce() -> T) -> (T, Vec<CapturedEvent>) {
    log_capture_kit::capture(f)
}

// ── 檻 8: scope 別バルーン定義の 2 層マージ（`load_scope_balloon_model`・R2.1/2.2/2.3/2.4/2.5・
//          R6.3/6.4・D8）─────────────────────────────────────────────────────────────

/// emo2 バルーン fixture（`emo2-kakukaku`）を `CARGO_MANIFEST_DIR`（`crates/areka-emo-present`）
/// 相対で解決する（areka 側 placement/assets テストと同一アンカー規約）。
///
/// 本 fixture は実資産である——`descript.txt`（基層）・`balloons0s.txt`（本体側の面別上書き層）・
/// `balloonk0s.txt`（相方側の面別上書き層）に加え、面画像 `balloons0.png` / `balloonk0.png` を
/// 併せ持つため、scope 0 / 1 が**別の面を採用し別の上書き層へ辿り着く**ことを実データで固定できる。
pub(super) fn emo2_balloon_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku")
}
