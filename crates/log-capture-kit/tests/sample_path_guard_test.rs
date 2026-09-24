//! 検体ゴースト／バルーンの在処を窓口の外で綴っていないか見張る常設検査（要件 1.7・1.8・10.3）。
//!
//! # 何を見張るか
//!
//! 検体の置き場を指す綴りが、窓口 crate の定義（`crates/sample-ghost-kit/src/`）の外の
//! **実行行**に 1 件も無いこと。綴りは 4 つの形で走査する（設計「見張り
//! sample_path_guard_test.rs」の ⑴〜⑷）。
//!
//! | 形 | 何を捕まえるか |
//! |----|----------------|
//! | ⑴ 旧置き場 | パイロット crate の example 配下に検体が置かれていた頃のパス |
//! | ⑵ 展開形の検体フォルダ | `vendors/sample_ghost` の下に検体名を継ぎ足した形 |
//! | ⑶ 展開先の名前空間 | 段 ③ で `target/` の下に作る展開先のフォルダ名 |
//! | ⑷ 同梱バルーンのパス組み | ゴーストのフォルダにバルーン名を継ぎ足して自分でパスを作る形 |
//!
//! ⑷ が禁じるのは**パスを組む形だけ**である。窓口の読み口へ名前を引数で渡す書き方
//! （`SampleRoot::balloon` や `SampleRoot::acquire` の実引数）は要件 1.3 が定める正規の
//! 使い方なので当たってはならない。説明文の中に名前が出てくるのも当たらない。
//! 両側は [`the_window_reader_arguments_and_prose_are_not_hits`] が固定する。
//!
//! # 走査語を手書きの表にしない
//!
//! ⑵ と ⑷ の語は登記表 `SAMPLES` から組み立てる。検体名・同梱バルーン名を見張り側へ
//! 手で写すと、検体を 1 つ足した瞬間に見張りが黙って穴を開ける（要件 1.5 の「足す作業は
//! 登記表に 1 行」が崩れる）。
//!
//! # なぜ窓口 crate ではなくここに住むのか
//!
//! 姉妹の見張り（`temp_path_guard_test.rs`・`with_default_guard_test.rs`）と同じ理由で、
//! この `tests/` がワークスペース全体の見張りの置き場であり、走査部品
//! `workspace_scan/mod.rs`（ファイル列挙・コメント除去・語の走査・manifest の列挙）を
//! 共有するためである。窓口 crate へ置くと走査器が複製される。
//!
//! # 「違反 0 件なら緑」への較正
//!
//! 主検査は壊れた道具でも緑になる形なので、陽性側の相棒を揃える。
//!
//! - 合成入力で 4 形すべてが当たること・正規の使い方と説明文とコメントには当たらないこと。
//! - 列挙が現実的な規模であること（母数 0 の緑を排除する）。
//! - 除外領域を走査すると確かに当たりがあり、除外を外すと違反が出ること
//!   （＝除外が飾りでない）。
//! - 本番依存の見張りは合成マニフェストで赤を作れること。
//!
//! # 除外領域の実体は今日どこまであるか（恒真化への備え）
//!
//! 設計の較正は「除外領域に ⑴〜⑷ の実体があること」を求めているが、**今日実体がある
//! のは ⑶ だけである**——⑵ と ⑷ は構造的に字面にならず、⑴ は窓口が段 ③ で旧置き場を
//! 綴らなくなって消えた。実体の無い形を黙って落とすと、除外が飾りになっても気付けない。
//! そこで [`EXCLUSION_FORMS`] が 4 形それぞれに「今日あるか・無いならなぜか」を明示し、
//! [`the_exclusion_declares_which_forms_it_holds`] が**両方向**で判定する——あると宣言した
//! 形が消えたら赤、無いと宣言した形が現れたら赤（宣言を書き換えさせる）。段 ③ で ⑶ の
//! 実体が入り ⑴ の実体が消えた瞬間に、どちらも宣言の更新を強制された。
//!
//! # 走査語を逐語で書かない約束
//!
//! 本ファイルは `crates/` の下にあるので走査対象そのものでもある。語を逐語で書くと
//! 見張りが自分自身を違反として拾う。⑴⑶ は `concat!` で 2 片に割り、⑵⑷ は登記表から
//! 組み立てるので字面には現れない。守れていることは
//! [`the_guard_file_never_spells_the_tokens_out_not_even_inside_comments`] が縛る。

mod workspace_scan;

use std::collections::BTreeSet;

use sample_ghost_kit::SAMPLES;

use workspace_scan::{
    manifest_lines, mentions_crate, production_dependencies_on, read_source, scan_tokens,
    walk_workspace_sources, workspace_manifests,
};

// ---------------------------------------------------------------------------
// 走査語（逐語で置かないため 2 片に割る／登記表から組み立てる。module doc を参照）
// ---------------------------------------------------------------------------

/// ⑴ 旧置き場のパス。
const TOKEN_OLD_PATH: &str = concat!("shiori-host-32", "/fixtures");

/// ⑴ 旧置き場のフォルダ名を継ぎ足す形。
const TOKEN_OLD_JOIN: &str = concat!("join(\"shiori-host", "-32\")");

/// ⑵ 展開形の検体が置かれている親フォルダ。これ自体は走査語ではない（語は検体名まで含む）。
const VENDOR_PARENT: &str = "vendors/sample_ghost";

/// ⑶ 段 ③ の展開先の名前空間フォルダ名。
const TOKEN_NAMESPACE: &str = concat!("nar-", "samples");

/// 窓口の**定義**が置かれた領域。走査から外す唯一の領域。
const GATEWAY_DEFINITION_PREFIX: &str = "crates/sample-ghost-kit/src/";

/// 窓口 crate のパッケージ名（本番依存の見張りが使う）。
const KIT_PACKAGE: &str = "sample-ghost-kit";

/// 本見張り自身の相対パス。
const THIS_GUARD: &str = "crates/log-capture-kit/tests/sample_path_guard_test.rs";

/// 走査語の 4 形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Form {
    /// ⑴ 旧置き場。
    OldLocation,
    /// ⑵ 展開形の検体フォルダ。
    CheckedInTree,
    /// ⑶ 展開先の名前空間。
    Namespace,
    /// ⑷ 同梱バルーン名でパスを組む形。
    BundledBalloonPath,
}

impl Form {
    /// 失敗の文言に出す名前。
    fn label(self) -> &'static str {
        match self {
            Form::OldLocation => "⑴ 旧置き場",
            Form::CheckedInTree => "⑵ 展開形の検体フォルダ",
            Form::Namespace => "⑶ 展開先の名前空間",
            Form::BundledBalloonPath => "⑷ 同梱バルーン名でのパス組み",
        }
    }

    /// この形の走査語。
    fn tokens(self) -> Vec<String> {
        match self {
            Form::OldLocation => vec![TOKEN_OLD_PATH.to_owned(), TOKEN_OLD_JOIN.to_owned()],
            Form::CheckedInTree => SAMPLES
                .iter()
                .flat_map(|s| {
                    [
                        format!("{VENDOR_PARENT}/{}/", s.name),
                        format!("{VENDOR_PARENT}/{}\"", s.name),
                    ]
                })
                .collect(),
            Form::Namespace => vec![TOKEN_NAMESPACE.to_owned()],
            Form::BundledBalloonPath => SAMPLES
                .iter()
                .flat_map(|s| {
                    s.balloons.iter().flat_map(|b| {
                        [
                            format!("join(\"{b}\")"),
                            format!("{b}/"),
                            format!("/{b}\""),
                            format!("{}(\"{b}\")", s.name),
                        ]
                    })
                })
                .collect(),
        }
    }
}

/// 全 4 形（宣言順＝設計の ⑴〜⑷ の順）。
const FORMS: [Form; 4] = [
    Form::OldLocation,
    Form::CheckedInTree,
    Form::Namespace,
    Form::BundledBalloonPath,
];

/// `(走査語, 属する形)` の全一覧。
fn token_table() -> Vec<(String, Form)> {
    FORMS
        .iter()
        .flat_map(|form| form.tokens().into_iter().map(move |t| (t, *form)))
        .collect()
}

// ---------------------------------------------------------------------------
// 除外領域に今日あるもの／まだ無いもの
// ---------------------------------------------------------------------------

/// 除外領域（窓口の定義）にその形の実体があるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    /// 今日この形の実体がある。消えたら赤（除外が飾りになった合図）。
    Present,
    /// まだ無い。名指しのタスクで入る予定。現れたら赤にして宣言を [`Presence::Present`] へ
    /// 書き換えさせる。
    #[expect(
        dead_code,
        reason = "使う形が今日は無い（⑶ は実体が入り Present へ移った）変種だが、次の形が加わるときに「いつ入るか」を書かせる仕組みはこれである。消すと強制できなくなる"
    )]
    ArrivesIn(&'static str),
    /// 構造的に字面にならない。窓口は登記表の値を**実行時に**継ぎ足すので、この形の
    /// 逐語の綴りは窓口の中にも生まれない。現れたら赤（前提が変わった合図）。
    ComposedAtRuntime(&'static str),
    /// かつて実体があったが、窓口がその綴りを持たなくなって消えた。現れたら赤
    /// （後戻りの合図）。
    ///
    /// 除外はこの形に対しては**飾り**である。飾りだと分かった形を表から落とすと、
    /// 除外が飾りになっても気付けなくなる——飾りであることを明示して残すのが
    /// [`EXCLUSION_FORMS`] の仕事である。走査語そのものは、旧置き場の綴りが窓口の外に
    /// 戻ってこないことを見張り続けるので不要にはならない。
    Retired(&'static str),
}

/// 4 形それぞれの、除外領域における実体の有無（2026-09-18 実測）。
///
/// 設計の較正は「除外領域に ⑴〜⑷ の実体があること」を求めているが、実測で実体がある
/// のは ⑶ の 1 形だけである。⑵ と ⑷ は窓口が実行時に組むので字面にならず、⑴ は窓口が
/// 段 ③ で旧置き場を綴らなくなって消えた。実体の無い形を黙って落とすと較正が恒真になる
/// ので、理由つきで宣言し、現れたら赤になるようにしてある（module doc を参照）。
const EXCLUSION_FORMS: &[(Form, Presence)] = &[
    (
        Form::OldLocation,
        Presence::Retired(
            "段 ③ で窓口は配布形（`.nar`）の置き場だけを綴るようになり、旧置き場を指す \
             登記の欄がタスク 5.4 で消えた",
        ),
    ),
    (
        Form::CheckedInTree,
        Presence::ComposedAtRuntime(
            "窓口は登記の親フォルダと検体名を実行時に継ぎ足す（段 ③ 以降の保管は \
             `<親>/<名>.nar` なので、検体名の後ろに区切りが続く形にもならない）",
        ),
    ),
    (Form::Namespace, Presence::Present),
    (
        Form::BundledBalloonPath,
        Presence::ComposedAtRuntime(
            "窓口は同梱バルーンのフォルダを登記表の名前から実行時に組む（`folder` に \
             `balloons` の要素を継ぎ足す）ので、バルーン名を含む逐語のパスは生まれない",
        ),
    ),
];

// ---------------------------------------------------------------------------
// 走査の取り回し
// ---------------------------------------------------------------------------

/// 走査で当たった 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Hit {
    /// ワークスペース根からの相対パス（区切りは `/`）。
    path: String,
    /// 1 始まりの行番号。
    line: usize,
    /// 当たった語。
    token: String,
    /// 語が属する形。
    form: Form,
}

/// 見張りの対象（窓口の定義を除いた全ソース）。
fn watched_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| !p.starts_with(GATEWAY_DEFINITION_PREFIX))
        .collect()
}

/// 窓口の定義（走査から外した領域）。
fn gateway_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| p.starts_with(GATEWAY_DEFINITION_PREFIX))
        .collect()
}

/// 与えられたファイル群を全 4 形で走査する。
fn scan_sources(paths: &[String]) -> Vec<Hit> {
    let table = token_table();
    let tokens: Vec<&str> = table.iter().map(|(t, _)| t.as_str()).collect();
    let mut hits = Vec::new();
    for path in paths {
        for (line, token) in scan_tokens(&read_source(path), &tokens) {
            let form = table
                .iter()
                .find(|(t, _)| *t == token)
                .map(|(_, f)| *f)
                .expect("当たった語は必ず一覧の中にある");
            hits.push(Hit {
                path: path.clone(),
                line,
                token,
                form,
            });
        }
    }
    hits
}

/// 合成入力を 1 本走査する（較正で使う。ファイル名は持たない）。
fn scan_text(src: &str) -> Vec<(usize, String)> {
    let table = token_table();
    let tokens: Vec<&str> = table.iter().map(|(t, _)| t.as_str()).collect();
    scan_tokens(src, &tokens)
}

/// 違反の一覧を人が読める形にする。
fn render(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| {
            format!(
                "  {}:{} （{}・語: {}）",
                h.path,
                h.line,
                h.form.label(),
                h.token
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 較正に使う `(ゴースト名, 同梱バルーン名)` の組を登記表から 1 つ選ぶ。
///
/// 名前を字面に持たないため、見本も登記表から取る。
fn a_bundled_balloon() -> (&'static str, &'static str) {
    SAMPLES
        .iter()
        .find_map(|s| s.balloons.first().map(|b| (s.name, *b)))
        .expect("登記表には同梱バルーンを持つ検体が少なくとも 1 つある")
}

// ---------------------------------------------------------------------------
// 主検査（要件 1.8）
// ---------------------------------------------------------------------------

#[test]
fn no_sample_path_spelling_lives_outside_the_gateway() {
    let violations = scan_sources(&watched_sources());
    assert!(
        violations.is_empty(),
        "検体の在処を自分で綴っている箇所がある。検体は窓口 `sample_ghost_kit::SampleRoot` \
         から名前で引くこと（`acquire` でゴースト／バルーンを、同梱バルーンは `balloon` を \
         呼ぶ）。窓口を通しておけば、段 ③ で保管が `.nar` へ変わっても呼び手は 1 行も \
         書き換えずに済む（要件 1.7・1.8）:\n{}",
        render(&violations)
    );
}

// ---------------------------------------------------------------------------
// 「0 件なら緑」への較正: 母数と除外
// ---------------------------------------------------------------------------

#[test]
fn the_scan_is_not_vacuous() {
    // 走査するファイルが 0 件なら主検査は恒真になる。
    let watched = watched_sources();
    assert!(
        watched.len() > 500,
        "列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        watched.len()
    );

    let gateway = gateway_sources();
    assert!(
        !gateway.is_empty(),
        "除外領域のファイルが 1 件も無い。窓口の置き場が動いたなら \
         GATEWAY_DEFINITION_PREFIX を直すこと: {GATEWAY_DEFINITION_PREFIX}"
    );

    assert_eq!(
        token_table().len(),
        FORMS.iter().map(|f| f.tokens().len()).sum::<usize>(),
        "走査語の一覧と形ごとの語数が食い違う"
    );
    for form in FORMS {
        assert!(
            !form.tokens().is_empty(),
            "走査語を 1 つも持たない形がある＝その形は何も検知しない: {}",
            form.label()
        );
    }
}

#[test]
fn removing_the_exclusion_turns_the_guard_red() {
    // 除外が飾りでないことの直接の証明。除外を外して全ソースを走査すると違反が出て、
    // その違反は**すべて除外領域の中**にある。
    let hits = scan_sources(&walk_workspace_sources());
    assert!(
        !hits.is_empty(),
        "除外を外しても当たりが 0 件＝走査そのものが何も検知していない"
    );
    let outside: Vec<&Hit> = hits
        .iter()
        .filter(|h| !h.path.starts_with(GATEWAY_DEFINITION_PREFIX))
        .collect();
    assert!(
        outside.is_empty(),
        "除外領域の外に当たりがある（主検査と食い違う）: {outside:?}"
    );
}

#[test]
fn the_exclusion_declares_which_forms_it_holds() {
    // 設計の較正「除外領域に ⑴〜⑷ の実体があること」を、段 ① の実測に合わせて
    // **両方向**の判定に組み替えたもの（module doc を参照）。
    let declared: Vec<Form> = EXCLUSION_FORMS.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        declared,
        FORMS.to_vec(),
        "宣言は 4 形すべてを設計の順に 1 度ずつ並べること（漏れた形は判定されない）"
    );

    let hits = scan_sources(&gateway_sources());
    let found: BTreeSet<Form> = hits.iter().map(|h| h.form).collect();

    let mut present = 0usize;
    for (form, presence) in EXCLUSION_FORMS {
        match presence {
            Presence::Present => {
                present += 1;
                assert!(
                    found.contains(form),
                    "{} の実体が除外領域から消えた。除外がこの形に対して飾りになっている。\
                     窓口が別の綴り方へ移ったのなら走査語を直し、形ごと不要になったのなら \
                     EXCLUSION_FORMS の宣言を書き換えること",
                    form.label()
                );
            }
            Presence::ArrivesIn(task) => {
                assert!(
                    !task.trim().is_empty(),
                    "いつ入るかを書かない宣言は追跡できない: {}",
                    form.label()
                );
                assert!(
                    !found.contains(form),
                    "{} の実体が除外領域に現れた（予定タスク: {task}）。\
                     EXCLUSION_FORMS の宣言を Presence::Present へ書き換えること",
                    form.label()
                );
            }
            Presence::ComposedAtRuntime(why) => {
                assert!(
                    !why.trim().is_empty(),
                    "理由を書かない宣言は検証できない: {}",
                    form.label()
                );
                assert!(
                    !found.contains(form),
                    "{} は「窓口が実行時に組み立てるので字面にならない」と宣言しているのに \
                     実体が現れた。前提が変わっている（理由: {why}）",
                    form.label()
                );
            }
            Presence::Retired(why) => {
                assert!(
                    !why.trim().is_empty(),
                    "理由を書かない宣言は検証できない: {}",
                    form.label()
                );
                assert!(
                    !found.contains(form),
                    "{} は窓口から消えたと宣言しているのに実体が現れた（理由: {why}）。\
                     窓口が旧い綴りへ後戻りしていないか確かめること",
                    form.label()
                );
            }
        }
    }

    assert!(
        present > 0,
        "除外領域にどの形の実体も無いと宣言している＝除外は飾りである。\
         除外そのものを外すか、走査語を見直すこと"
    );
}

// ---------------------------------------------------------------------------
// 較正: 合成入力で 4 形すべてが当たる
// ---------------------------------------------------------------------------

#[test]
fn the_old_location_is_detected() {
    let path_form =
        format!("    let dir = repo.join(\"crates/pilot/examples/{TOKEN_OLD_PATH}/emo2\");\n");
    assert_eq!(
        scan_text(&path_form),
        vec![(1usize, TOKEN_OLD_PATH.to_owned())],
        "旧置き場のパスを検知できていない"
    );

    let join_form = format!("    let dir = examples.{TOKEN_OLD_JOIN}.join(\"fixtures\");\n");
    assert_eq!(
        scan_text(&join_form),
        vec![(1usize, TOKEN_OLD_JOIN.to_owned())],
        "旧置き場のフォルダ名を継ぎ足す形を検知できていない"
    );

    // 近接の陰性: 名前がたまたま同じ綴りで始まる別のフォルダには当たらない。
    let near_miss = "    let dir = examples.join(\"shiori-host-32-helper\").join(\"fixtures\");\n";
    assert_eq!(
        scan_text(near_miss),
        Vec::new(),
        "名前の続きが違うフォルダに当たっている"
    );
}

#[test]
fn the_checked_in_sample_tree_is_detected_but_the_nar_file_name_is_not() {
    // 走査語は登記された**全ての**検体名について組まれる（[`Form::tokens`]）ので、
    // 陽性の確認はどの 1 つでも同じ。登記の先頭を採る。
    let name = SAMPLES
        .first()
        .map(|s| s.name)
        .expect("検体が 1 つ以上登記されている");

    let token = format!("{VENDOR_PARENT}/{name}/");
    let positive = format!("    let dic = repo.join(\"{token}dic\");\n");
    assert_eq!(
        scan_text(&positive),
        vec![(1usize, token.clone())],
        "展開形の検体フォルダを検知できていない"
    );

    // 末尾 `/` を付けずにフォルダで閉じる綴りも同じ形である。
    let bare = format!("{VENDOR_PARENT}/{name}\"");
    let positive = format!(
        "    let dir = repo.join(\"{bare});
"
    );
    assert_eq!(
        scan_text(&positive),
        vec![(1usize, bare)],
        "末尾 `/` の無い展開形の検体フォルダを検知できていない"
    );

    // `.nar` の保管場所を名指しするのは禁じていない（要件 1.8）。
    let nar = format!("    let nar = repo.join(\"{VENDOR_PARENT}/{name}.nar\");\n");
    assert_eq!(
        scan_text(&nar),
        Vec::new(),
        "`.nar` のファイル名に当たっている（保管場所の名指しは禁じていない）"
    );
}

#[test]
fn the_extraction_namespace_is_detected() {
    let positive = format!("    let root = target.join(\"{TOKEN_NAMESPACE}\").join(name);\n");
    assert_eq!(
        scan_text(&positive),
        vec![(1usize, TOKEN_NAMESPACE.to_owned())],
        "展開先の名前空間を検知できていない"
    );

    // 近接の陰性: 絶対パスを得るコマンド名（要件 1.9）には当たらない。
    let near_miss = "    let cmd = \"nar-sample-path\";\n";
    assert_eq!(
        scan_text(near_miss),
        Vec::new(),
        "名前空間の語がコマンド名に当たっている"
    );
}

#[test]
fn all_four_shapes_of_a_bundled_balloon_path_are_detected() {
    let (ghost, balloon) = a_bundled_balloon();
    for token in [
        format!("join(\"{balloon}\")"),
        format!("{balloon}/"),
        format!("/{balloon}\""),
        format!("{ghost}(\"{balloon}\")"),
    ] {
        let line = format!("    let dir = ghost_root.{token}.to_path_buf();\n");
        assert_eq!(
            scan_text(&line),
            vec![(1usize, token.clone())],
            "同梱バルーン名でパスを組む形を検知できていない: {token}"
        );
    }

    // 区切りから始まる語は、直前が識別子文字でも当たらねばならない
    //（走査部品の左端アンカーは識別子で始まる語にだけ効く）。
    let inside_a_literal = format!("    let dir = repo.join(\"ghost/emo2/{balloon}\");\n");
    assert_eq!(
        scan_text(&inside_a_literal),
        vec![(1usize, format!("/{balloon}\""))],
        "パス文字列の末尾に名前を継ぎ足した形を検知できていない"
    );
}

#[test]
fn the_window_reader_arguments_and_prose_are_not_hits() {
    let (_, balloon) = a_bundled_balloon();

    // 窓口の読み口へ名前を引数で渡すのは要件 1.3 の正規の使い方。
    let reader = format!("    let dir = emo2.balloon(\"{balloon}\").expect(\"同梱バルーン\");\n");
    assert_eq!(scan_text(&reader), Vec::new(), "窓口の読み口に当たっている");

    // 検体そのものを名前で取得する形（同梱バルーンの派生は単体の検体として登記されている）。
    let derived = SAMPLES
        .iter()
        .map(|s| s.name)
        .find(|n| n.starts_with(balloon) && *n != balloon)
        .expect("同梱バルーンの派生が単体の検体として登記されている");
    let acquire =
        format!("    let wplimit = SampleRoot::acquire(\"{derived}\").expect(\"登記済み\");\n");
    assert_eq!(
        scan_text(&acquire),
        Vec::new(),
        "検体名を引数で渡す形に当たっている（1.3 の正規の使い方）"
    );

    // 説明文の中の名前。
    let prose = format!("    let note = \"{balloon} の font.height,28\";\n");
    assert_eq!(
        scan_text(&prose),
        Vec::new(),
        "説明文の中の名前に当たっている"
    );
}

#[test]
fn a_spelling_in_a_comment_is_not_a_hit_but_the_same_line_of_code_is() {
    let body = format!("let dir = repo.join(\"crates/pilot/examples/{TOKEN_OLD_PATH}/emo2\");");

    let commented = format!("    // かつては {body} と書いていた\n");
    assert_eq!(
        scan_text(&commented),
        Vec::new(),
        "コメント中の綴りを拾っている（走査は必ずコメント除去を通すこと）"
    );

    let block_commented = format!("    /* 旧実装:\n       {body}\n    */\n");
    assert_eq!(
        scan_text(&block_commented),
        Vec::new(),
        "塊コメント中の綴りを拾っている"
    );

    let executable = format!("    {body}\n");
    assert_eq!(
        scan_text(&executable),
        vec![(1usize, TOKEN_OLD_PATH.to_owned())],
        "実行行で当たらない＝走査そのものが空振りで、上の 0 件は何も意味しない"
    );

    let trailing = format!("    {body} // 旧置き場\n");
    assert_eq!(
        scan_text(&trailing),
        vec![(1usize, TOKEN_OLD_PATH.to_owned())],
        "行末コメントの除去が行の実行部分まで巻き込んでいる"
    );
}

// ---------------------------------------------------------------------------
// 本番依存の見張り（要件 1.7・10.3）
// ---------------------------------------------------------------------------

/// 開発側の依存として窓口 crate を引いているか（純関数・較正の陽性側）。
fn has_dev_kit_dependency(src: &str) -> bool {
    manifest_lines(src).iter().any(|l| {
        l.section.contains("dev-dependencies")
            && if l.is_header {
                mentions_crate(&l.section, KIT_PACKAGE)
            } else {
                mentions_crate(&l.text, KIT_PACKAGE)
            }
    })
}

#[test]
fn the_sample_gateway_never_appears_in_a_production_dependency_table() {
    let mut violations: Vec<String> = Vec::new();
    for (name, text) in workspace_manifests(KIT_PACKAGE) {
        for line in production_dependencies_on(&text, KIT_PACKAGE) {
            violations.push(format!(
                "  crates/{name}/Cargo.toml:{} [{}] {}",
                line.line, line.section, line.text
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "窓口 crate はテスト専用で、製品側の依存に現れてはいけない。本番コードが検体の在処を \
         知らないことは約束ではなく依存の向きで保証している（要件 1.7・10.3）:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_sample_gateway_is_actually_pulled_in_as_a_dev_dependency() {
    // 較正: 上の検査が「manifest を 1 つも読めていない」「名前の照合が壊れている」で
    // 空になっていないことを示す。
    let manifests = workspace_manifests(KIT_PACKAGE);
    assert!(
        manifests.len() > 15,
        "manifest の列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        manifests.len()
    );
    let dev: Vec<&str> = manifests
        .iter()
        .filter(|(_, text)| has_dev_kit_dependency(text))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        dev.len() > 5,
        "窓口 crate を dev-dependency として引いている crate が少なすぎる＝名前の照合が \
         壊れている疑い: {dev:?}"
    );
}

/// manifest の見本。依存行だけを差し替えて両側を作る（依存行は必ず 6 行目）。
fn manifest_fixture(section: &str, line: &str) -> String {
    format!("[package]\nname = \"demo\"\n\n[{section}]\ntracing = {{ workspace = true }}\n{line}\n")
}

/// 見本の依存行（窓口 crate を引く素の形）。
const KIT_DEP_LINE: &str = "sample-ghost-kit = { path = \"../sample-ghost-kit\" }";

#[test]
fn a_production_dependency_on_the_sample_gateway_is_a_violation() {
    for section in [
        "dependencies",
        "build-dependencies",
        "target.'cfg(windows)'.dependencies",
    ] {
        let src = manifest_fixture(section, KIT_DEP_LINE);
        let found = production_dependencies_on(&src, KIT_PACKAGE);
        assert_eq!(
            found.len(),
            1,
            "[{section}] の窓口 crate 依存は違反として出るはず: {found:?}"
        );
        assert_eq!(found[0].line, 6, "行番号は元の manifest のもの");
        assert_eq!(found[0].section, section);
    }

    let sub_table = "[package]\nname = \"demo\"\n\n[dependencies.sample-ghost-kit]\npath = \"../sample-ghost-kit\"\n";
    let found = production_dependencies_on(sub_table, KIT_PACKAGE);
    assert_eq!(found.len(), 1, "下位表の形も拾うはず: {found:?}");
    assert!(found[0].is_header, "見出し行 1 件として報告されるはず");
}

#[test]
fn a_dev_dependency_on_the_sample_gateway_is_not_a_violation() {
    for section in ["dev-dependencies", "target.'cfg(windows)'.dev-dependencies"] {
        let src = manifest_fixture(section, KIT_DEP_LINE);
        assert_eq!(
            production_dependencies_on(&src, KIT_PACKAGE),
            Vec::new(),
            "[{section}] は開発側なので違反ではない"
        );
        assert!(
            has_dev_kit_dependency(&src),
            "[{section}] は開発側の依存として検出されるはず"
        );
    }
}

// ---------------------------------------------------------------------------
// 見張り自身が走査語を逐語で持たないこと
// ---------------------------------------------------------------------------

#[test]
fn the_guard_file_itself_is_not_a_hit() {
    assert_eq!(
        scan_sources(&[THIS_GUARD.to_owned()]),
        Vec::new(),
        "見張り自身が走査語を逐語で持ってしまっている（concat! で割るか登記表から組むこと）"
    );
}

/// コメントを**除かずに**生テキストから走査語を探す（純関数）。
fn raw_occurrences(src: &str, tokens: &[String]) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    for (index, line) in src.lines().enumerate() {
        for token in tokens {
            if line.contains(token.as_str()) {
                hits.push((index + 1, token.clone()));
            }
        }
    }
    hits
}

#[test]
fn the_guard_file_never_spells_the_tokens_out_not_even_inside_comments() {
    let tokens: Vec<String> = token_table().into_iter().map(|(t, _)| t).collect();

    // 較正: 生テキスト走査はコメントを除かない。除いてしまうと本検査は恒真になる。
    let planted = format!("// 説明: かつては {TOKEN_NAMESPACE} と書いていた");
    assert_eq!(
        raw_occurrences(&planted, &tokens),
        vec![(1usize, TOKEN_NAMESPACE.to_owned())],
        "コメントの中でも生テキスト走査は当たらねばならない（当たらないなら本検査は恒真）"
    );

    let violations: Vec<String> = raw_occurrences(&read_source(THIS_GUARD), &tokens)
        .into_iter()
        .map(|(line, token)| format!("  {THIS_GUARD}:{line} （語: {token}）"))
        .collect();
    assert!(
        violations.is_empty(),
        "走査語はコメントの中も含めて逐語で書いてはいけない（見張りが自分自身を拾う）。\
         concat! で 2 片に割るか、登記表から組み立てること:\n{}",
        violations.join("\n")
    );
}
