//! サインオフ手順書の判定語が判定器の実装と一致することの檻（task 4.1・要件 8.1／8.5）。
//!
//! 手順書（`.kiro/specs/areka-P0-dpi-transition-atomicity/signoff-procedure.md`）は
//! 「第三者が同一手順を再実行できる粒度」で判定語を並べる文書である。文書の側で語を 1 つ
//! 書き間違えると、採取者は**存在しない語を grep して 0 件を得る**——それは要件 8.5 が
//! 名指しで禁じている「消灯した観測点を発生 0 回の根拠に用いる」形そのものであり、しかも
//! 実機を回した後にしか気づけない。ゆえに文書と判定器の語彙の一致は人の目視ではなく
//! 実行テストで固定する。
//!
//! # 何を固定するか
//!
//! - 手順書に載る観測行の例（`[transition]` を含む行）は、**発行側の単一定義元**が持つ
//!   レコード種別・段階語・フィールド名だけで構成されていること。
//! - 逆向きも固定する: 発行側が持つレコード種別 10 種が**すべて**手順書に現れること。
//!   片側だけだと「例を 1 本も書かなければ緑」という恒真の檻になる（本仕様で 2 度出た形）。
//! - 判定器の入口の語（環境変数名・観測 target・行頭タグ・Report が刷る 2 系統の名前と
//!   合格語・ランナーのテスト名）が字面で載っていること。
//!
//! # 檻そのものが壊れていないこと
//!
//! 検査に使う述語は、壊した行を実際に落とすことを同じテストの中で確かめる（緑は道具が
//! 壊れていても出る・記憶〈検証の道具そのものが壊れる・較正せよ〉）。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_present::presenter::{KIND_SURFACE, SURFACE_FIELDS, SURFACE_STAGE_ALL};
use wintf::ecs::window::transition_diag::{
    ENQUEUE_FIELDS, FIELD_FRAME, FIELD_KIND, FIELD_STAGE, FIELD_T_US, FLUSH_FIELDS, KIND_ALL,
    KIND_ENQUEUE, KIND_FLUSH, KIND_MONITOR, KIND_MSG, KIND_WRITE, MONITOR_FIELDS, MSG_FIELDS,
    RECORD_PREFIX_TAG, STAGE_ALL, TRANSITION_TARGET, WRITE_FIELDS,
};

use super::TRANSITION_LOG_ENV;
use crate::placement::transition_diag::{
    CHAIN_FIELDS, CHAIN_STAGE_ALL, GROUND_FIELDS, HOLD_FIELDS, KIND_CHAIN, KIND_GROUND, KIND_HOLD,
    KIND_SNAPSHOT, PLACEMENT_KIND_ALL, SNAPSHOT_FIELDS,
};

/// 手順書のリポジトリ相対パス（`crates/areka` から見た相対）。
const PROCEDURE_RELATIVE_PATH: &str =
    "../../.kiro/specs/areka-P0-dpi-transition-atomicity/signoff-procedure.md";

/// 手順書の本文を読む。読めなければ**失敗**（無い文書に対して緑を出さない）。
fn procedure_text() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(PROCEDURE_RELATIVE_PATH);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "サインオフ手順書を読めない: {} ({error})。task 4.1 の成果物であり、\
             本檻はこの文書の判定語が判定器と一致することだけを見る",
            path.display()
        )
    })
}

/// レコード種別 → その種別の必須フィールド列（接頭語を除く）。
///
/// 3 crate の単一定義元をここで 1 枚の表に束ねる（本檻は表を作るだけで、語は 1 つも
/// 自前で書かない）。
fn fields_by_kind() -> BTreeMap<&'static str, &'static [&'static str]> {
    BTreeMap::from([
        (KIND_MONITOR, MONITOR_FIELDS),
        (KIND_WRITE, WRITE_FIELDS),
        (KIND_FLUSH, FLUSH_FIELDS),
        (KIND_MSG, MSG_FIELDS),
        (KIND_ENQUEUE, ENQUEUE_FIELDS),
        (KIND_SURFACE, SURFACE_FIELDS),
        (KIND_SNAPSHOT, SNAPSHOT_FIELDS),
        (KIND_HOLD, HOLD_FIELDS),
        (KIND_GROUND, GROUND_FIELDS),
        (KIND_CHAIN, CHAIN_FIELDS),
    ])
}

/// レコード種別 → その種別の**任意**フィールド列。
///
/// 表は判定器（`transition_judge::optional_fields`）が持つ 1 枚を引くだけで、語は 1 つも
/// 自前で書かない。
fn optional_fields(kind: &str) -> &'static [&'static str] {
    crate::placement::transition_judge::optional_fields(kind)
}

/// 発行側が持つレコード種別の全体（3 crate の `*_ALL` の和）。
fn all_kinds() -> BTreeSet<&'static str> {
    KIND_ALL
        .iter()
        .chain(PLACEMENT_KIND_ALL.iter())
        .copied()
        .chain(std::iter::once(KIND_SURFACE))
        .collect()
}

/// 発行側が持つ段階語の全体（`stage=` の値域）。
fn all_stages() -> BTreeSet<&'static str> {
    STAGE_ALL
        .iter()
        .chain(SURFACE_STAGE_ALL.iter())
        .chain(CHAIN_STAGE_ALL.iter())
        .copied()
        .collect()
}

/// 行から観測レコード部分（行頭タグ以降）を切り出す。
///
/// 手順書の例は `DEBUG wintf::transition: [transition] frame=…` の形で載る（実機ログの
/// 見た目そのまま）ので、タグより前は捨てる。
///
/// **観測行の例と見なすのは、行頭タグの直後が `frame=` の行だけ**である——それが発行側の
/// `record_prefix` が必ず作る形であり、本文中で語そのものに言及した行（タグを引用符で
/// 括っただけの散文）を例と取り違えないための境界である。緩めると散文が偽の失敗を作り、
/// 広げすぎると例を 1 本も持たない文書が緑になる。
fn record_part(line: &str) -> Option<&str> {
    let head = format!("{RECORD_PREFIX_TAG} {FIELD_FRAME}=");
    line.find(&head).map(|at| &line[at..])
}

/// 観測レコード 1 行を、発行側の語彙だけで書かれているか検査する。
///
/// 返す `Err` は「どの語が語彙に無いか」を名指しする（沈黙で落とさない）。
fn validate_record_line(record: &str) -> Result<(), String> {
    let fields: Vec<(&str, &str)> = record
        .split_ascii_whitespace()
        .skip(1) // 行頭タグ
        .map(|token| match token.split_once('=') {
            Some(pair) => pair,
            None => (token, ""),
        })
        .collect();

    let mut named: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, value) in &fields {
        // 同名キーの 2 度出しは判定側（後勝ちの辞書化）でレコード種別を消す事故の元。
        if named.insert(name, value).is_some() {
            return Err(format!("フィールド名 `{name}` が 1 行に 2 度出ている"));
        }
    }

    for required in [FIELD_FRAME, FIELD_T_US, FIELD_KIND] {
        if !named.contains_key(required) {
            return Err(format!("接頭語のフィールド `{required}=` が無い"));
        }
    }

    let kind = named[FIELD_KIND];
    let table = fields_by_kind();
    let Some(required_fields) = table.get(kind) else {
        return Err(format!(
            "`{FIELD_KIND}={kind}` は発行側の語彙に無い（実在するのは {:?}）",
            all_kinds()
        ));
    };

    let mut allowed: BTreeSet<&str> = required_fields.iter().copied().collect();
    allowed.extend([FIELD_FRAME, FIELD_T_US, FIELD_KIND]);
    // 任意フィールド（発行側は必ず載せるが、是正前の採取ログには無いので必須にしていない）。
    // 手順書がこれを引用できないと、採取者は行に在る語を手順書で確かめられなくなる。
    allowed.extend(optional_fields(kind).iter().copied());
    for (name, _) in &fields {
        // `snapshot` だけは可変長の `m<i>=` を持つ（`snapshot_line` の実装どおり）。
        let is_monitor_slot = kind == KIND_SNAPSHOT
            && name.starts_with('m')
            && name.len() > 1
            && name[1..].chars().all(|c| c.is_ascii_digit());
        if !allowed.contains(name) && !is_monitor_slot {
            return Err(format!(
                "`{FIELD_KIND}={kind}` の行に語彙外のフィールド `{name}=` がある"
            ));
        }
    }
    for required in required_fields.iter() {
        if !named.contains_key(required) {
            return Err(format!(
                "`{FIELD_KIND}={kind}` の必須フィールド `{required}=` が例から欠けている"
            ));
        }
    }

    if let Some(stage) = named.get(FIELD_STAGE)
        && !all_stages().contains(stage)
    {
        return Err(format!(
            "`{FIELD_STAGE}={stage}` は発行側の語彙に無い（実在するのは {:?}）",
            all_stages()
        ));
    }

    Ok(())
}

#[test]
fn the_procedure_only_quotes_record_lines_the_emitters_can_produce() {
    let text = procedure_text();
    let examples: Vec<&str> = text.lines().filter_map(record_part).collect();

    // 例が 1 本も無ければ、下の検査は恒真になる（零件の主張には陽性の対を置く）。
    assert!(
        examples.len() >= all_kinds().len(),
        "手順書の観測行の例が {} 本しかない（レコード種別 {} 種ぶんは要る）",
        examples.len(),
        all_kinds().len()
    );

    for record in &examples {
        if let Err(reason) = validate_record_line(record) {
            panic!("手順書の観測行の例が発行側の語彙と食い違う: {reason}\n  行: {record}");
        }
    }
}

#[test]
fn the_procedure_names_every_record_kind_the_emitters_produce() {
    let text = procedure_text();
    // 点灯表が 1 種でも落ちていると、その観測点は「手順書が触れていない」＝採取者が
    // 点灯を確かめる術を持たないまま 0 件を根拠にできてしまう（要件 8.5）。
    let missing: Vec<&str> = all_kinds()
        .into_iter()
        .filter(|kind| !contains_token(&text, &format!("{FIELD_KIND}={kind}")))
        .collect();
    assert!(
        missing.is_empty(),
        "手順書が `{FIELD_KIND}=` で名指ししていないレコード種別がある: {missing:?}"
    );
}

/// 語を**トークン境界つき**で探す。
///
/// 素の `contains` だと `AREKA_TRANSITION_LOGS` のような**後ろに字が付いた誤記**が
/// 部分一致で通ってしまい、実機で環境変数を読ませたときに初めて気づくことになる
/// （この緩さは実際に本檻の較正で見つかった）。語の端が識別子文字なら、その外側が
/// 識別子文字でないことまで確かめる。
fn contains_token(haystack: &str, needle: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let head_is_ident = needle.chars().next().is_some_and(ident);
    let tail_is_ident = needle.chars().last().is_some_and(ident);
    haystack.match_indices(needle).any(|(at, _)| {
        let before_ok = !head_is_ident || !haystack[..at].chars().next_back().is_some_and(ident);
        let after_ok = !tail_is_ident
            || !haystack[at + needle.len()..]
                .chars()
                .next()
                .is_some_and(ident);
        before_ok && after_ok
    })
}

#[test]
fn the_procedure_names_the_runner_vocabulary_verbatim() {
    let text = procedure_text();
    // 判定器・ランナーの入口の語。字面が 1 つでもずれると手順書のコマンドは動かない。
    for word in [
        TRANSITION_LOG_ENV,
        TRANSITION_TARGET,
        RECORD_PREFIX_TAG,
        // Report が刷る 2 系統の名前（`transition_judge_verdict::write_family` の書式は
        // `    {family}: …` ゆえコロンまで含めて確かめる）と合格語。
        "deterministic:",
        "signoff:",
        "PASS",
        // ランナーのテスト名と、それを 1 本だけ選ぶ `cargo test` のフィルタ語。
        "transition_signoff",
        "judges_a_real_machine_transition_log",
    ] {
        assert!(
            contains_token(&text, word),
            "手順書が判定器の語 `{word}` を 1 度も書いていない（部分一致でごまかさない）"
        );
    }
}

#[test]
fn the_word_check_is_not_satisfied_by_a_longer_word() {
    // 較正: 語の一部を含むだけの文書は通ってはならない。
    assert!(contains_token(
        "値は AREKA_TRANSITION_LOG である",
        TRANSITION_LOG_ENV
    ));
    assert!(!contains_token(
        "値は AREKA_TRANSITION_LOGS である",
        TRANSITION_LOG_ENV
    ));
    assert!(!contains_token(
        "target は wintf::transition_diag である",
        TRANSITION_TARGET
    ));
}

#[test]
fn the_line_validator_rejects_broken_records() {
    // 檻そのものの較正。上の 3 本が緑でも、述語が何も落とせなければ意味が無い。
    let sound = format!(
        "{RECORD_PREFIX_TAG} {FIELD_FRAME}=7 {FIELD_T_US}=1200 {FIELD_KIND}={KIND_MONITOR} \
         entity=12v1 old_dpi=192 new_dpi=96 old_wa=0,0,2880,1704 new_wa=0,0,2880,1752"
    );
    validate_record_line(&sound).expect("発行側の語彙どおりの行は通る");

    for (broken, expected) in [
        (sound.replace("kind=monitor", "kind=moniter"), "語彙に無い"),
        (sound.replace("new_dpi=", "new_dpix="), "語彙外のフィールド"),
        (sound.replace("frame=7 ", ""), "接頭語のフィールド"),
        (
            format!("{sound} {FIELD_KIND}={KIND_WRITE}"),
            "1 行に 2 度出ている",
        ),
    ] {
        let error = validate_record_line(&broken)
            .expect_err("壊した行が通るなら、この檻は何も守っていない");
        assert!(
            error.contains(expected),
            "落ちた理由が説明になっていない: {error}"
        );
    }

    // 必須フィールドの欠落も落ちること（`-` 番兵は「書いてある」ので落ちない）。
    let missing_field = sound.replace(" new_wa=0,0,2880,1752", "");
    let error = validate_record_line(&missing_field).expect_err("必須フィールドの欠落は落ちる");
    assert!(error.contains("欠けている"), "{error}");
}

// ---------------------------------------------------------------------------
// Report の出力例に並ぶ違反が、その上限系統で実際に出得るか（是正 1・2 の階級）
// ---------------------------------------------------------------------------
//
// 手順書は「出力は次の形である」と言って Report の例を貼る。そこで違反を**誤った系統の
// 下に置く**と、採取者は §6.6 の 2 行を逆に埋める——しかも実機を回すまで気づけない。
// 例の行が語彙として正しいことは前段の検査が見るが、**どちらのブロックに属するか**は
// 見ていなかった（初版はこれで `MismatchFrames` を `signoff:` の下に置いたまま全緑だった）。
//
// 分類は手書きしない。上限の組そのものに最大違反の判定量を当て、**判定器が実際に積んだ
// 違反**から系統ごとの語を起こす。こうすると `Bounds` の armed 項目が変わった日に、
// 本検査の分類も自動で追随する（表を 2 箇所に持つと必ず片方が腐る）。

use super::{Bounds, TransitionSummary, Violation, WindowKey, judge};

/// 例示ブロックで系統を切り替える見出し（`transition_judge_verdict::write_family` の書式）。
const FAMILY_DETERMINISTIC: &str = "deterministic:";
const FAMILY_SIGNOFF: &str = "signoff:";

/// 判定対象の窓（例示ブロックと同じ形の鍵）。
fn sample_window() -> WindowKey {
    WindowKey {
        scope: Some(0),
        kind: "char".to_owned(),
    }
}

/// 量が**在って上限を超えている**判定量（各系統の「上限超」の腕を全部通す）。
fn summary_with_every_quantity_out_of_bounds() -> TransitionSummary {
    let mut summary = TransitionSummary::empty(None);
    summary.malformed_records = 3;
    summary.frames_indeterminate = true;
    summary.frames_to_last_write = Some(9);
    summary.writes_per_window.insert(sample_window(), 4);
    summary.path_a_writes = 2;
    summary.sync_stage_writes = 2;
    summary.balloon_same_frame = false;
    summary
        .mismatch_frames_per_window
        .insert(sample_window(), 5);
    summary.chain_realigned = 3;
    summary.ground_diff_max = Some(-48);
    summary
        .visualize_to_write_us
        .insert(sample_window(), 999_999);
    summary.flush_total_us_max = Some(999_999);
    summary
}

/// 量が**欠けている**判定量（「判定対象の量が欠けている」の腕を通す）。
fn summary_with_every_quantity_missing() -> TransitionSummary {
    TransitionSummary::empty(None)
}

/// 違反 1 件の字面から、数値を除いた見出し（実測値に依らず種別を指す部分）を採る。
///
/// 違反の書式は「語 実測 > 上限 上限」の形なので、最初の数字より前が種別の署名になる。
/// 数字を含まない違反（随伴が別フレーム・量が欠けている 等）は全文が署名になる。
fn violation_signature(rendered: &str) -> String {
    let head = rendered
        .find(|c: char| c.is_ascii_digit())
        .map_or(rendered, |at| &rendered[..at]);
    head.trim().to_owned()
}

/// **判定器に訊く**: この上限の組で出得る違反の署名の全体。
fn signatures_producible_by(bounds: &Bounds) -> BTreeSet<String> {
    let mut signatures = BTreeSet::new();
    for summary in [
        summary_with_every_quantity_out_of_bounds(),
        summary_with_every_quantity_missing(),
    ] {
        if let Err(violations) = judge(&summary, bounds) {
            for violation in violations {
                let signature = violation_signature(&violation.to_string());
                // 署名が空になるのは、その違反の Display が数字で始まったときである。空の署名は
                // 系統の集合へ入ると数字で始まる任意の行に一致してしまい、誤帰属を静かに見逃す。
                assert!(
                    !signature.is_empty(),
                    "違反の Display が数字で始まっており署名が空になる: {violation}"
                );
                signatures.insert(signature);
            }
        }
    }
    signatures
}

/// 例示ブロックを走査し、**その系統では出得ない**違反行を返す。
///
/// 走査対象は「字下げされた `- ` の行」で、直前に現れた系統見出しへ属させる。素の
/// Markdown 箇条書きは行頭に字下げが無いので混ざらない。系統見出しの外の行はコードの
/// 囲いの終わりとみなして系統を解除する。
fn misattributed_violations(text: &str) -> Vec<String> {
    let deterministic = signatures_producible_by(&Bounds::deterministic());
    let signoff = signatures_producible_by(&Bounds::signoff());

    let mut offenders = Vec::new();
    let mut family: Option<(&str, &BTreeSet<String>)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(FAMILY_DETERMINISTIC) {
            family = Some((FAMILY_DETERMINISTIC, &deterministic));
            continue;
        }
        if trimmed.starts_with(FAMILY_SIGNOFF) {
            family = Some((FAMILY_SIGNOFF, &signoff));
            continue;
        }
        let is_indented_bullet = line.starts_with(char::is_whitespace) && trimmed.starts_with("- ");
        match (&family, is_indented_bullet) {
            (Some((name, producible)), true) => {
                let rendered = trimmed.trim_start_matches("- ");
                let signature = violation_signature(rendered);
                if !producible.contains(&signature) {
                    offenders.push(format!(
                        "`{name}` の下に置かれているが、その上限の組では出得ない違反である: {rendered}"
                    ));
                }
            }
            // 系統見出しの直後に続く字下げ行以外が来たら囲いの外へ出たとみなす。
            (Some(_), false) => family = None,
            (None, _) => {}
        }
    }
    offenders
}

/// 例示ブロックで実際に検査できた違反行の本数（恒真でないことの裏取りに使う）。
fn counted_violation_lines(text: &str) -> usize {
    let mut count = 0usize;
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(FAMILY_DETERMINISTIC) || trimmed.starts_with(FAMILY_SIGNOFF) {
            inside = true;
            continue;
        }
        let is_indented_bullet = line.starts_with(char::is_whitespace) && trimmed.starts_with("- ");
        if inside && is_indented_bullet {
            count += 1;
        } else if inside {
            inside = false;
        }
    }
    count
}

#[test]
fn the_two_bounds_families_produce_different_violations() {
    // 分類そのものの前提。2 つの署名集合が同じなら、下の検査は何も分けられない。
    let deterministic = signatures_producible_by(&Bounds::deterministic());
    let signoff = signatures_producible_by(&Bounds::signoff());
    assert!(!deterministic.is_empty() && !signoff.is_empty());
    assert_ne!(
        deterministic, signoff,
        "2 系統が同じ違反しか出さないなら、系統の取り違えは原理的に検出できない"
    );

    // `frame_bound` の門の内側にある違反は決定論系統だけが出す。この非対称こそ、
    // 手順書の例示ブロックで 1 度取り違えられた当のものである。
    //
    // 比べる相手は**変種そのものから起こした署名**であって、字面の部分一致ではない
    // ——「フレーム差」を含む語には門の外にある `FramesIndeterminate`
    // （観測が壊れている＝どちらの系統でも出る）も居り、部分一致で書くと本検査は
    // その共有の違反を捕まえて誤って赤になる（初版がこれで落ちた）。
    for gated in [
        Violation::MismatchFrames {
            window: sample_window(),
            frames: 1,
            max: 0,
        },
        Violation::BalloonWrittenInAnotherFrame,
    ] {
        let signature = violation_signature(&gated.to_string());
        assert!(
            deterministic.contains(&signature),
            "決定論系統が出すはずの違反が出ていない: {signature}"
        );
        assert!(
            !signoff.contains(&signature),
            "実機専用系統がフレーム単位の違反を出している             （`Bounds::signoff` が `frame_bound` を armed にした？）: {signature}"
        );
    }

    // 逆向き: 観測が壊れていることを言う違反は門の外にあり、**両系統で**出る。
    // これを片方だけの語と思い込むと、上の非対称の主張が言い過ぎになる。
    let shared = violation_signature(&Violation::FramesIndeterminate.to_string());
    assert!(
        deterministic.contains(&shared) && signoff.contains(&shared),
        "壊れた観測はどちらの上限も支えないはずである: {shared}"
    );
}

#[test]
fn the_procedure_attributes_every_example_violation_to_the_right_bounds_family() {
    let text = procedure_text();

    // 恒真でないことの裏取り: 例示ブロックに違反行が 1 本も無ければ下の主張は空虚。
    let counted = counted_violation_lines(&text);
    assert!(
        counted >= 2,
        "Report の例示ブロックで検査できた違反行が {counted} 本しかない\
         （2 系統ぶんの例が要る）"
    );

    let offenders = misattributed_violations(&text);
    assert!(
        offenders.is_empty(),
        "手順書の Report 例が違反を誤った上限系統へ帰属させている:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn the_attribution_check_catches_a_violation_placed_under_the_wrong_family() {
    // 較正（陽性の対）: 正しい帰属は素通しし、入れ替えた帰属は必ず捕まえること。
    // 上の検査は「0 件であること」の主張なので、この対が無いと恒真と区別できない。
    let correct = "\
    deterministic: 1 件の違反
      - 可視化と書込のフレーム差 1 > 上限 0（scope=0 win_kind=char）
    signoff: 1 件の違反
      - 一括書込の総所要 286000µs > 上限 16667µs
";
    assert!(
        misattributed_violations(correct).is_empty(),
        "正しい帰属を誤りと報告している"
    );

    let swapped = "\
    deterministic: 1 件の違反
      - 一括書込の総所要 286000µs > 上限 16667µs
    signoff: 1 件の違反
      - 可視化と書込のフレーム差 1 > 上限 0（scope=0 win_kind=char）
";
    let offenders = misattributed_violations(swapped);
    assert_eq!(
        offenders.len(),
        2,
        "入れ替えた 2 行の両方が捕まらなければ、片方向しか見ていない: {offenders:?}"
    );
    assert!(
        offenders
            .iter()
            .any(|o| o.contains(FAMILY_SIGNOFF) && o.contains("フレーム差")),
        "初版の誤り（フレーム差を実機専用系統の下に置く）が捕まっていない: {offenders:?}"
    );

    // 語彙としては正しいが、どちらの系統でも出得ない文言も落ちること。
    let invented = "\
    signoff: 1 件の違反
      - 窓が跳ねた 3 > 上限 0
";
    assert!(
        !misattributed_violations(invented).is_empty(),
        "判定器が 1 度も刷らない文言が素通りしている"
    );
}
