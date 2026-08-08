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
// ここでは tracing の既定 subscriber を差し替えてイベントを捕捉し、発火条件を直に押さえる。
//
// ログ捕捉ハーネスは同 crate `presenter.rs` の tests に同型のものが在るが、あちらは
// test-local な private 型ゆえ本モジュールから参照できない。新規 dev-dependency を
// 足さない方針ゆえ、`tracing` 本体のみで最小構成を再現する。

/// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
#[derive(Debug, Clone)]
pub(super) struct CapturedEvent {
    pub(super) level: tracing::Level,
    pub(super) fields: std::collections::HashMap<String, String>,
}

impl CapturedEvent {
    /// フィールド値を引用符抜きで引く（`%`（Display）記録は素の文字列・`?`（Debug）記録は
    /// 引用符付きになり得るため、両表記に依存しない比較にする）。
    pub(super) fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(|v| v.trim_matches('"'))
    }
}

/// 全フィールドを Debug 表現で拾う visitor。
///
/// [`tracing::field::Visit`] の `record_u64`/`record_str` 等はすべて既定実装が `record_debug`
/// へ転送するため、`record_debug` 1 本で型を問わず全フィールドを捕捉できる。
struct FieldGrab(std::collections::HashMap<String, String>);

impl tracing::field::Visit for FieldGrab {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

/// イベントを溜めるだけの最小 subscriber（span は使わないので new_span は固定 id を返す）。
#[derive(Clone, Default)]
struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut grab = FieldGrab(std::collections::HashMap::new());
        event.record(&mut grab);
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .push(CapturedEvent {
                level: *event.metadata().level(),
                fields: grab.0,
            });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// interest キャッシュへ `never` を焼かせないための常駐 dispatcher（同 crate `scale.rs` の
/// 確立した対策を本ファイル境界内へ最小複製したもの）。
///
/// `register_callsite` が常に [`tracing::subscriber::Interest::sometimes`] を返すことだけが
/// 仕事で、`enabled()` は偽・`event()` は no-op（他テストの観測へ副作用を与えない）。
struct InterestProbe;

impl tracing::Subscriber for InterestProbe {
    fn register_callsite(
        &self,
        _meta: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        // 既定実装は `enabled()` が偽なら `never` を返してしまう。ここを `sometimes` に
        // 固定することが本 probe の唯一の存在理由。
        tracing::subscriber::Interest::sometimes()
    }
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        false
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, _event: &tracing::Event<'_>) {}
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// probe dispatcher を **2 個**プロセス寿命で常駐させる（冪等）。
///
/// 2 個必要なのは tracing-core の `has_just_one = (dispatchers.len() <= 1)` ゆえ——1 個では
/// 登録直後も真のままとなり、interest 計算が「登録したスレッドの既定 dispatcher」
/// （購読者を持たないテストスレッドでは `NoSubscriber`＝`never`）を参照する毒の経路が
/// 生き残る。2 個目の登録で確定的に偽へ落とし、interest を「生存する登録済み dispatcher
/// 全体の `and`」で決めさせる。probe は常に `sometimes` を返すゆえ合成結果は決して
/// `never` にならない。
fn ensure_interest_probes() {
    static PROBES: std::sync::OnceLock<(tracing::Dispatch, tracing::Dispatch)> =
        std::sync::OnceLock::new();
    PROBES.get_or_init(|| {
        // `Dispatch::new` が callsite の登録＋全走査再計算を行う。
        (
            tracing::Dispatch::new(InterestProbe),
            tracing::Dispatch::new(InterestProbe),
        )
    });
}

/// `f` の実行中に出たイベントを捕捉して `(戻り値, イベント列)` を返す。
///
/// `with_default` が差し替えるのはスレッドローカルの既定 dispatcher だが、callsite の
/// interest キャッシュは**プロセス大域**であり「その callsite をプロセス内で最初に踏んだ
/// スレッドが勝つ」。本ファイルのログ callsite は捕捉しない他テストと共有されるため、
/// probe 常駐（[`ensure_interest_probes`]）＋窓の内側での `rebuild_interest_cache` の
/// 二段で毒化を潰す。詳細な機序は同 crate `scale.rs` の `mod tests` 冒頭コメントに在る。
pub(super) fn capture_events<T>(f: impl FnOnce() -> T) -> (T, Vec<CapturedEvent>) {
    ensure_interest_probes();

    let cap = CaptureSubscriber::default();
    let out = tracing::subscriber::with_default(cap.clone(), || {
        // probe 常駐前（プロセス起動〜初回捕捉）に焼かれた `never` の掃き残しを潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let events = cap.0.lock().expect("捕捉バッファの毒化なし").clone();
    (out, events)
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
