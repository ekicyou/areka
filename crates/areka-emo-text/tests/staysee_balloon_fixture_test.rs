//! # staysee_balloon_fixture_test — 既定バルーン `StayseeBalloon` を検体に値を固定する
//!
//! `StayseeBalloon` は areka の既定バルーン（第三者がバルーンを同梱しないゴーストを入れたときに使われる資産）として
//! 保管した上流無改変のファイル群である。保管したファイルが欠けても増えても、定義の読み取りが後退しても赤にするテストが無ければ、
//! 「既定バルーンが表示できる」保証は毎回の目視に頼ることになる。
//!
//! 注記: 本ファイル群で「既定バルーン」とあるのは 2026-10 までの役名である。今の `StayseeBalloon` は配布物に同梱する選べるバルーンで、
//! 既定は `areka-balloon` に替わった。固定する中身は変わらない。
//!
//! 検体は窓口 `sample-ghost-kit` から `StayseeBalloon` の名前で引く。在処の綴りは本ファイルにもテーマ別ファイルにも持たず、
//! 取り方を付け替えるのは `test_support.rs` の `staysee_root()` の本体だけである。
//!
//! 本テストは公開 API（`areka_parsers::{charset, kv, balloon}`・`areka_emo_present::balloon`・`areka_emo_atlas`・
//! `areka_emo_text::writing`）を外から読むだけである。検体側に不足があってもバルーンを改変して期待値に合わせることはしない。
//!
//! 実 GPU と実窓は使わず、同一入力に対して常に同一の結果を返す。使う外の仕組みはテーマによって違う。
//!
//! - `assets`・`definition`・`faces` はファイル読み込みと純粋層の解決だけ。
//! - `bake` は画像復号（WIC）を使う。COM を MTA で張る。
//! - `region`・`wrapping`・`script` は DirectWrite の factory を使う（文字の送り幅と行送りを実測するため）。
//!   既定書体が入っていない環境では `region` の寸法の検査が先に赤くなる。
//!
//! 観測の本体はテーマ別ファイルへ分けてあり、本ファイルは接続宣言だけを持つ入口である。
//!
//! | テーマ | ファイル | 観測 |
//! |---|---|---|
//! | `test_support` | `staysee_balloon_fixture/test_support.rs` | テーマ間で共有する小ヘルパと保管フォルダの期待値 |
//! | `assets` | `staysee_balloon_fixture/assets.rs` | 保管フォルダの名前集合と枠画像の原寸 |
//! | `definition` | `staysee_balloon_fixture/definition.rs` | 定義ファイルの復号と読み取り |
//! | `faces` | `staysee_balloon_fixture/faces.rs` | 面の系列解決とその間の記録 |
//! | `bake` | `staysee_balloon_fixture/bake.rs` | 焼き込みと半透明の保持 |
//! | `region` | `staysee_balloon_fixture/region.rs` | 文字の領域解決と既定書体の受け入れ口 |
//! | `wrapping` | `staysee_balloon_fixture/wrapping.rs` | 折返しと描画範囲への内包 |
//! | `script` | `staysee_balloon_fixture/script.rs` | 選択肢と遅延座標指定の位置 |
//!
//! ### 置き場がテーマ別ディレクトリである理由
//!
//! `structure.md` の分割規約は `<stem>_<テーマ名>.rs` を同じディレクトリに置く形を定めるが、`tests/` 直下の `*.rs` は cargo が
//! 独立した統合テストのターゲットとして自動で拾う（実測: 親に `#[path]` で繋いだうえで `tests/staysee_balloon_fixture_test_probe.rs` を
//! 置くと、同じテストが親のターゲットと自分自身のターゲットの 2 か所で走った）。`Cargo.toml` は変えられないので、自動収集の対象外である
//! テーマ別ディレクトリへ置き、`structure.md` の統合テストの慣行（`tests/{ドメイン}.rs` を束ね役の入口、実体は `tests/{ドメイン}/` 配下・
//! ドメイン名と重複する接頭辞は除去）に合わせた。同 crate の `decoration_readback_test.rs` ＋ `decoration_readback/` が同型の先例である。

#[path = "support/mod.rs"]
mod support;

#[path = "staysee_balloon_fixture/test_support.rs"]
mod test_support;

#[path = "staysee_balloon_fixture/assets.rs"]
mod assets;

#[path = "staysee_balloon_fixture/definition.rs"]
mod definition;

#[path = "staysee_balloon_fixture/faces.rs"]
mod faces;

#[path = "staysee_balloon_fixture/bake.rs"]
mod bake;

#[path = "staysee_balloon_fixture/region.rs"]
mod region;

#[path = "staysee_balloon_fixture/wrapping.rs"]
mod wrapping;

#[path = "staysee_balloon_fixture/script.rs"]
mod script;
