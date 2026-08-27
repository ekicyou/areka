//! バルーン追従オフセットの**実機サインオフ手順書**と、その判定ランナー
//! （task 8.2・要件 8.1／8.2／8.3／8.4／8.5）。
//!
//! design.md「実機サインオフ（要件 8）」の 5 は「手順書はモジュール doc として置く」と定める。
//! 本ファイルの module doc が**その手順書そのもの**であり、直下の `#[ignore]` ランナーが
//! 決定論テストと**同一の判定器**（[`super::judge_offset_log`]）を実機ログへ当てる。判定の
//! 実装をここに 1 行も持たないのが要点で、持った瞬間に「決定論テストが緑でもサインオフだけ
//! 別の判定を通る」形が生まれる。
//!
//! 先行仕様の手順書ファイル（完了アーカイブ側）は**書き換えない**——自らの行に限る規律を
//! 検証側にも適用する（要件 6.6）。本手順書が引き受けるのは `kind=offset` の観測点だけである。
//!
//! # 0. 合否を決めるのは誰か（要件 8.3）
//!
//! **合否は記録の機械判定で決める。** 採取者の仕事は「手順どおりに動かしてログを作ること」
//! だけであり、合格・不合格を目で決めてはならない。判定はランナー（§4）が刷る
//! 「違反 0 件」かどうかで読む。目視は §6.3「判定器が見ないもの」を補うためだけに行い、
//! その結果は判定器の合否を上書きしない。
//!
//! # 1. 環境（要件 8.1）
//!
//! 拡大率の異なるモニタを **2 台以上**（125% 相当と 200% 相当）備えた実機で行う。実効 DPI は
//! DPI 対応プロセス自身のログで確かめる——非対応プロセスから読むと全モニタが 96 に丸められ、
//! 「拡大率の差を 1 度も跨いでいないログ」を作ってしまう。
//!
//! # 2. 観測の点灯
//!
//! 観測 target は `wintf::transition`、水準は `debug`。行頭タグは `[transition]` である。
//!
//! ```text
//! RUST_LOG=wintf::transition=debug areka > <絶対パス>\signoff.log 2>&1
//! ```
//!
//! 点灯しているかは、採取後に行頭タグを数えて確かめる（0 件なら採り直し）。
//!
//! ```text
//! grep -c "\[transition\]" <絶対パス>\signoff.log
//! ```
//!
//! 消灯した観測点の採取を「発生 0 回」の根拠にしてはならない（要件 8.5）。ランナーは追随
//! レコードが 1 行も無いログを**失敗**として落とすので、この取り違えは静かには通らない。
//!
//! # 3. 手順（必須の 5 手・1 手でも欠くと判定が立たない）
//!
//! 1. **ゴーストを起動し、バルーンを出す。** 起動直後の配置が済むまで待つ。
//! 2. **素の追従スコープ（キーワード指定でないバルーン）を最低 1 つ含める**（**必須**）。
//!    素の追従とは、位置がキーワード由来の基本位置で決まっていないバルーンである。
//!    *理由*: 判定 ⑶（低い拡大率側で追随が出ていること）の母数は、スコープごとに
//!    「そのスコープが最後に `verdict=keyword-pending` を出した遷移より後」に限られる。
//!    ログに現れるスコープが**すべて**キーワード指定で、しかも素材の消費が最後の低い側の
//!    遷移より後に来ると、数えられる行が 1 件も無くなり「低い拡大率側で追随が出ていない」の
//!    **偽の赤**が出る。素の追従スコープは門を 1 度も出さないので常に母数に入り、この形を塞ぐ。
//! 3. **キーワード指定のバルーンについて、素材を消費させてから低い拡大率側へ遷移させる**
//!    （**必須**）。素材の消費とは、キーワード由来の基本位置を確定させる面切替・発話を
//!    1 度通すことである。消費させたうえで、**その後に**ゴーストを低い拡大率のモニタへ移す。
//!    *理由*: 判定 ⑷（揃えの残差）は、キーワード指定スコープが**素材消費後に**出した
//!    `verdict=rescaled` の行でしか測れない。消費前の遷移しか無いログは残差を 1 度も測れず、
//!    「揃えを 1 度も測れていない」の**偽の赤**になる（受容された残余＝素材未消費のまま
//!    寸据え置きの遷移を迎えた記録は正しい記録であって欠陥ではない）。
//! 4. **ゴーストをモニタ間で往復させる**（125% 側 → 200% 側 → 125% 側 …）。同じ拡大率へ
//!    **戻る**遷移が最低 1 回要る（要件 8.2）。戻らないログは判定 ⑴ を立てられず
//!    「往復が 1 度も観測されていない」になる。往復のあいだ、バルーンをドラッグしない・面を
//!    切り替えない——どちらも基準を引き直すので、往復の区間が切れて突合が成立しなくなる。
//! 5. **先行仕様の残所見を目で確かめる**（要件 8.4 の目視側）: 低い拡大率の側で、バルーンが
//!    キャラに対して定常的にずれていないこと。ずれが見えたら、判定器が緑でも所見として残す。
//!
//! # 4. ランナーの走らせ方
//!
//! ```text
//! AREKA_TRANSITION_LOG=<絶対パス> cargo test -p areka transition_judge_offset_signoff -- --ignored --nocapture
//! ```
//!
//! 既定の `cargo test` では `#[ignore]` により 1 度も走らない——実機ログが無い環境で
//! 「違反 0 件」を出すと、それが充足の根拠に化けるためである（要件 8.5）。ランナーの本体は
//! [`judges_a_real_machine_offset_log`]、判定の入口は [`signoff_offset_log`] である。
//!
//! **静かに成功しない**: 環境変数が未設定・パスが読めない・追随レコードが 1 行も無い——
//! いずれも**失敗**として落ちる。無視指定のテストが不備なパスで緑になるのは、テストが
//! 無いより悪い。
//!
//! # 5. 判定語（`verdict=` の値・全 6 語）
//!
//! | 判定語 | 意味 | 値 |
//! |---|---|---|
//! | `rescaled` | 基準から表示 DPI 比で引き直した | 動き得る |
//! | `anchored` | 未係留の基準を最初の観測で係留した | 動かない |
//! | `unchanged` | 比が恒等（遷移していない） | 動かない |
//! | `keyword-pending` | キーワード由来の素材が未消費＝見送り（門） | 動かない |
//! | `unresolved` | 基準 DPI か現在 DPI の片側だけが 0＝縮退 | 動かない |
//! | `saturated` | 引き直しが `i32` の域で飽和した | 動き得る |
//!
//! 観測行の見た目（採取者が読む形）:
//!
//! ```text
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=monitor entity=12v1 old_dpi=96 new_dpi=192 old_wa=0,0,2880,1752 new_wa=0,0,2880,1704
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=offset scope=0 base_dpi=96 new_dpi=192 base_offset=10,20 old_offset=10,20 new_offset=20,40 verdict=rescaled
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=offset scope=1 base_dpi=- new_dpi=0 base_offset=0,0 old_offset=0,0 new_offset=0,0 verdict=keyword-pending
//! ```
//!
//! 欄が読めなかったところは落とさず番兵 `-` で埋まる（落とすと「記録が出ていない」と
//! 見分けが付かない）。
//!
//! # 6. 合否条件
//!
//! ## 6.1 合格
//!
//! ランナーが刷る 1 行目が「違反 0 件」であり、かつ「遷移 N 本」「offset 行 M 件」の
//! **どちらも 0 でない**こと。0 本・0 件で緑は出ない（ランナーが先に落とす）。
//!
//! ## 6.2 不合格（判定器が立てる違反・全件が刷られる）
//!
//! | 判定 | 要件 | 立つ違反 |
//! |---|---|---|
//! | ⑴ 往復の前後で反映後の値が bit 同一 | 8.2 | 値が bit 同一でない／往復が 1 度も観測されていない |
//! | ⑵ 判定語が期待の腕 | 8.3 | 期待の腕でない／動かさない腕で動いた／語彙表に無い／門が表示 DPI を運んでいる |
//! | ⑶ 低い拡大率側で追随が出ている | 8.4 | 低い拡大率側で `rescaled` が 1 度も出ていない |
//! | ⑷ キーワード指定スコープの揃えの残差 | 8.5 | 残差が許容量を超えた／揃えを 1 度も測れていない |
//! | 入力そのもの | 8.5 | 追随レコードが 1 行も無い／追随レコードを読めない |
//!
//! 違反が 1 件でもあれば不合格である。「材料が無い」種の違反（往復が観測されていない・
//! 低い拡大率側で出ていない・揃えを測れていない・レコードが 1 行も無い）は、**採り方の不備**
//! であることが多い——§3 の必須の手を 1 つ飛ばすと出る。まず手順を見直し、手順どおりでも
//! 出るなら製品側の欠陥として扱う。
//!
//! ## 6.3 判定器が見ないもの（緑を「全部正しい」と読まないための欄）
//!
//! 判定器が緑でも、次の 4 つは**判定されていない**。これらは task 10.1 の目視サインオフ項目が
//! 引き受ける。
//!
//! 1. **素の追従スコープの値が一貫して間違っていても 4 判定すべてを通る。** 揃えの残差の検査
//!    （⑷）はキーワード指定スコープだけを見るので、素の追従の値がずれたまま往復で再現性を
//!    保っていれば、⑴〜⑶ は成立してしまう。
//! 2. **揃えのずれそのものは見えない。** 残差の検査は D8 が数える 3 つの丸めの出所のうち
//!    1 つだけを測り、そこへ 3 つぶんの上限（3px）を当てている。ゆえに真の残差に対して
//!    およそ 6 倍ゆるい。数 px の見た目のずれは緑のまま通る。
//! 3. **オフセット以外の壊れは視野の外。** キャラ窓の位置・重なり順・可視性・バルーンの寸法は
//!    1 つも見ていない。
//! 4. **一部のスコープだけが毎遷移で基準を建て直す欠陥は隠れ得る。** 他のスコープの往復が
//!    成立していれば ⑴ は緑になる。スコープごとの往復の本数は目で確かめる。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use super::transition_judge_offset_tests::{log_of, pass_lines};
use super::{OffsetReport, judge_offset_log};
use crate::placement::transition_diag::KIND_OFFSET;
use crate::placement::transition_judge::TRANSITION_LOG_ENV;
use wintf::ecs::window::transition_diag::FIELD_KIND;

/// 実機ログを読み、追随レコードの判定まで通す。
///
/// `raw_path` は環境変数の値（未設定なら `None`）。I/O の失敗も「追随レコード 0 行」も `Err`
/// にする——どれも「違反 0 件」を作れてしまう入力だからである（要件 8.5）。
fn signoff_offset_log(raw_path: Option<&str>) -> Result<(OffsetReport, String), String> {
    let Some(raw_path) = raw_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Err(format!(
            "{TRANSITION_LOG_ENV} が未設定である。実機ログの絶対パスを与えて \
             `{TRANSITION_LOG_ENV}=<絶対パス> cargo test -p areka transition_judge_offset_signoff -- --ignored --nocapture` \
             で実行する（手順は本モジュールの doc §4）"
        ));
    };
    let path = Path::new(raw_path);
    let log = fs::read_to_string(path).map_err(|error| {
        format!("{TRANSITION_LOG_ENV}={raw_path} を読めない: {error}（絶対パスで与えること）")
    })?;
    let report = judge_offset_log(&log);
    if report.rows == 0 {
        return Err(format!(
            "{TRANSITION_LOG_ENV}={raw_path} に追随レコードが 1 行も無い（{} 文字）。\
             観測 target と水準が有効になっていない採取を「発生 0 回」の根拠にしない（要件 8.5・doc §2）",
            log.len()
        ));
    }
    Ok((report, raw_path.to_owned()))
}

/// 実機ログの判定（既定では走らない）。
#[test]
#[ignore = "実機ログの判定（AREKA_TRANSITION_LOG に絶対パスを与えて明示実行する）"]
fn judges_a_real_machine_offset_log() {
    let raw_path = std::env::var(TRANSITION_LOG_ENV).ok();
    let (report, path) = match signoff_offset_log(raw_path.as_deref()) {
        Ok(judged) => judged,
        Err(message) => panic!("{message}"),
    };

    // 合否によらず判定結果の全文を出す（`--nocapture` で記録へ貼る）。
    println!("== balloon offset signoff: {path} ==");
    print!("{report}");

    assert!(
        !report.failed(),
        "実機ログが判定を満たさない（上の列挙が違反の全件・採り方の不備で出る種は doc §6.2）"
    );
}

// ---------------------------------------------------------------------------
// 入口そのものの檻（既定で走る）
// ---------------------------------------------------------------------------

#[test]
fn a_missing_environment_variable_is_an_error_not_an_empty_pass() {
    for raw_path in [None, Some(""), Some("   ")] {
        let error = signoff_offset_log(raw_path).expect_err("未設定は失敗でなければならない");
        assert!(
            error.contains(TRANSITION_LOG_ENV),
            "何を与えればよいか読めない: {error}"
        );
    }
}

#[test]
fn an_unreadable_path_is_an_error_not_an_empty_pass() {
    let temp = TempPath::new("offset-signoff-missing");
    let missing = temp.child("does-not-exist.log");
    assert!(!missing.exists(), "この檻は存在しないパスを前提にする");
    let error = signoff_offset_log(missing.to_str()).expect_err("読めないパスは失敗");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn a_directory_given_instead_of_a_log_is_an_error_not_an_empty_pass() {
    // 採取先のフォルダを渡す取り違えは実際に起こる。読めない理由が何であれ失敗にする。
    let temp = TempPath::new("offset-signoff-directory");
    let directory = temp.path();
    assert!(directory.is_dir());
    let error = signoff_offset_log(directory.to_str()).expect_err("フォルダは読めない");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn a_log_without_any_offset_record_is_an_error_not_an_empty_pass() {
    // 空のファイル・観測行を 1 行も含まないファイル・観測行は在るが追随レコードだけが
    // 消えているファイルの 3 つ。どれも「違反 0 件」を作れてしまう入力なので、合格ではなく
    // 失敗として落ちなければならない（要件 8.5）。
    let temp = TempPath::new("offset-signoff-empty");
    let path = temp.child("empty.log");
    let kind_offset = format!("{FIELD_KIND}={KIND_OFFSET}");
    let without_offset_rows = log_of(
        &pass_lines()
            .into_iter()
            .filter(|line| !line.contains(&kind_offset))
            .collect::<Vec<_>>(),
    );
    assert!(
        !without_offset_rows.is_empty(),
        "起点の行は残っているはず（そうでないと 3 つ目の入力が 2 つ目と同じになる）"
    );
    for body in ["", "何も観測していないログ\n", &without_offset_rows] {
        fs::write(&path, body).expect("一時ファイルを書けるはず");
        let error = signoff_offset_log(path.to_str()).expect_err("追随レコード 0 行は失敗");
        assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
    }
    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}

#[test]
fn the_runner_reads_the_same_pure_function_as_the_deterministic_tests() {
    // 既知の合格ログを一時ファイルへ落として入口を一巡させ、判定が決定論テストと同じ結論を
    // 返すことを固定する（判定の実装が 2 つに分かれていないことの裏取り）。
    let temp = TempPath::new("offset-signoff-fixture");
    let path = temp.child("fixture.log");
    let log = log_of(&pass_lines());
    fs::write(&path, &log).expect("一時ファイルを書けるはず");

    let (report, echoed) = signoff_offset_log(path.to_str()).expect("既知の合格ログは判定できる");
    assert_eq!(echoed, path.to_str().expect("UTF-8 のはず"));
    assert_eq!(report, judge_offset_log(&log), "入口が別の判定を通っている");
    assert!(!report.failed(), "既知の合格ログが赤になった:\n{report}");
    assert_eq!(report.transitions.len(), 4, "{report}");

    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}
