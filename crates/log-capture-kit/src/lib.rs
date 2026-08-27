//! ログ捕捉テストの硬化機構を、ワークスペースで **1 箇所**だけ定義する crate。
//!
//! この doc は**利用手順**である。ここだけを読めば、別の crate から「このログが出る」
//! 「このログは出ない」を主張するテストが書ける。実装（`src/` の各 module）を開く必要は無い。
//!
//! 掲載している Rust の例はすべて **doctest** として `cargo test -p log-capture-kit` で
//! コンパイル・実行される。しかも doctest は本 crate を外から `use` する**別 crate**として
//! 組まれるので、そのまま消費側で成立する形になっている（手順書が黙って古びない）。
//!
//! # 引き方
//!
//! 消費 crate の `Cargo.toml` に 1 行加える。**`[dev-dependencies]` からのみ**引く。
//!
//! ```toml
//! [dev-dependencies]
//! log-capture-kit = { path = "../log-capture-kit" }
//! ```
//!
//! `RUST_LOG` 相当の directive で**実濾過**した出力が要る場合だけ feature `env-filter` を
//! 有効にする。有効にしてよいのは `wintf` のみで、他の crate は素のまま引く。
//!
//! ```toml
//! [dev-dependencies]
//! log-capture-kit = { path = "../log-capture-kit", features = ["env-filter"] }
//! ```
//!
//! 併せて、テストから `tracing::info!` などを発火するので **`tracing` が消費 crate 側から
//! 引けること**が要る（本 crate の依存は伝播しない）。消費 10 crate はいずれも既に
//! `[dependencies]` に `tracing` を持っているので追加の作業は無いが、新しい crate から
//! 引くときは 2 行になる。
//!
//! bin crate の in-crate `#[cfg(test)]` テスト・`areka-*` 各 crate・統合テスト（`tests/`）の
//! いずれからも同じ形で `use` できる。
//!
//! # 存在を主張するテストの書き方
//!
//! [`capture`] に処理を渡すと、`f` の戻り値と、`f` の実行中に**呼出スレッド**で同期的に
//! 発火したイベント列が返る。戻り値は一切加工されずそのまま返るので、被検査コードの結果と
//! ログの両方を 1 回の実行で見られる。
//!
//! 値の取り出しは 3 本。[`CapturedEvent::message`] が本文、[`CapturedEvent::field`] が
//! フィールドの **Debug 表現**（文字列なら引用符つき）、[`CapturedEvent::field_str`] が
//! **生値**（引用符なし）である。
//!
//! ```rust
//! use log_capture_kit::capture;
//!
//! let (out, events) = capture(|| {
//!     tracing::warn!(target: "areka_demo::presence", code = 7, reason = "低速", "更新を落とした");
//!     "戻り値はそのまま返る"
//! });
//!
//! assert_eq!(out, "戻り値はそのまま返る");
//!
//! let hit = events
//!     .iter()
//!     .find(|e| e.target == "areka_demo::presence")
//!     .expect("警告が出ているはず");
//! assert_eq!(hit.level, tracing::Level::WARN);
//! assert_eq!(hit.message(), "更新を落とした");
//! assert_eq!(hit.field("code"), Some("7"));          // 数値は Debug 表現
//! assert_eq!(hit.field_str("reason"), Some("低速")); // 文字列は生値で引ける
//! assert_eq!(hit.field("reason"), Some("\"低速\"")); // 同じ値の Debug 表現
//! ```
//!
//! TRACE を含む**全レベル・全宛先**が捕捉対象で、水準による取りこぼしは無い。
//!
//! # 不在を主張するテストの書き方
//!
//! 「このログは出ない」という主張は、捕捉そのものが働いていなければ**捕捉 0 件のまま
//! 静かに通る**（偽陰性）。[`capture`] はこれを既定で塞ぐ——窓の内側で対照イベント
//! （番兵）を 1 件自分で発火し、それを捕捉できなければ panic する。対照は返却前に
//! 取り除かれるので、呼出側の件数・主張は番兵の分だけ変わらない。**対照を呼出側が
//! 書き足す必要は無い。**
//!
//! ただし番兵が示すのは「**窓**が生きていた」ことであって、「**主張の対象になっている
//! 発行点**が生きていた」ことではない。不在の主張を書くときは、同じ窓・同じスレッドで
//! 「出る側」を 1 件確かめる対照を隣に置く。これで、被検査コードごと素通りしていた場合も
//! 赤になる。
//!
//! ```rust
//! use log_capture_kit::capture;
//!
//! let ((), events) = capture(|| {
//!     // 被検査コード（正常系なので警告は出ないはず）
//!     tracing::info!(target: "areka_demo::absence", phase = "done", "正常終了");
//! });
//!
//! // 対照: この窓でこの経路が本当に動いていた（空振りの緑を塞ぐ）
//! assert_eq!(
//!     events.iter().filter(|e| e.target == "areka_demo::absence").count(),
//!     1
//! );
//! // 主張: 警告は 1 件も出ない
//! assert!(events.iter().all(|e| e.level != tracing::Level::WARN));
//! ```
//!
//! # 統合テスト（`tests/`）から引く
//!
//! 統合テストは消費 crate の外側から `use` する別 crate だが、引き方は in-crate テストと
//! 同一である（上の例はまさにその形で走っている）。
//!
//! ```rust
//! // crates/areka-foo/tests/foo_test.rs
//! use log_capture_kit::{LineFormat, capture_lines};
//!
//! let (_, lines) = capture_lines(LineFormat::LevelTargetFields, || {
//!     tracing::info!(target: "areka_foo", step = "start", "起動した");
//! });
//!
//! assert!(lines.iter().any(|l| l == r#"level=INFO target=areka_foo message=起動した step="start""#));
//! ```
//!
//! # 全スレッド捕捉の使いどころと両立条件
//!
//! [`capture`] は**呼出スレッド局所**で、窓の内側でも他スレッドが発火したイベントは入らない。
//! テストの大半はこれで足りる。別スレッド（アクタースレッド等）で発火するログを照合する
//! 必要があるときだけ [`install_global_capture_all`] を使う。
//!
//! こちらはプロセス全体の既定を**一度だけ**据え付ける窓口で、取り消せない。戻り値は
//! 窓の結果ではなく**据え付けた蓄積先そのもの**（以後ずっと伸び続ける共有バッファ）である。
//!
//! ```rust
//! use log_capture_kit::install_global_capture_all;
//!
//! let sink = install_global_capture_all();
//!
//! std::thread::spawn(|| tracing::warn!(target: "areka_demo::worker", "別スレッドの警告"))
//!     .join()
//!     .expect("作業スレッドは panic しない");
//!
//! let events = sink.lock().expect("蓄積先は毒化していない");
//! assert!(events.iter().any(|e| e.target == "areka_demo::worker"));
//! ```
//!
//! **両立条件**は 2 つ。
//!
//! 1. 据え付けた後は、そのテストバイナリ内の `tracing::enabled!` が**全スレッドで真**になる
//!    （フィルタを持たない捕捉先がプロセスの既定になるため）。ログ有効判定が偽であることを
//!    前提にするテストと同じバイナリには置けない。
//! 2. 全体設定を置いてよいのはこの窓口だけである。他所が `set_global_default`／`init` を
//!    先に行っていると、この窓口は黙って縮退せず**明示的に panic する**。
//!
//! この窓口を使うファイルは番人テストの例外表に列挙する（暗黙に増えない）。
//!
//! # そのほかの窓口
//!
//! - [`capture_lines`] — 捕捉結果をその場で 1 行 1 イベントへ整形する。形は [`LineFormat`]
//!   の 2 種（宛先を載せる／載せない）で、既存の自前整形の出力を 1 バイトも違わずに再現する。
//! - [`format_line`] — 既に手元にある [`CapturedEvent`] を同じ規則で 1 行にする。
//! - [`count_levels`] — レベル別の件数（[`LevelCounts`]）だけを見る。警告 0 件の主張などに使う。
//! - [`CapturedEvent::field_names_sorted`]／[`CapturedEvent::fields_map`] — フィールド集合を
//!   ちょうどで固定する判定に使う（いずれも Debug 表現・名前の昇順）。
//! - [`ensure_interest_probes`] — 常駐の仕掛けを窓を開かずに確立する（冪等）。[`capture`] が
//!   内部で呼ぶので、通常は消費側が直接呼ぶ必要は無い。
//! - [`interest_probes_enabled`] — 常駐の仕掛けが実際に確立されているかを返す。**測定専用の
//!   切替**（環境変数 `AREKA_LOG_CAPTURE_PROBES=off` で常駐を止める・要件 13.1）が、測定対象の
//!   プロセスまで本当に届いていたかを、切替と同じ経路で確かめるための窓口である。
//!   **捕捉テストがこれを読んで分岐してはならない**——硬化の有無で主張が変わるテストは、
//!   硬化が効いていないときに黙って別のことを確かめて緑になる。それは本 crate が
//!   根絶するために作られた当の形である。
//! - `capture_under_filter`（feature `env-filter`）— `RUST_LOG` 相当の directive を実際に
//!   適用し、濾過を通過した整形済み出力だけを 1 本の文字列で返す。
//!
//! # 注意（先に知っておくと踏まない罠）
//!
//! - [`CapturedEvent::field_str`] が返すのは `record_str` 経路で渡された値だけである。
//!   ログ側が `?expr`／`%expr`（`Debug`／`Display` 経由）でフィールドを渡していると
//!   **`None` になる**。生値で完全一致を判定する側は、[`CapturedEvent::field`] の Debug 表現
//!   からの復旧経路を必ず併せ持つこと（`field_str` だけで書くと、その経路のログが黙って
//!   落ちて主張が空振りになる）。
//! - `capture_under_filter` は番兵の行を「行が番兵の宛先文字列 `log_capture_kit::sentinel` を
//!   含むか」で見分けて取り除く。呼出側のログ本文がその字面を含むと、その行ごと戻り値から
//!   消える（現行の消費サイトに該当は無いことを実測で確認済み）。
//!
//! # 機序: なぜ「スレッドローカルだから安全」が誤りなのか
//!
//! 捕捉の説明として広く書かれている「`tracing::subscriber::with_default` はスレッドローカル
//! だから、並行実行でも他のテストと干渉しない」は**誤り**である。この誤りを手本にすると、
//! 硬化を持たない捕捉テストが増え、赤と緑の意味が壊れる。
//!
//! 正しくはこうである。
//!
//! - `with_default` が差し替えるのは**スレッドローカルの既定 dispatcher** だけである。
//!   ここは確かにスレッドごとに独立している。
//! - しかし「そのログを評価するかどうか」を決める **callsite の interest キャッシュは
//!   プロセス全体で 1 つ**であり、**その発行点をプロセス内で最初に踏んだスレッドの判定が
//!   焼き付く**。
//! - 捕捉窓を持たないスレッドの既定は `NoSubscriber` で、その判定は「不要」である。先に
//!   踏まれると `never` が大域へ焼き付き、以後その発行点のイベントは早期 return で捨てられる。
//!   自分のスレッドへ捕捉先を差していても関係なく取りこぼす。
//! - 結果、不在の主張は捕捉 0 件のまま静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で
//!   確率的に赤になる（偽陽性）。**干渉しないのは dispatcher だけで、interest は全スレッドで
//!   1 つしかない。**
//!
//! 本 crate が保証するのは次の 3 点で、[`capture`] を通す限り呼出側は何もしなくてよい。
//!
//! 1. プロセス寿命の probe を常駐させ、interest が `never` へ落ちる経路を恒久的に閉じる。
//! 2. 窓の内側で interest を再計算し、常駐より前に焼き付いた `never` を解消する。
//! 3. 窓の内側で対照イベントを発火し、捕捉が働いていなければ panic する（空の結果を静かに
//!    返さない）。
//!
//! この機序を crate ごとに書き写すと、写し損ねた側だけが静かに嘘をつく。だから定義箇所を
//! 本 crate に一本化する（根拠となる `tracing-core` の実コード引用は `src/probe.rs` にある）。
//!
//! # 依存方向の規律
//!
//! 本 crate はワークスペース内のどの crate にも依存しない leaf であり、`publish = false`。
//! 消費側は **`[dev-dependencies]` からのみ**引く。`[dependencies]`（および
//! `[build-dependencies]`）に現れた時点で規律違反であり、番人テストが赤にする。
//! この規律により、`wintf` のような下層 crate が上位の本番 crate へ依存を持つことなく
//! 同じ捕捉機構を共有できる。

mod capture;
mod event;
#[cfg(feature = "env-filter")]
mod filter;
mod global;
mod probe;

pub use capture::capture;
pub use event::{
    CapturedEvent, FieldValue, LevelCounts, LineFormat, capture_lines, count_levels, format_line,
};
#[cfg(feature = "env-filter")]
pub use filter::capture_under_filter;
pub use global::install_global_capture_all;
pub use probe::{ensure_interest_probes, interest_probes_enabled};
