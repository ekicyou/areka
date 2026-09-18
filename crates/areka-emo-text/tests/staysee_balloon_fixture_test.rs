//! # staysee_balloon_fixture_test — 既定バルーン `StayseeBalloon` を検体に値を固定する
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**3.1**／**3.2**／**3.3**／
//! **3.4**／**3.8**／**3.11**／**5.4**／**7.1**・設計 **C2**）。
//!
//! ## このテストが塞ぐ穴
//!
//! `vendors/sample_ghost/StayseeBalloon/` は areka の**既定バルーン**（第三者がバルーンを
//! 同梱しないゴーストを入れたときに使われる資産）として保管した上流無改変のファイル群である。
//! 保管したファイルが欠けても増えても、あるいは定義の読み取りが後退しても、それを赤にする
//! テストが無ければ「既定バルーンが表示できる」保証は毎回の目視に頼ることになる。
//!
//! ## 検体パスは 1 定数だけが持つ（要件 3.2）
//!
//! 下流の `areka-P0-nar-install` が検体を `.nar` へ畳み、共有ヘルパへ寄せるとき、
//! 付け替えるのは [`STAYSEE_BALLOON_DIR`] の 1 行だけで済む形にしてある。パス文字列を
//! 本ファイルおよびテーマ別ファイルの他の場所へ散らしてはならない。
//!
//! ## 本番コードは 1 行も変えない（要件 8.1）
//!
//! 本テストは公開 API（`areka_parsers::{charset, kv, balloon}`・`areka_emo_present::balloon`・
//! `areka_emo_atlas`・`areka_emo_text::writing`）を**外から読むだけ**である。検体側に不足が
//! あってもバルーンを改変して期待値に合わせることはしない（要件 3.9）。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・WIC による画像復号のみ。実 GPU・実窓・DirectWrite を
//! 要さず、同一入力に対して常に同一の結果を返す。
//!
//! ## テーマ別ファイルへの分割（要件 3.11・設計 C2 の縮退先）
//!
//! 観測が 1 ファイル 1,000 行の目安を超える見込みになったため、`structure.md`
//! 「Test Naming Conventions」の分割規約に従い、観測の本体をテーマ別ファイルへ移した。
//! 本ファイルは**検体パス定数と接続宣言だけ**を持つ入口である。
//!
//! | テーマ | ファイル | 観測 |
//! |---|---|---|
//! | `test_support` | `staysee_balloon_fixture/test_support.rs` | テーマ間で共有する小ヘルパと保管フォルダの期待値 |
//! | `assets` | `staysee_balloon_fixture/assets.rs` | 保管フォルダの名前集合と枠画像の原寸 |
//! | `definition` | `staysee_balloon_fixture/definition.rs` | 定義ファイルの復号と読み取り |
//! | `faces` | `staysee_balloon_fixture/faces.rs` | 面の系列解決とその間の記録 |
//! | `bake` | `staysee_balloon_fixture/bake.rs` | 焼き込みと半透明の保持 |
//! | `region` | `staysee_balloon_fixture/region.rs` | 文字の領域解決と既定書体の受け入れ口 |
//!
//! ### 置き場がテーマ別**ディレクトリ**である理由
//!
//! `structure.md` の分割規約は `<stem>_<テーマ名>.rs` を同じディレクトリに置く形を定めるが、
//! `tests/` 直下の `*.rs` は cargo が**独立した統合テストのターゲットとして自動で拾う**
//! （実測: 親に `#[path]` で繋いだうえで `tests/staysee_balloon_fixture_test_probe.rs` を
//! 置くと、同じテストが親のターゲットと自分自身のターゲットの 2 か所で走った）。
//! `Cargo.toml` は 1 つも変えられない（要件 8.2）ので、自動収集の対象外である
//! テーマ別ディレクトリへ置き、`structure.md` の統合テストの慣行
//! （`tests/{ドメイン}.rs` を束ね役の入口、実体は `tests/{ドメイン}/` 配下・ドメイン名と
//! 重複する接頭辞は除去）に合わせた。同 crate の `decoration_readback_test.rs` ＋
//! `decoration_readback/` が同型の先例である。

#[cfg(test)]
#[path = "staysee_balloon_fixture/test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "staysee_balloon_fixture/assets.rs"]
mod assets;

#[cfg(test)]
#[path = "staysee_balloon_fixture/definition.rs"]
mod definition;

#[cfg(test)]
#[path = "staysee_balloon_fixture/faces.rs"]
mod faces;

#[cfg(test)]
#[path = "staysee_balloon_fixture/bake.rs"]
mod bake;

#[cfg(test)]
#[path = "staysee_balloon_fixture/region.rs"]
mod region;

// ── 検体の所在（要件 3.2: パスを持つのはこの 1 定数だけ）───────────────────

/// 保管フォルダの所在（`crates/areka-emo-text` から見た相対パス）。
///
/// **本テスト全体で検体パスを綴るのはここだけ**である。`nar-install` が共有ヘルパを
/// 導入したら、この 1 行をヘルパ呼び出しへ付け替える（要件 3.2・設計 C2）。
/// 実体化は [`test_support::staysee_root`] が行う。
const STAYSEE_BALLOON_DIR: &str = "../../vendors/sample_ghost/StayseeBalloon";
