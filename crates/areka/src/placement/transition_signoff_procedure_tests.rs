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
//! - 逆向きも固定する: 発行側が持つレコード種別が**すべて**、本リポジトリが備える手順書の
//!   **どれか**に現れること。片側だけだと「例を 1 本も書かなければ緑」という恒真の檻になる
//!   （本仕様で 2 度出た形）。守るべき不変は「採取者が grep すべき語が、どこかの手順書に
//!   載っていること」であって「先行仕様の手順書に載っていること」ではない——本檻は手順書が
//!   1 本しか無かった時期に書かれたので、出所を [`PROCEDURE_SOURCES`] という集合に一般化して
//!   ある。まだどの手順書も載せていない種別は [`PENDING_PROCEDURE_KINDS`] へ**明示的に**
//!   記録し、黙って見逃さない（保留欄は自ら消える形にしてある——同定数の doc を見よ）。
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
    KIND_ENQUEUE, KIND_FLUSH, KIND_MONITOR, KIND_MSG, KIND_WINDPI, KIND_WRITE, MONITOR_FIELDS,
    MSG_FIELDS, RECORD_PREFIX_TAG, STAGE_ALL, TRANSITION_TARGET, WINDPI_FIELDS, WRITE_FIELDS,
};

use super::TRANSITION_LOG_ENV;
use crate::placement::transition_diag::{
    CHAIN_FIELDS, CHAIN_STAGE_ALL, FIELD_VERDICT, GROUND_FIELDS, HOLD_FIELDS, KIND_CHAIN,
    KIND_GROUND, KIND_HOLD, KIND_OFFSET, KIND_SNAPSHOT, OFFSET_FIELDS, OFFSET_VERDICT_ALL,
    PLACEMENT_KIND_ALL, SNAPSHOT_FIELDS,
};
use crate::placement::transition_judge_offset::judge_offset_log;

/// 手順書のリポジトリ相対パス（`crates/areka` から見た相対）。
const PROCEDURE_RELATIVE_PATH: &str =
    "../../.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md";

/// 本リポジトリが備える手順書の出所（`crates/areka` から見た相対パス）。
///
/// 採取者が「この語を grep せよ」と読む文書は、時期によって 1 本とは限らない。先行仕様
/// （atom）の手順書は完了アーカイブに在り、**他仕様の文書は書き換えない**（要件 6.6）ので、
/// 新しい種別は自分の仕様の手順書が引き受ける。
///
/// 本仕様（balloon-offset-dpi）の手順書は **task 8.2 が置いた**——出所はランナーと同居する
/// モジュール doc であり（design「実機サインオフ」の 5）、本檻はそれを 1 個の文書として読む。
const PROCEDURE_SOURCES: &[&str] = &[PROCEDURE_RELATIVE_PATH, OFFSET_PROCEDURE_RELATIVE_PATH];

/// 本仕様の手順書のリポジトリ相対パス（`crates/areka` から見た相対）。
///
/// 文書は独立した `.md` ではなく**ランナーのモジュール doc** に置いてある（design の指定）。
/// 本檻は出所をテキストとして読むだけなので、拡張子が `.rs` でも読める——手順書とランナーが
/// 同じファイルに在ることは、手順の字面と実装が離れないという点でむしろ望ましい。
const OFFSET_PROCEDURE_RELATIVE_PATH: &str =
    "src/placement/transition_judge_offset_signoff_tests.rs";

/// どの手順書もまだ名指ししていないレコード種別（**期限つきの保留**）。
///
/// 語彙を先に建てる（task 5.1）と、手順書（task 8.2）が置かれるまでのあいだ「発行側には
/// 在るが、どの手順書にも載っていない」種別が生じる。黙って除外すると
/// 「未達が spec の内側から見えない」形になるので、ここへ名指しで残す。
///
/// **この保留欄は自ら消える。** 檻は ⑴ 載っていない種別が保留欄の内側に収まること
/// （`missing ⊆ pending`）だけでなく、⑵ 保留欄の種別が**まだ本当に載っていない**こと
/// （`pending ∩ covered = ∅`）も見る。task 8.2 が本仕様の手順書を [`PROCEDURE_SOURCES`] へ
/// 足したので、唯一の保留であった `offset` は**ここから消えた**（残していれば ⑵ が赤になる）。
/// 空であることが正常な状態であり、次に保留が生じたときだけ行が増える。
///
/// # 現在の保留
///
/// **無し。** task 8.3 が新設した `windpi` が唯一の保留だったが、task 8.5 が本仕様の手順書へ
/// `kind=windpi` を書いたので⑵の検査が実際に赤くなり（`保留欄に残っているが、もう手順書に
/// 載っている種別がある: ["windpi"]`）、この行を消して緑へ戻した。自壊は設計どおりに働いた。
const PENDING_PROCEDURE_KINDS: &[&str] = &[];

/// 先行仕様（atom）の手順書の本文を読む。読めなければ**失敗**（無い文書に対して緑を出さない）。
fn procedure_text() -> String {
    read_procedure(PROCEDURE_RELATIVE_PATH)
}

/// 出所 1 本の本文を読む。読めなければ**失敗**（無い文書に対して緑を出さない）。
fn read_procedure(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "サインオフ手順書を読めない: {} ({error})。task 4.1 の成果物であり、\
             本檻はこの文書の判定語が判定器と一致することだけを見る",
            path.display()
        )
    })
}

/// 出所集合すべての本文を連結して返す（語の在処の検査は「どれかに載っていれば良い」）。
///
/// 出所の区切りは改行で入れる——連結の継ぎ目が偶然 1 つの語を作らないようにする。
fn procedure_sources_text() -> String {
    PROCEDURE_SOURCES
        .iter()
        .map(|relative| read_procedure(relative))
        .collect::<Vec<_>>()
        .join("\n")
}

/// レコード種別 → その種別の必須フィールド列（接頭語を除く）。
///
/// 3 crate の単一定義元をここで 1 枚の表に束ねる（本檻は表を作るだけで、語は 1 つも
/// 自前で書かない）。
fn fields_by_kind() -> BTreeMap<&'static str, &'static [&'static str]> {
    BTreeMap::from([
        (KIND_MONITOR, MONITOR_FIELDS),
        (KIND_WINDPI, WINDPI_FIELDS),
        (KIND_WRITE, WRITE_FIELDS),
        (KIND_FLUSH, FLUSH_FIELDS),
        (KIND_MSG, MSG_FIELDS),
        (KIND_ENQUEUE, ENQUEUE_FIELDS),
        (KIND_SURFACE, SURFACE_FIELDS),
        (KIND_SNAPSHOT, SNAPSHOT_FIELDS),
        (KIND_HOLD, HOLD_FIELDS),
        (KIND_GROUND, GROUND_FIELDS),
        (KIND_CHAIN, CHAIN_FIELDS),
        (KIND_OFFSET, OFFSET_FIELDS),
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
    // 出所は**集合の全体**を読む（task 8.2）。先行仕様の手順書だけを読んでいると、本仕様の
    // 手順書が引く観測行の例を誰も検査しない＝新しい種別の例が静かに検査の外へ落ちる。
    let text = procedure_sources_text();
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
    let text = procedure_sources_text();
    // 点灯表が 1 種でも落ちていると、その観測点は「どの手順書も触れていない」＝採取者が
    // 点灯を確かめる術を持たないまま 0 件を根拠にできてしまう（要件 8.5）。
    let missing: BTreeSet<&str> = all_kinds()
        .into_iter()
        .filter(|kind| !contains_token(&text, &format!("{FIELD_KIND}={kind}")))
        .collect();
    let pending: BTreeSet<&str> = PENDING_PROCEDURE_KINDS.iter().copied().collect();

    let unrecorded: Vec<&str> = missing.difference(&pending).copied().collect();
    assert!(
        unrecorded.is_empty(),
        "どの手順書も `{FIELD_KIND}=` で名指ししていないレコード種別がある: {unrecorded:?}\n  \
         出所: {PROCEDURE_SOURCES:?}（保留なら {PENDING_PROCEDURE_KINDS:?} へ明示的に記録する）"
    );

    // 保留欄は自ら消える: 既にどこかの手順書が載せた種別が保留欄に残っていたら赤にする。
    let stale: Vec<&str> = pending.difference(&missing).copied().collect();
    assert!(
        stale.is_empty(),
        "保留欄に残っているが、もう手順書に載っている種別がある: {stale:?}\n  \
         task 8.2 が本仕様の手順書を出所集合へ足したら、保留欄の当該行を消すこと"
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
    // ここは**先行仕様の手順書 1 本**を読む（出所集合ではない）。下に並ぶのは先行仕様の
    // ランナーの語であり、集合で読むと「本仕様の手順書が同じ語を書いているから緑」に
    // なってしまう——先行仕様の手順書が語を落としても気づけない。仕様ごとのランナーの語は
    // 仕様ごとの手順書に対して確かめる（本仕様のぶんは下の検査）。
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
fn the_offset_procedure_names_its_runner_vocabulary_verbatim() {
    // 本仕様（追随レコード）の手順書とランナーの語。字面が 1 つでもずれると、採取者は
    // 存在しない語を grep して 0 件を得る（要件 8.5 が名指しで禁じている形）。
    let text = read_procedure(OFFSET_PROCEDURE_RELATIVE_PATH);
    let mut words: Vec<&str> = vec![
        TRANSITION_LOG_ENV,
        TRANSITION_TARGET,
        RECORD_PREFIX_TAG,
        KIND_OFFSET,
        FIELD_VERDICT,
        // ランナーのテスト名と、それを 1 本だけ選ぶ `cargo test` のフィルタ語。
        "transition_judge_offset_signoff",
        "judges_a_real_machine_offset_log",
    ];
    // 判定語は表そのものから起こす（手で並べると、語が増えた日に手順書だけが古くなる）。
    words.extend(OFFSET_VERDICT_ALL.iter().copied());
    for word in words {
        assert!(
            contains_token(&text, word),
            "本仕様の手順書が判定器の語 `{word}` を 1 度も書いていない（部分一致でごまかさない）"
        );
    }
}

#[test]
fn the_offset_procedure_states_what_the_judge_does_not_see() {
    // 「違反 0 件」を「全部正しい」と読ませないための欄（task 8.1 のレビュー由来）。
    // 見出しごと消えると、緑の読み違えが静かに戻ってくる。
    let text = read_procedure(OFFSET_PROCEDURE_RELATIVE_PATH);
    for phrase in [
        "判定器が見ないもの",
        "素の追従スコープ",
        "重なり順",
        "task 10.1",
        // 5 件目（task 8.1 の限界 ⒟ の後半）——最初の起点より前の行は 1 件も判定されない。
        "最初の遷移起点より前に出た記録は 1 件も判定されない",
    ] {
        assert!(
            text.contains(phrase),
            "本仕様の手順書が「判定器が見ないもの」の欄から `{phrase}` を落としている"
        );
    }
}

#[test]
fn the_offset_procedure_forbids_the_three_shapes_that_make_a_false_red() {
    // 手順が ⑴「素の追従スコープを 1 つ含める」⑵「キーワード指定を**素材未消費のまま**
    // 遷移へ 1 度通す」⑶「そのうえで素材を消費させてから低い側へ遷移させる」の 3 つを
    // **必須**として書いていること。どれを落としても、正しい実装のまま赤になるログが
    // 採れてしまう（task 8.1 のレビューが 2 つ、その次のレビューが 3 つ目を名指しした）。
    //
    // ⑵ が要るのは、`check_alignment_residual` がキーワード指定スコープの集合を**遷移の
    // 内側に現れた `keyword-pending` の行だけ**から作るためである。素材を先に消費させると
    // 門は 2 度と出ず、集合が空のまま `measured == 0` になって `NoKeywordAlignmentMeasured`
    // の偽の赤が出る——⑶ だけを書いた手順は、その形へ**まっすぐ導く**。
    let text = read_procedure(OFFSET_PROCEDURE_RELATIVE_PATH);
    for phrase in [
        "キーワード指定でないバルーン",
        "素材未消費のまま遷移へ最低 1 度通す",
        "素材を消費させてから低い拡大率側へ遷移させる",
        "偽の赤",
    ] {
        assert!(
            text.contains(phrase),
            "本仕様の手順書が偽の赤を防ぐ必須の手 `{phrase}` を書いていない"
        );
    }
}

/// 要約行の数が入る 3 箇所を `N`・`M`・`0` へ置き換える（合格は違反 0 件ゆえ 3 つ目は `0`）。
fn with_count_placeholders(line: &str) -> String {
    let mut out = String::new();
    let mut runs = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
            out.push_str(["N", "M", "0"].get(runs).copied().unwrap_or("?"));
            runs += 1;
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn the_offset_procedure_quotes_the_pass_line_the_report_prints() {
    // §6.1 の合格語は `OffsetReport` の `Display` そのものである。書式だけを変えると、
    // 手順書の合格語が静かに古くなり、採取者は**存在しない語を読む**（要件 8.5 が名指しで
    // 禁じている形）。ゆえに文言は手書きせず Display から起こして突合する。
    let printed = judge_offset_log("").to_string();
    let first = printed
        .lines()
        .next()
        .expect("Display は 1 行目に要約を刷る");
    let expected = with_count_placeholders(first);
    let text = read_procedure(OFFSET_PROCEDURE_RELATIVE_PATH);
    assert!(
        text.contains(&expected),
        "本仕様の手順書の合格語が Report の書式と食い違う（期待: `{expected}`）"
    );

    // 較正 ⑴: 置き換えそのものが効いていること（効いていなければ上は恒真になり得る）。
    assert_eq!(
        with_count_placeholders("遷移 12 本・行 3 件・違反 0 件"),
        "遷移 N 本・行 M 件・違反 0 件"
    );
    // 較正 ⑵: 書式を変えた形は載っていない＝上の検査は字面を本当に見ている。
    assert!(!text.contains(&expected.replace('・', ", ")));
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

use super::{Bounds, Quantity, TransitionSummary, Violation, WindowKey, judge};

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
///
/// 書込が 1 件も無い形なので、**窓ごと**の量の欠落だけは起こせない（下の変種が要る）。
fn summary_with_every_quantity_missing() -> TransitionSummary {
    TransitionSummary::empty(None)
}

/// **書込は在るのに窓ごとの量が欠けている**判定量。
///
/// 判定器の被覆検査（`transition_judge_verdict.rs` の ⑼）は `judged_windows` を回るので、
/// `writes_per_window` が空だと 1 度も回らない。窓ごとの 2 つの「量が欠けている」は
/// 上の 2 変種のどちらからも起こせず、この第 3 の形だけが届く
/// （`the_producible_signatures_cover_the_per_window_unmeasured_arms` が固定する）。
///
/// 書込回数は上限（`WRITES_PER_WINDOW_MAX`）の内側に置く——ここで上限超も同時に起こすと、
/// 本変種が足す署名が「窓ごとの書込回数」の側なのか被覆の側なのか読めなくなる。
fn summary_with_written_window_missing_its_quantities() -> TransitionSummary {
    let mut summary = TransitionSummary::empty(None);
    summary.writes_per_window.insert(sample_window(), 1);
    summary
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
        summary_with_written_window_missing_its_quantities(),
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
fn the_producible_signatures_cover_the_per_window_unmeasured_arms() {
    // `signatures_producible_by` は「その上限の組で出得る違反の**全体**」を名乗る集合であり、
    // `misattributed_violations` はそこに載らない行を「出得ない」と断じる。ゆえに集合の
    // 取りこぼしはそのまま**偽陽性**へ跳ね返る——手順書が正しく書いた行を誤りと報告する。
    //
    // 判定器の窓ごとの被覆検査（`transition_judge_verdict.rs:551-565` の走査）が積む 2 つの
    // 「量が欠けている」は、変種を 2 つしか回さない間は**構造的に起こせなかった**——
    // 量が揃った変種では欠落が無く、量が 1 つも無い変種では `writes_per_window` が空で
    // `judged_windows`（同 `:857-863`）が 1 度も回らないためである。「書込は在るのに窓ごとの
    // 量が欠けている」第 3 の形だけがこの 2 腕へ届く。
    let deterministic = signatures_producible_by(&Bounds::deterministic());
    let signoff = signatures_producible_by(&Bounds::signoff());

    let mismatch = violation_signature(
        &Violation::Unmeasured(Quantity::MismatchFrames(sample_window())).to_string(),
    );
    assert!(
        deterministic.contains(&mismatch),
        "窓ごとのフレーム差が欠けている違反を決定論系統が 1 度も出せていない\
         （手順書がこの行を正しく置いても偽陽性になる）: {deterministic:?}"
    );

    let visualize = violation_signature(
        &Violation::Unmeasured(Quantity::VisualizeToWriteUs(sample_window())).to_string(),
    );
    assert!(
        signoff.contains(&visualize),
        "窓ごとの可視化から書込までが欠けている違反を実機専用系統が 1 度も出せていない\
         （手順書がこの行を正しく置いても偽陽性になる）: {signoff:?}"
    );

    // 2 腕は門の内外で分かれている（`frame_bound` 側と `visualize_to_write_us_max` 側）。
    // 集合を広げた結果として系統の非対称が潰れていないことも同時に固定する。
    assert!(
        !signoff.contains(&mismatch),
        "実機専用系統がフレーム単位の欠落を出している: {mismatch}"
    );
    assert!(
        !deterministic.contains(&visualize),
        "決定論系統が実機専用の量の欠落を出している: {visualize}"
    );
}

#[test]
fn the_two_families_share_exactly_the_input_sanity_violations() {
    // §6.2 の帰属規則（「違反行がどちらのブロックの下にあったかで系統を判断する」）には
    // 例外がある——**入力の健全性**を言う違反は上限の門の外で積まれるので、2 系統の下に
    // 同時に並ぶ。規則はこの 3 つに対して「両方」を返し、それが正しい答えである。
    //
    // どれがその 3 つかを手順書の散文だけに置くと静かにずれる（`Bounds` の armed 項目を
    // 動かした日に、門の内外が入れ替わっても文書は何も言わない）。ゆえに判定器から
    // **共通部分として**起こし、手順書がその 3 つを字面で名指ししていることまで固定する。
    let deterministic = signatures_producible_by(&Bounds::deterministic());
    let signoff = signatures_producible_by(&Bounds::signoff());
    let shared: BTreeSet<String> = deterministic.intersection(&signoff).cloned().collect();

    let expected: BTreeSet<String> = [
        Violation::MalformedRecords { count: 3 },
        Violation::FramesIndeterminate,
        Violation::MissingOrigin,
    ]
    .iter()
    .map(|violation| violation_signature(&violation.to_string()))
    .collect();
    assert_eq!(
        shared, expected,
        "両系統の下に同時に並ぶ違反の集合が変わった。手順書 §6.2 の例外の段落を\
         同じコミットで直すこと（門の内外が入れ替わった可能性がある）"
    );

    // 手順書が 3 つとも名指ししていること。字面は判定器から起こすので、Display を
    // 書き換えた日に本検査が落ちる（文書が静かに嘘になる形を塞ぐ）。
    let text = procedure_text();
    for signature in &expected {
        assert!(
            text.contains(signature.as_str()),
            "手順書が「両系統の下に並び得る」違反 `{signature}` を字面で名指ししていない"
        );
    }
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
