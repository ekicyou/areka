//! 実 pasta 実走テスト（件 1 の試験・areka-P0-emo2-real-run-stability 要件 1.1・1.8・1.9・2.1）＋人間サインオフ手順。
//!
//! 「emo2 が起動して喋る」最終統合（M-boot）を、**実 pasta.dll・実表示・実 DPI**で
//! 任意に追験するための env-gate opt-in ハーネス。観測の二段（決定論 spine ＋ 本 env-gate 実走）の
//! うち後者に相当し、決定論 spine（`crates/areka/src/emo2_boot/spine.rs`）が headless GPU で
//! 押さえ切れない「実表示・実 helper・実 DPI」の最終確認を人間判断に委ねる。
//!
//! ## 既定 OFF（DoD 非前提・要件 1.8）
//! 環境変数 [`REAL_RUN_ENV`]（`AREKA_EMO2_REAL_RUN`）が **未設定／空／空白のみ／非 UTF-8** のときは
//! 子を起こさず即座に `return`（skip）する。ゆえに常設スイート（`cargo test --workspace`・外部 CI なし・
//! ローカル DoD ゲート）では**実 helper／実表示に一切依存せず自明に緑**を保つ。この skip 経路こそが
//! 常設スイートでの観測可能な完了条件であり、実走そのものは DoD の前提にしない（人間 opt-in の追験）。
//!
//! ## 実走（env セット時・要件 1.1）
//! 1 回の走りは共通の手続き（`emo2_real_run/common.rs` の `spawn_emo2`＋`boot_and_close`）に任せる。
//! 実機の試験の窓口 `real-run-kit` で、`CARGO_BIN_EXE_areka`（Cargo が統合テスト用に先ビルドして渡す
//! **実バイナリ**のパス）を emo2 の検体（`Launch::Sample { emo2, emo2-kakukaku }`）の新しい写しで立て、
//! 印を順に待つ:
//! - wire 成立マーカー `emo2-boot: 実 sink 結線が成立しました（wire 成立）`
//! - attach 完了マーカー `emo2 attach: 装着計画を実行`
//! - 折返し ON 解決の証跡 `wrap=BudouxWordWrap`
//!
//! 印 `wrap=BudouxWordWrap` を見てから**利用者と同じ閉じ方で閉じ**、別れの台詞の後に終了コード 0 で
//! 終わることを見る。保険の自動終了（共通の手続きの `SAFETY_EXIT`＝45 秒）は、閉じ方が働かなかったときに
//! 子を残さないためだけにある。保険の自動終了で終わった回・印の前に子が終わった回・終了コードが 0 でない回は
//! 赤とし、子の出力を添えて報告する。子は開発者の `AREKA_ROOT`・`AREKA_PROFILE_DIR` を受け継がず、
//! 回の空の根・回の記憶の置き場（`target\` の下）・空いた MCP の番号で立つ（D-14）。
//!
//! 折返しモードの実機証跡について: balloon descript 基層 fixture
//! `検体 emo2 の同梱バルーン emo2-kakukaku の descript.txt` に
//! `budoux_newline,1` を追記してある（sakura／kero 両バルーンへ効く基層 1 行）。ゆえに実バイナリが
//! 実 balloon model を解決してテキスト供給面を装着した時点で、装着 info! が
//! `wrap=BudouxWordWrap` を残す（＝折返し ON として解決された証跡）。
//!
//! ## 時間の 1 行（要件 2.1）
//! 走りが緑なら、標準エラーへ ASCII の 1 行を出す（`-- --nocapture` を付けると見える）。例:
//!
//!     emo2_real_run: wire_ms=812 attach_ms=1540 wrap_ms=6260 close_ms=22104 talk_to_wrap_ms=310 attach_to_talk_ms=4410 primary_dpi=192
//!
//! - `wire_ms`／`attach_ms`／`wrap_ms`: 起こした直後から各印が見えるまでのミリ秒。
//! - `close_ms`: 閉じる知らせから子が終わるまで（別れの台詞を含む）のミリ秒。
//! - `talk_to_wrap_ms`: 記録の時刻で読んだ最初の台詞（`event="boot_talk"`）の行から `wrap=` の行まで。
//! - `attach_to_talk_ms`: 装着の行から最初の台詞の行まで。正なら台詞が後、負なら装着が後（回ごとに変わる）。
//! - `primary_dpi`: 主のモニタの DPI（200% なら 192、100% 相当なら 96）。
//! - 行が見つからない値は `none`。
//!
//! ## 回し方
//! 排他テストの机を借りてから（`doc/impl-watch.md`）:
//!
//!     $env:AREKA_EMO2_REAL_RUN="1"; cargo test -p areka --test emo2_real_run -- --nocapture
//!
//! 100% 相当で回すときは、子にだけ `__COMPAT_LAYER` を渡す `AREKA_EMO2_COMPAT_LAYER` に互換モードの値を置く
//! （まず `DPIUNAWARE`、効かなければ `GDIDPISCALING DPIUNAWARE`）。時間の 1 行の `primary_dpi=96` で効いたことを見る:
//!
//!     $env:AREKA_EMO2_REAL_RUN="1"; $env:AREKA_EMO2_COMPAT_LAYER="DPIUNAWARE"; cargo test -p areka --test emo2_real_run -- --nocapture
//!
//! ## 実走の前提（人間が opt-in 時に用意する・既定 OFF 経路は非依存）
//! - **実 pasta helper**: `default_helper_exe_path()`（`crates/areka/src/main.rs`）は
//!   `current_exe()` 隣の `shiori-host32-helper.exe` を解決する。窓口が i686 の成果物を実バイナリ
//!   隣へ揃えるので、先に `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` で
//!   建てておく。
//! - **実表示**: attach 完了マーカーは `GhostWindows`＋GPU 資源ゲート成立フレームで
//!   初めて発火する。モニタ 0 台の headless 環境では「起動窓を開けません」の `error!` を残して
//!   終了コード 1 で終わり attach 完了マーカーが出ない（＝本テストは headless では成立しない）。これは env-gate
//!   opt-in の設計意図どおりであり、実表示のある実機でのみ緑になる。

// 1 回の走りだけを使い、くり返しの試験が使う残り（条件の読み・失敗の分け）の警告を抑える。
#[allow(dead_code)]
#[path = "emo2_real_run/common.rs"]
mod common;

/// 実走を有効化する opt-in 環境変数（未設定／空なら即 skip・要件 1.8・DoD 非前提）。
const REAL_RUN_ENV: &str = "AREKA_EMO2_REAL_RUN";

// ===========================================================================
// 実 DPI 人間サインオフ手順
// ===========================================================================
//
// **マイルストーン（M-boot）完了は、この人間サインオフを経てのみ宣言される。AI 単独では宣言しない**。
// 本ファイルの自動の確かめ（印 3 つ・利用者と同じ閉じ方での終了コード 0）は自動部を押さえるに過ぎず、
// 以下の目視確認（実 DPI・実発話・実ドラッグ）は人間が実機で行う。
//
// ## 前提
// - 実表示のある実機で行う（開発機の主モニタの既定は 200%）。
// - 実 pasta helper（`shiori-host32-helper.exe`）を実バイナリ隣へ配置する（本ファイル冒頭 doc 参照）。
//
// ## 目視チェックリスト（4 点・全て満たして初めてサインオフ）
//   1. **実サーフェス表示位置**: 起動だけで emo2 の実サーフェス（立ち絵）が既定位置に表示される
//      （不可視のまま・座標破綻・画面外沈み込みが無い）。
//   2. **typewriter 進行**: OnBoot 応答トークがバルーンへ typewriter 進行で流れる（一括表示でなく
//      1 文字ずつ）。
//   3. **ドラッグ追従**: マスコットをドラッグするとサーフェスがポインタへ追従する（window-placement
//      のドラッグ機構）。
//   4. **close→静かな終了**: 窓を閉じると別れの台詞（OnClose 応答）を経てから全エンジンが静かに正常終了する
//      （会話が途中で切れず・後片付けが行われる）。
//
// ## 折返し（budoux_newline）の目視について
// balloon descript 基層 fixture（`検体 emo2 の同梱バルーン emo2-kakukaku の
// descript.txt`）に `budoux_newline,1` を追記済みゆえ、実機では sakura／kero 両バルーンとも折返し ON で
// 起動する。目視では **バルーンのテキストが単語（分かち書き）境界で折り返り、語中で不自然に切れない**
// ことを確認する（OFF の 1 文字ずつ折返しとの差）。ON として解決された機械可読の証跡は本ファイルの
// `#[test]` が `wrap=BudouxWordWrap` を待って押さえる。
//
// ## サインオフ実行の実際
// - 上記 1〜4（＋折返しの目視）のうち **2（typewriter）・3（ドラッグ）・4（別れの台詞を経た終了）は、
//   窓が開いたまま人間が観察・操作する必要がある**。目視のための時間を確保するには、本テストではなく、
//   **寛大な有界自動終了**（`180000`＝3 分）を付けて実バイナリを直接起動する:
//
//       $env:RUST_LOG="info,kanade=trace"; $env:AREKA_APP_SMOKE_EXIT_MS="180000"; `
//         target\<profile>\areka.exe <emo2 ghost_root> <emo2 balloon_root>
//
//   （`<emo2 ghost_root>`／`<emo2 balloon_root>` の絶対パスは
//     `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が刷る
//     `folder=`／`balloon.emo2-kakukaku=` の行から得る）。3 分の間に 1〜4＋折返しを目視し、観察後に窓を
//     閉じて別れの台詞を見て、4（静かな終了）を確認する。閉じずに放置しても 3 分で有界に終わる。
//   ※ `180000` は**この人間 opt-in の直接起動専用**であり、締切を持たない。本ファイルの `#[test]` の
//     保険の自動終了（共通の手続きの `SAFETY_EXIT`）は締切より十分小さく保つ（さもなくば締切がハング判定で
//     先に発火する）。両者を混同しない。
// - 本ファイルの `#[test]`（`AREKA_EMO2_REAL_RUN=1` で起動）は、これと独立に**印（wire／attach／
//   `wrap=BudouxWordWrap`）を見てから利用者と同じ閉じ方で閉じ、終了コード 0 で終わること**の機械可読部を
//   押さえる。両者（自動部＋人間目視部）が揃ってサインオフ成立とする。

/// 実 pasta 実走の自動観測（要件 1.1・2.1）。既定 OFF（要件 1.8）で、env セット時のみ共通の手続きで
/// 1 回走り、印 3 つを待って利用者と同じ閉じ方で閉じ、時間の 1 行を標準エラーへ出す。
///
/// env 未設定／空のときは即 skip（`return`）するため、常設 `cargo test --workspace` では実 helper／
/// 実表示に依存せず高速に緑を保つ（この skip が常設スイートでの観測可能な完了条件）。実 DPI・実発話・
/// 実ドラッグの目視サインオフは本ファイル上部の「実 DPI 人間サインオフ手順」に従い人間が行う。
#[test]
fn emo2_real_run_boots_talks_and_exits_zero() {
    // 既定 OFF ゲート（要件 1.8・DoD 非前提）: 未設定／空／空白のみ／非 UTF-8 は全て skip 扱いで即 return。
    // これにより実 helper／実表示のない常設スイートでも本テストは自明に緑（＝観測可能な完了条件）。
    match std::env::var(REAL_RUN_ENV) {
        Ok(v) if !v.trim().is_empty() => { /* env セット済み → 実走へ進む */ }
        _ => {
            eprintln!(
                "emo2_real_run: skipped (opt-in; set {REAL_RUN_ENV}=1 to run the real-pasta / real-DPI verification)"
            );
            return;
        }
    }

    // --- 実走（要件 1.1）: 印を待ち、利用者と同じ閉じ方で閉じる。失敗は共通の手続きが出力を添えて panic する ---
    let (run, started) = common::spawn_emo2(None);
    let times = common::boot_and_close(run, started, false);
    // 時間の 1 行（要件 2.1）。
    eprintln!("{}", times.line("emo2_real_run"));
}
