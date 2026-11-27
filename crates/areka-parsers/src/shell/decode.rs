//! 意味層（Decode）— 構文トークン → 値正規化済み `Shell`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、シェルサーフェス
//! モデル `Shell` を組み立てる（依存方向 `model ← lexer ← decode`）。下流が再パース
//! 不要なのは、この層が値を decode しきるため。sakura 同様に `Result` を
//! 返さず・パニックせず・空入力で空を返す寛容決定的パスを採る。
//!
//! 入口は `decode_token`（`parse_all` が字句の列を登場順に 1 つずつ渡す）。読む途中の状態
//! （範囲の幅の合計・今の見出し・積んだ崩れ）は `Reading` が持つ。
//! - 閉じたブレス（`Token::Brace`）は見出しで振り分ける。優先順位は
//!   `descript` → `kero.surface.alias` → `surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
//!   alias／append は "surface" を含むので、`surfaceNNN` より**先**に判定する。
//! - descriptブレスとトップレベル行（`Token::TopLevel`）からは `animation-sort`／`collision-sort`
//!   だけを読む。charset・version など他の行は保持しない。
//! - `surfaceNNN`・`surface.append*` は見出しの記述子（`N,M`・`N-M`・`!N`）を `parse_targets` で
//!   展開せずに捕捉し、本体の element（`overlay`・`base`）・collision・animation を共有の
//!   `decode_elements`・`decode_collisions`・`decode_animations` で読む。
//! - `kero.surface.alias` は `KEY,[id,...]` を不透明キー＋順序付き ID へ写す（重複も保持）。
//! - 読めない番号は 0 にせず飛ばして崩れ（`ShellIssue`）を積む。範囲の幅は
//!   シェル全体の合計に上限 `RANGE_WIDTH_CAP` をかける。崩れの知らせ
//!   （`Token::Unclosed`／`Token::Reopened`）は `parse_all` が写すので、ここでは読まない。
//! - `collisionex*` は `collision*` と同じ並びへ書いた順に積み、読めない行は理由つきの崩れを積む。
//!   楕円・円・多角形・region も値にして並びへ入れる（崩れの記録は出さない）。
//! - それ以外のメソッド・interval の語・未知のブレスや行は、値にしないか原文のまま
//!   運び、走査を中断しない。

use std::collections::HashMap;

use super::element_method_words::is_image_element_method;
use super::lexer::{Token, heading_text};
use super::model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionBounds, CollisionExFault, CollisionName,
    CollisionRegion, CollisionShape, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern,
    RANGE_WIDTH_CAP, Shell, ShellIssue, SortOrder, Surface, SurfaceAlias, SurfaceAppend,
};

/// 1 つのシェルを読む途中の状態: シェル全体の範囲の幅の合計・今のブレスの見出し・積んだ崩れ。
#[derive(Default)]
pub(crate) struct Reading {
    range_width: u64,
    heading: String,
    pub(crate) issues: Vec<ShellIssue>,
}

impl Reading {
    /// ブレスの見出しを崩れの名指しに使う原文として控える。
    pub(crate) fn begin(&mut self, header: &[String]) {
        self.heading = heading_text(header);
    }

    fn unreadable(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues
            .push(ShellIssue::UnreadableNumber { heading, text });
    }

    fn too_wide(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues.push(ShellIssue::RangeTooWide { heading, text });
    }

    fn wait_range(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues.push(ShellIssue::WaitRange { heading, text });
    }

    fn collision_ex(&mut self, fields: &[String], fault: CollisionExFault) {
        let heading = self.heading.clone();
        let text = fields.join(",");
        self.issues.push(ShellIssue::CollisionEx {
            heading,
            text,
            fault,
        });
    }
}

/// 空の `Shell`（読み始め）。
pub(crate) fn empty_shell() -> Shell {
    Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions: Vec::new(),
    }
}

/// 構文トークン列を値正規化済みの `Shell` へ写像する（テストの口・崩れは捨てる）。
#[cfg(test)]
pub(crate) fn decode(tokens: Vec<Token>) -> Shell {
    let mut shell = empty_shell();
    let mut reading = Reading::default();
    for token in &tokens {
        decode_token(&mut shell, token, &mut reading);
    }
    shell
}

/// 構文トークン 1 つを `shell` へ写す（`parse_all` が登場順に呼ぶ）。
///
/// ブロックヘッダで振り分ける。charset は読み飛ばし保持しない。
/// 失敗しない（`Result` でない）。飛ばしたものは `reading` に崩れを積む。
pub(crate) fn decode_token(shell: &mut Shell, token: &Token, reading: &mut Reading) {
    match token {
        Token::Brace { header, body } => {
            reading.begin(header);
            dispatch_block(shell, header, body, reading);
        }
        // `animation-sort`/`collision-sort` 以外（charset 行など）は保持しない。
        Token::TopLevel(fields) => decode_sort_key(shell, fields),
        // 崩れの知らせは `parse_all` が `ShellIssue` へ写す（ここでは読まない）。
        Token::Unclosed { .. } | Token::Reopened { .. } | Token::Stray { .. } => {}
    }
}

/// ブレスの見出し（CSV フィールド列）でブロック種別を判定し振り分ける。
///
/// 優先順位は `descript` → `kero.surface.alias` → `surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
/// alias/append は "surface" を含むため plain `surfaceNNN` より先に判定する。
fn dispatch_block(
    shell: &mut Shell,
    header: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) {
    let head = header.first().map(String::as_str).unwrap_or("");

    if head == "descript" {
        // 後の行が勝つ。ほかの行（version 等）は保持しない。
        for fields in body {
            decode_sort_key(shell, fields);
        }
        return;
    }
    if head == "kero.surface.alias" {
        decode_alias_block(shell, body, reading);
        return;
    }
    if head == "surface.alias" {
        // `sakura.surface.alias` と同じく読まない（`surface` の番号の無い surface を作らない）。
        return;
    }
    if head.starts_with("surface.append") {
        decode_append_block(shell, header, body, reading);
        return;
    }
    if let Some(header_number) = head.strip_prefix("surface") {
        let rest = header.get(1..).unwrap_or(&[]);
        let targets = parse_targets(header_number, rest, reading);
        if targets.is_empty() {
            // 記述子が 1 つも残らないブレスは積まない。
            return;
        }
        shell
            .definitions
            .push(DefRef::Surface(shell.surfaces.len()));
        // overlay・base 以外の element メソッド等は値にしない。
        shell.surfaces.push(Surface {
            targets,
            elements: decode_elements(body, reading),
            collisions: decode_collisions(body, reading),
            animations: decode_animations(body, reading),
        });
    }

    // 未知ブロック（未対応ヘッダ）は何も積まない。
}

/// 本体行群の `elementN,<描画メソッド>,PATH,X,Y` を `Element` へ decode する。
///
/// - 第 2 欄が [`is_image_element_method`] で真の行だけを扱う。偽の語の行は値化せず読み飛ばす。
/// - 画像パス（field[2]）は無加工保持（区切り正規化なし）。
/// - element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持）。
fn decode_elements(body: &[Vec<String>], reading: &mut Reading) -> Vec<Element> {
    let mut elements: Vec<Element> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        if let Some(rest) = key.strip_prefix("element")
            && is_image_element_method(fields.get(1).map(String::as_str).unwrap_or(""))
        {
            let Ok(layer) = rest.parse::<u32>() else {
                reading.unreadable(key);
                continue;
            };
            elements.push(Element {
                layer,
                path: ElementPath::new(field_string(fields, 2)),
                x: field_i64(fields, 3),
                y: field_i64(fields, 4),
            });
        }
    }

    elements.sort_by_key(|e| e.layer);

    elements
}

/// 箱の転記（`boxes.rs`）が拾う element定義の描画メソッドの語か（完全一致・`balloon`）。
/// 描けない行の転記（`undrawn.rs`）は、これと [`is_image_element_method`] のどちらも偽の行を拾う。
pub(super) fn is_box_element_method(word: &str) -> bool {
    word == "balloon"
}

/// 本体行群の `collisionN,始点X,始点Y,終点X,終点Y,ID` と `collisionexN,ID,タイプ,座標…` を
/// 書いた順に 1 つの `Collision` の並びへ decode する。
///
/// - `collision*` は ukadoc 順序 始点X/始点Y/終点X/終点Y ＝ left/top/right/bottom の矩形。
///   欠落・非数値は既定 0（パニックしない）。
/// - `collisionex*` は `collision` より先に見分け、`decode_collisionex_row` で読む。使えない行は
///   理由つきの崩れを積んで飛ばし、欄が多い行だけは前から使って崩れを積む。
/// - どちらも番号は数字だけの N。番号の無い・桁あふれは飛ばして崩れを積み、数字でない残りは黙って飛ばす。
fn decode_collisions(body: &[Vec<String>], reading: &mut Reading) -> Vec<Collision> {
    let mut collisions: Vec<Collision> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");
        if let Some(rest) = key.strip_prefix("collisionex") {
            if !rest.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Ok(index) = rest.parse::<u32>() else {
                reading.unreadable(&fields.join(","));
                continue;
            };
            match decode_collisionex_row(index, fields) {
                Ok((collision, fault)) => {
                    if let Some(fault) = fault {
                        reading.collision_ex(fields, fault);
                    }
                    collisions.push(collision);
                }
                Err(fault) => reading.collision_ex(fields, fault),
            }
        } else if let Some(rest) = key.strip_prefix("collision") {
            if rest.chars().all(|c| c.is_ascii_digit()) {
                // 番号の無い・桁あふれの番号は飛ばす。
                let Ok(index) = rest.parse::<u32>() else {
                    reading.unreadable(key);
                    continue;
                };
                collisions.push(Collision {
                    index,
                    shape: CollisionShape::Rect(CollisionBounds {
                        left: field_i64(fields, 1),
                        top: field_i64(fields, 2),
                        right: field_i64(fields, 3),
                        bottom: field_i64(fields, 4),
                    }),
                    name: CollisionName::new(field_string(fields, 5)),
                });
            }
        }
    }

    collisions
}

/// collisionex の 1 行（番号は読めた後）を値にする。`Err` の行は並びに入れない。
/// `Ok((c, Some(ExtraFields)))` は「前から求める数だけを使ったうえで記録する」行。
///
/// 欄は 0＝`collisionexN`・1＝名前（書いたまま）・2＝形（大小文字を区別しない）・3 以降＝座標か region の欄。
/// 行末の空の欄 1 つは数える前に除く。
/// 1 行に複数の理由が当てはまっても理由は 1 つ。ファイルは開かない。
fn decode_collisionex_row(
    index: u32,
    fields: &[String],
) -> Result<(Collision, Option<CollisionExFault>), CollisionExFault> {
    let fields = match fields.split_last() {
        Some((last, head)) if last.is_empty() && !head.is_empty() => head,
        _ => fields,
    };
    let args = fields.get(3..).unwrap_or(&[]);
    let shape_name = field_string(fields, 2).to_ascii_lowercase();
    let (shape, used) = match shape_name.as_str() {
        "rect" | "ellipse" | "circle" | "polygon" => {
            let need = match shape_name.as_str() {
                "circle" => 3,
                "polygon" => 6,
                _ => 4,
            };
            if args.len() < need {
                return Err(CollisionExFault::MissingCoordinates);
            }
            // 多角形は全部の欄を使うので「欄が多い」は出ない。
            let used = if shape_name == "polygon" {
                if args.len() % 2 == 1 {
                    return Err(CollisionExFault::OddCoordinates);
                }
                args.len()
            } else {
                need
            };
            let v = args[..used]
                .iter()
                .map(|f| f.parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| CollisionExFault::NotInteger)?;
            let bounds = || CollisionBounds {
                left: v[0],
                top: v[1],
                right: v[2],
                bottom: v[3],
            };
            let shape = match shape_name.as_str() {
                "rect" => CollisionShape::Rect(bounds()),
                "ellipse" => CollisionShape::Ellipse(bounds()),
                "circle" => CollisionShape::Circle {
                    x: v[0],
                    y: v[1],
                    radius: v[2],
                },
                _ => CollisionShape::Polygon(v.chunks(2).map(|p| (p[0], p[1])).collect()),
            };
            (shape, used)
        }
        "region" => {
            let color = |i: usize| args.get(i).and_then(|f| f.parse::<u8>().ok());
            let invert = match args.get(4) {
                None => Some(false),
                Some(f) if f.eq_ignore_ascii_case("true") => Some(true),
                Some(f) if f.eq_ignore_ascii_case("false") => Some(false),
                Some(_) => None,
            };
            let file = field_string(args, 0);
            let (false, Some(red), Some(green), Some(blue), Some(invert)) =
                (file.is_empty(), color(1), color(2), color(3), invert)
            else {
                return Err(CollisionExFault::BadRegion);
            };
            let region = CollisionRegion {
                file,
                red,
                green,
                blue,
                invert,
                x: 0,
                y: 0,
            };
            (CollisionShape::Region(region), 5)
        }
        _ => return Err(CollisionExFault::UnknownShape),
    };
    let extra = (args.len() > used).then_some(CollisionExFault::ExtraFields);
    let name = CollisionName::new(field_string(fields, 1));
    Ok((Collision { index, shape, name }, extra))
}

/// 本体行の `idx` フィールドを無加工の `String` として取り出す（欠落は空文字列）。
pub(super) fn field_string(fields: &[String], idx: usize) -> String {
    fields.get(idx).cloned().unwrap_or_default()
}

/// 本体行の `idx` フィールドを `i64` として取り出す（欠落・非数値は既定 0）。
fn field_i64(fields: &[String], idx: usize) -> i64 {
    fields
        .get(idx)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// pattern定義のウェイトの欄を読む。`最小-最大` で両側が数なら小さい方を待ちにして
/// `WaitRange` を積む（ランダムに選ぶのは未対応）。それ以外は数でなければ 0。
fn pattern_wait(fields: &[String], idx: usize, reading: &mut Reading) -> u32 {
    let text = fields.get(idx).map_or("", |s| s.trim());
    if let Some((low, high)) = text.split_once('-')
        && let (Ok(low), Ok(high)) = (low.trim().parse::<u32>(), high.trim().parse::<u32>())
    {
        reading.wait_range(text);
        return low.min(high);
    }
    text.parse::<u32>().unwrap_or(0)
}

/// 本体行の `idx` フィールドを `u32` として取り出す（欠落・非数値は既定 0）。
fn field_u32(fields: &[String], idx: usize) -> u32 {
    fields
        .get(idx)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

/// 本体行群の `animationN.interval` / `animationN.patternM` を animation ID で集約する。
///
/// 集約規則:
/// - **ドット分割＋ ID 集約**: field[0] を最初の `.` で分割し、接頭辞
///   `animationN` から N を、接尾辞から `interval` / `patternM` を得る。同一 N の行は
///   1 個の `Animation` へ束ね、animation は N の**初出順**で保持する。
/// - **interval**: `interval,KIND[,K]` の KIND を
///   `bind` → `Bind`、`random` → `Random{k}`、`bind+random` → `BindRandom{k}` に正規化し、
///   未認識キーワード（`sometimes`/`rarely`/`always` 等）は原文のまま
///   `Interval::Other(keyword)` へ転記する。K は field[2] を u32・欠落既定 0。
///   interval のみで pattern が無くても animation を積み、pattern が interval 行より先に
///   現れても同一 ID へ束ねる。
/// - **既定 interval**: ある ID に pattern はあるが interval 行が現れない場合、
///   既定 `Interval::Bind` で `Animation` を積む。
/// - **pattern**: `patternM,描画メソッド,ID,WAIT,X,Y` の M を**そのまま**保持し、
///   欠番を合成しない。field[1] は描画メソッドとして原文のまま運ぶ（意味解釈は下流）。
///   `surface_id` は field[2] を **i64** で保持し、負値（`-1`/`-2`）をセンチネルとして失わない。
///
/// 失敗しない（`Vec` を返す・パニックしない・空/崩れ入力に寛容）。
fn decode_animations(body: &[Vec<String>], reading: &mut Reading) -> Vec<Animation> {
    let mut animations: Vec<Animation> = Vec::new();
    let mut slots: HashMap<u32, usize> = HashMap::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        let rest = match key.strip_prefix("animation") {
            Some(rest) => rest,
            None => continue,
        };
        // `.` が無い崩れキー（例 `animationX`）は読み飛ばす。
        let (id_text, suffix) = match rest.split_once('.') {
            Some(pair) => pair,
            None => continue,
        };
        let Ok(id) = id_text.parse::<u32>() else {
            reading.unreadable(key);
            continue;
        };

        if suffix == "interval" {
            let interval = normalize_interval(fields);
            let anim = animation_slot(&mut animations, &mut slots, id);
            anim.interval = interval;
            continue;
        }

        if let Some(index_text) = suffix.strip_prefix("pattern") {
            // 非 overlay（replace/base/制御系 等）も落とさず method を運ぶ（意味解釈は下流）。
            // 欄 2 が `(` の並びなら `)` まで継ぎ直し、待ち・X・Y はその次から読む。
            let Ok(index) = index_text.parse::<u32>() else {
                reading.unreadable(key);
                continue;
            };
            let (target, tail) = pattern_target(fields);
            let pattern = Pattern {
                index,
                // 欠落は空文字（下流で `Unknown` になる）。
                method: DrawMethod::new(field_string(fields, 1)),
                surface_id: field_i64(fields, 2),
                target,
                wait: pattern_wait(fields, tail, reading),
                x: field_i64(fields, tail + 1),
                y: field_i64(fields, tail + 2),
            };
            let anim = animation_slot(&mut animations, &mut slots, id);
            anim.patterns.push(pattern);
            continue;
        }
    }

    animations
}

/// pattern定義の欄 2 の原文と、待ちを読む欄の位置を返す。
///
/// 字句の段は `[ ]` の中のカンマしか守らないので、欄 2 が `(` で始まり `)` を含まないときだけ、
/// `)` を含む欄（無ければ行の終わり）までを `,` で継ぎ直す。語の意味は知らない（原文を写すだけ）。
fn pattern_target(fields: &[String]) -> (String, usize) {
    let col = field_string(fields, 2);
    if !col.starts_with('(') || col.contains(')') {
        return (col, 3);
    }
    let end = fields[2..]
        .iter()
        .position(|f| f.contains(')'))
        .map_or(fields.len(), |p| 2 + p + 1);
    (fields[2..end].join(","), end)
}

/// `animations` 内の ID `id` の `Animation` への可変参照を返す。無ければ既定 `Interval::Bind`
/// ＋空 pattern の枠を初出順で末尾に積んでから返す（interval が pattern より後に来ても
/// 同一 ID へ束ねられるようにする）。`slots` は番号 → `animations` の位置の表。
fn animation_slot<'a>(
    animations: &'a mut Vec<Animation>,
    slots: &mut HashMap<u32, usize>,
    id: u32,
) -> &'a mut Animation {
    let pos = *slots.entry(id).or_insert_with(|| {
        animations.push(Animation {
            id,
            interval: Interval::Bind,
            patterns: Vec::new(),
        });
        animations.len() - 1
    });
    &mut animations[pos]
}

/// `animationN.interval,KIND[,K]` 行を `Interval` へ忠実転記する。
///
/// KIND（field[1]）が bind/random/bind+random ならその語彙へ、それ以外の
/// 未認識キーワード（sometimes/rarely/always 等）は原文のまま
/// `Interval::Other(keyword)` へ転記する。KIND 欠落は空文字 `Other("")`（下流吸収）。
/// K（field[2]）は u32・欠落既定 0。
///
/// 引き金の 3 語は小文字の完全一致だけを読む: `runonce` → `Runonce`（第 3 欄以降は読まない）、`talk`／`periodic` は field[2] が
/// 1 以上の整数なら `Talk{n}`／`Periodic{secs}`、読めないとき（欠落・0・非数値）は第 2 欄以降を
/// `,` で繋いだ原文を `Other` へ運ぶ（失敗せず記録も出さない・無効の記録は下流の表が出す）。
fn normalize_interval(fields: &[String]) -> Interval {
    match fields.get(1).map(String::as_str) {
        Some("bind") => Interval::Bind,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#random_2c_6570_5024:1
        Some("random") => Interval::Random {
            k: field_u32(fields, 2),
        },
        Some("bind+random") => Interval::BindRandom {
            k: field_u32(fields, 2),
        },
        Some("runonce") => Interval::Runonce,
        // `field_u32` は欠落・非数値を 0 へ倒すので、0 が「正の整数として読めない」の全部。
        Some(word @ ("talk" | "periodic")) => match (word, field_u32(fields, 2)) {
            (_, 0) => Interval::Other(fields[1..].join(",").into()),
            ("talk", n) => Interval::Talk { n },
            (_, secs) => Interval::Periodic { secs },
        },
        other => Interval::Other(other.unwrap_or("").into()),
    }
}

/// `surface.append*` 追記ブロックを `SurfaceAppend` へ decode し `shell.appends` へ積む。
///
/// - ターゲット指定はヘッダから `parse_targets` で記述子リストとして捕捉する（展開しない）。
/// - element・collision・animation は通常 surface と同じ読み手で読む。
/// - append は出現順で `shell.appends` に保持する。
/// - 失敗しない（`Result` でない・パニックしない）。
fn decode_append_block(
    shell: &mut Shell,
    header: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) {
    let head = header.first().map(String::as_str).unwrap_or("");
    let header_number = head.strip_prefix("surface.append").unwrap_or("");
    let rest = header.get(1..).unwrap_or(&[]);
    let targets = parse_targets(header_number, rest, reading);
    if targets.is_empty() {
        return;
    }

    shell.definitions.push(DefRef::Append(shell.appends.len()));
    shell.appends.push(SurfaceAppend {
        targets,
        elements: decode_elements(body, reading),
        collisions: decode_collisions(body, reading),
        animations: decode_animations(body, reading),
    });
}

/// append ターゲット指定を記述子リスト `Vec<AppendTarget>` へ捕捉する（展開しない）。
///
/// ヘッダ数値 `header` を **第1要素**とし、後続 `rest`（列挙・範囲）を続けた順序付きリストを返す。
/// ヘッダ数値は列挙された単一 ID と同格に扱い、カテゴリ番号的な特別扱いはしない。範囲 `a-b` は
/// `Range` 記述子として保持し、個別 ID へ展開しない（展開・実 surface ツリーへの転記は下流）。
///
/// 入力例: `header="10"`, `rest=["2100-2110", "2200-2210"]`
///   → `[Single(10), Range{2100,2110}, Range{2200,2210}]`。
/// 入力例: `header="2200"`, `rest=[]` → `[Single(2200)]`。
///
/// 読めない記述子と、範囲の幅の合計が上限を越える記述子は飛ばし、`reading` に崩れを積む。
/// 箱の転記も同じ関数を通るので同じ上限がかかる。
pub(super) fn parse_targets(
    header: &str,
    rest: &[String],
    reading: &mut Reading,
) -> Vec<AppendTarget> {
    std::iter::once(header)
        .chain(rest.iter().map(String::as_str))
        .filter_map(|field| parse_target_element(field, reading))
        .collect()
}

/// 単一ターゲットフィールドを `AppendTarget` へ写像する（ヘッダ数値・列挙要素で共用）。
///
/// - 先頭 `!` は除外指定（`!N`→`Exclude`・`!a-b`→`ExcludeRange`）。記述子のまま保持し減算しない。
/// - `-` を含むフィールドは `a-b` 範囲とみなし `Range{start,end}` を返す（展開しない）。それ以外は `Single`。
///   範囲の両端は trim する。
/// - 読めない数は 0 にせず `None`（`UnreadableNumber`）。
/// - `Range` の幅（`end - start + 1`・`u64`）をシェル全体で足し、合計が `RANGE_WIDTH_CAP` を越えるなら
///   `None`（`RangeTooWide`）。番号の値と除外の範囲（広げない）にはかけない。
fn parse_target_element(field: &str, reading: &mut Reading) -> Option<AppendTarget> {
    let number = |text: &str| text.trim().parse::<u32>().ok();
    let pair = |text: &str| {
        let (start, end) = text.split_once('-')?;
        Some((number(start)?, number(end)?))
    };
    // 除外指定 `!`（satori 実例・ukadoc）。
    let target = match field.strip_prefix('!') {
        Some(excluded) if excluded.contains('-') => {
            pair(excluded).map(|(start, end)| AppendTarget::ExcludeRange { start, end })
        }
        Some(excluded) => number(excluded).map(AppendTarget::Exclude),
        None if field.contains('-') => {
            pair(field).map(|(start, end)| AppendTarget::Range { start, end })
        }
        None => number(field).map(AppendTarget::Single),
    };
    let Some(target) = target else {
        reading.unreadable(field);
        return None;
    };
    if let AppendTarget::Range { start, end } = target {
        // 逆向きの範囲も同じ幅で数える（少なく見積もらない）。
        let width = u64::from(start.abs_diff(end)) + 1;
        if reading.range_width + width > RANGE_WIDTH_CAP {
            reading.too_wide(field);
            return None;
        }
        reading.range_width += width;
    }
    Some(target)
}

/// トップレベル行が `animation-sort`／`collision-sort` なら値を `shell` へ充填する。
///
/// - field[0] がキー名・field[1] が値（`ascend`→`Ascend`・`descend`→`Descend`）。
/// - それ以外の空でない値は原文のまま `Other` で運ぶ。空の値・欠落は `None`（既定適用は下流）。
/// - キーが両者いずれでもない行（`charset` 等）は何もしない（非保持）。
fn decode_sort_key(shell: &mut Shell, fields: &[String]) {
    let key = fields.first().map(String::as_str).unwrap_or("");
    let value = fields.get(1).map(String::as_str);
    let order = match value {
        Some("ascend") => Some(SortOrder::Ascend),
        Some("descend") => Some(SortOrder::Descend),
        // 知らない値は原文を運ぶ（既定 descend/none の解釈と知らせは下流）。
        Some(other) if !other.is_empty() => Some(SortOrder::Other(other.into())),
        _ => None,
    };
    match key {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1
        "animation-sort" => shell.animation_sort = order,
        "collision-sort" => shell.collision_sort = order,
        _ => {}
    }
}

/// `kero.surface.alias` ブロックの本体行群を `SurfaceAlias` 群へ decode し `shell.aliases` へ積む。
///
/// `KEY,[id,...]` は field[0]==KEY・field[1]=="[id,...]" として届く
/// （`[...]` 内カンマは lexer が保護するため 1 フィールド）。
///
/// - **キーは不透明**（field[0]）: 数値（`6`）・日本語（`静観`）いずれも意味解釈せず `AliasKey` へ包む。
/// - **値は順序付き数値 ID**: field[1] の角括弧を剥がし、内側をカンマ分割して u32 へ写す。
///   読めない項目は 0 にせず飛ばして崩れを積む。
/// - **重複キーは潰さない**: 同一キーの複数出現を全保持する（衝突解決は下流）。
/// - キー欠落は読み飛ばす。値欠落・空 `[]` は空 ids で保持する。
fn decode_alias_block(shell: &mut Shell, body: &[Vec<String>], reading: &mut Reading) {
    for fields in body {
        let key = match fields.first() {
            Some(k) => k,
            None => continue,
        };
        shell.definitions.push(DefRef::Alias(shell.aliases.len()));
        shell.aliases.push(SurfaceAlias {
            key: AliasKey::new(key.clone()),
            ids: parse_alias_ids(fields.get(1).map(String::as_str).unwrap_or(""), reading),
        });
        // 値は欄 1 の `[id,...]` だけ。角括弧の無い `6,2106,2206` の 2 つ目より後は読まないので崩れに積む。
        for extra in fields.iter().skip(2) {
            reading.unreadable(extra);
        }
    }
}

/// alias 値フィールド `[id,id,...]` を順序付き `Vec<u32>` へ写す。
///
/// 前後の `[` `]` は無くてもよい。空要素・非数値は飛ばして崩れを積む。
/// 空（`[]` または空文字列）は空 `Vec` を返す。
fn parse_alias_ids(value: &str, reading: &mut Reading) -> Vec<u32> {
    let inner = value
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(value)
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .filter_map(|s| {
            let id = s.trim().parse::<u32>().ok();
            if id.is_none() {
                reading.unreadable(s.trim());
            }
            id
        })
        .collect()
}

/// 文字コードごとの固定物 4 種が同一の解析結果を産むことを固定する兄弟テスト。
#[cfg(test)]
#[path = "decode_charset_tests.rs"]
mod charset_tests;

/// 引き金の 3 語（`talk,数値`・`runonce`・`periodic,数値`）の読みを固定する兄弟テスト。
#[cfg(test)]
#[path = "decode_interval_trigger_tests.rs"]
mod interval_trigger_tests;

/// pattern定義の欄 2 の原文（`(` で始まる並びの継ぎ直しを含む）を固定する兄弟テスト。
#[cfg(test)]
#[path = "decode_pattern_target_tests.rs"]
mod pattern_target_tests;

/// `collisionex` の記録の型と読み手を固定する兄弟テスト。
#[cfg(test)]
#[path = "decode_collisionex_tests.rs"]
mod collisionex_tests;
