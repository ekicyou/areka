//! # areka_balloon_fixture_test — 自作のバルーン `areka-balloon` の中身を固定する
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.1**〜**12.5**・設計「Testing Strategy」の
//! 「中身のテスト」）。
//!
//! ## このテストが塞ぐ穴
//!
//! `areka-balloon` は絵の道具（`areka-emo-atlas/examples/gen_default_balloon/`）が書いた
//! 書庫である。道具と書庫のずれは別の見張り（`areka-emo-atlas/tests/default_balloon_drift_test.rs`）
//! が見るが、それは「保管物が道具の今の出力か」しか言わない。本テストは「保管物が要件どおりか」を
//! 要件の数値から**手で書いた**期待値で見る（道具のコードは参照しない）。
//!
//! ## 検体は窓口から名前で引く（要件 12.1）
//!
//! 検体は窓口 `sample-ghost-kit` から `areka-balloon` の名前で引く。在処の綴りは本ファイルにも
//! テーマ別ファイルにも持たない。取り方は `test_support.rs` の `balloon_root()` の本体だけにある。
//!
//! ## 決定論
//!
//! 実 GPU と実窓は使わない。`assets` はファイル読み込みと、動く絵の見出しを聞くための
//! WIC の読み手（COM を MTA で張る）だけを使う。
//!
//! ## テーマ別ファイル
//!
//! `staysee_balloon_fixture_test.rs` と同じ形で、本ファイルは**接続宣言だけ**を持つ入口である。
//! テーマ別ファイルを `tests/` 直下でなくディレクトリに置く理由も同じ（`tests/*.rs` は cargo が
//! 別のテストのターゲットとして自動で拾う）。
//!
//! | テーマ | ファイル | 観測 |
//! |---|---|---|
//! | `test_support` | `areka_balloon_fixture/test_support.rs` | 検体の取得と期待値の表 |
//! | `assets` | `areka_balloon_fixture/assets.rs` | ファイル名の集合・絵の寸法・名前の重なり |
//! | `definition` | `areka_balloon_fixture/definition.rs` | 文のファイルの中身・読み込みの記録・描画範囲の解け方 |
//! | `bake` | `areka_balloon_fixture/bake.rs` | 面 0 の焼き込みと半透明の保持 |
//! | `online` | `areka_balloon_fixture/online.rs` | オンラインの印のコマ・待ち時間・繰り返し・灯る点の α |

#[cfg(test)]
#[path = "areka_balloon_fixture/test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "areka_balloon_fixture/assets.rs"]
mod assets;

#[cfg(test)]
#[path = "areka_balloon_fixture/definition.rs"]
mod definition;

#[cfg(test)]
#[path = "areka_balloon_fixture/bake.rs"]
mod bake;

#[cfg(test)]
#[path = "areka_balloon_fixture/online.rs"]
mod online;
