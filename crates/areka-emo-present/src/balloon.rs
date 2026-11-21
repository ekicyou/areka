//! バルーン枠画像を、シェルと同一の compose/present 経路へ載せる入力適合層。
//!
//! バルーンの入力は scope 別に解決した面画像。scope 番号から接頭辞優先連鎖（`prefix_chain`）を導出し、
//! 面 ID 単位で連鎖を辿って採用面列（`ResolvedFace`）を決め、synthetic surfaces.txt テキスト
//! （`surface{ID}` に単一 overlay element＝採用面の実ファイル名）を生成して `areka_parsers::shell::parse` →
//! `areka_emo_atlas::bake` → `EmoWorld::build`＋`bind_atlas` と、シェルと同じ公開 API 経路で
//! `(EmoWorld, AtlasTable)` を組む。直 WIC バイパスは設けない。
//!
//! - 列挙対象は当該 scope の連鎖に載る接頭辞の `{接頭辞}{ID}.png` だけ（scope 0 は `balloonp0def`/`balloons`、
//!   scope 1 はそれらに先立つ `balloonp1def`/`balloonk`）。`balloonc*`（入力ボックス）・`arrow*`・`marker*`・
//!   `online*` はどの連鎖にも載らず列挙されない。`balloonk*` は scope 1 以上でのみ採用される。
//! - 透過の宣言は基層 `descript.txt` の `use_self_alpha` を `load_balloon_use_self_alpha` がバルーン 1 つにつき
//!   1 回読み、[`build_balloon_target_from_faces`] が引数で受ける。面ごとの設定ファイルでは上書きしない。
//!   `.pna` は表示に使わない。
//! - surface id は `{接頭辞}{ID}` の ID そのまま（`balloon.defaultsurface` 既定 0 と整合）。
//!
//! 失敗経路は log-first（`tracing::error!`＋`Err`）。一覧の失敗・面 0 が解決できない・採用面列が空・bake の脱落は、
//! 真因をログへ出したうえで原因ごとの [`BalloonLoadError`] を返す。読み込みの失敗は提示（`apply_show`）より前に
//! 呼び手へ返るので、合成の全透明（Hide 縮退）とは別物。呼び手はバルーン未配線・告知・採寸の見送りへ縮退するだけで、
//! ゴーストごと殺さない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AnimationLimits, AtlasTable, ElementDecoder, PackConfig, SetId, SurfaceSet,
    UseSelfAlpha, bake_quiet, bake_with_limits,
};
use areka_emo_compose::EmoWorld;
use areka_parsers::balloon::BalloonModel;
use areka_parsers::charset::{DefaultEncoding, decode};

use crate::load_records::LoadRecords;

/// 面画像の拡張子（小文字比較）。シェル側の面画像（`surface{ID}.png`）も同じで、判定 [`face_digits_of`] ごと共有する。
const FRAME_SUFFIX: &str = ".png";
/// バルーンの読み込みの失敗（原因ごと・[`crate::shell_target::ShellLoadError`] と同じ形）。
///
/// どの変位も、返す前に権威の側が真因を `error!` で記録している。
#[derive(Debug, thiserror::Error)]
pub enum BalloonLoadError {
    /// バルーンのフォルダの一覧が取れなかった。
    #[error("balloon フォルダの一覧に失敗: {path}")]
    List {
        /// 一覧しようとしたバルーンのフォルダ。
        path: PathBuf,
        /// OS が返した理由。
        #[source]
        source: std::io::Error,
    },
    /// 面 ID 0 が連鎖のどの接頭辞でも見つからない。
    #[error("balloon の面 0 がどの接頭辞でも見つからない: {path} scope {scope}")]
    NoFace0 {
        /// バルーンのフォルダ。
        path: PathBuf,
        /// 解決しようとしたスコープ。
        scope: u32,
    },
    /// 組み立てに渡された採用面列が空。
    #[error("balloon の採用面列が空: {path}")]
    Empty {
        /// バルーンのフォルダ。
        path: PathBuf,
    },
    /// 面画像の bake が 1 枚以上脱落した。
    #[error("balloon の面画像を焼けなかった: {path}")]
    Bake {
        /// バルーンのフォルダ。
        path: PathBuf,
    },
}

/// バルーン既定設定ファイル名（2 層マージの**基層**・シェル側 descript と同名別物）。
pub(crate) const DESCRIPT_TXT: &str = "descript.txt";

/// 系列族の定義（表データ）。
///
/// バルーン資産は族をまたいで同一の scope 別接尾辞体系を持つ（吹き出し `balloons` / `balloonk` /
/// `balloonp{n}def`、スクロール矢印 `arrows` / `arrowk` / `arrowp{n}def`、マーカー `markers` / `markerk` /
/// `markerp{n}def`）。族名でパラメタ化して保持するので、他族も同じ機構を再利用できる。
///
/// 旧名候補は可変長。装飾族には旧名がもう一段深く存在する（`arrows` が本体用で、旧バージョン対応のため
/// `arrow` で代用を推奨／`markers` に対する `marker` も同型）ので、`scope0_legacy` に `["arrows", "arrow"]` と
/// 積めば表現できる。
#[derive(Debug, Clone, Copy)]
pub struct SeriesFamily {
    /// 族の基底名（吹き出し族＝`"balloon"`・正規名は `{base}p{n}def`）。
    pub base: &'static str,
    /// scope 0 の旧名候補列（吹き出し族＝`["balloons"]`）。
    pub scope0_legacy: &'static [&'static str],
    /// scope 1 の旧名候補列（吹き出し族＝`["balloonk"]`）。
    pub scope1_legacy: &'static [&'static str],
}

/// 吹き出し族。
pub(crate) const BALLOON_FAMILY: SeriesFamily = SeriesFamily {
    base: "balloon",
    scope0_legacy: &["balloons"],
    scope1_legacy: &["balloonk"],
};

/// 連鎖内の 1 接頭辞（採用時の分類タグ付き）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeriesPrefix {
    /// 探索に用いる接頭辞（例: `"balloonp1def"` / `"balloonk"`）。
    pub prefix: String,
    /// この接頭辞が連鎖内で担う役割。
    pub tier: ChainTier,
}

/// 連鎖内での役割（フォールバック分類を運ぶ——採用時の警告判定・対応表記録の区分に用いる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTier {
    /// 当該 scope 自身の候補（正規名＋当該 scope の旧名）。
    Own,
    /// n≧2 連鎖の名指し相方系列（`balloonk`）。正典が三人目以降の流用先として名指ししたもので、
    /// scope 1 の解決へ再帰的に縮退するものではない。
    KeroNamed,
    /// デフォルト定義（`balloonp0def` → `balloons`）。全連鎖の最終受け皿は scope 0 だけが持つ地位で、
    /// scope 1 は持たない。
    Default,
}

impl SeriesFamily {
    /// 当該 scope の正規名 `{base}p{scope}def`。
    ///
    /// 正典は p 系列を `\p[2]` 以降としてのみ記述する。scope 0 / 1 の正規名（`balloonp0def` / `balloonp1def`）を
    /// 先行探索するのは areka 裁量の正規化拡張。
    fn canonical(&self, scope: u32) -> String {
        format!("{}p{scope}def", self.base)
    }

    /// 当該 scope の**旧名候補列**（scope 0 / 1 のみが持ち、scope 2 以上は旧名なし＝空）。
    fn legacy(&self, scope: u32) -> &'static [&'static str] {
        match scope {
            0 => self.scope0_legacy,
            1 => self.scope1_legacy,
            _ => &[],
        }
    }
}

/// scope 番号から**接頭辞優先連鎖**を導出する（純関数・表データ駆動）。
///
/// 連鎖は 3 段の連結である——
///
/// ```text
/// chain(s) = Own(s) ++ KeroNamed(s≧2 のみ) ++ Default(s≧1 のみ)
///   Own      : 正規名 {base}p{s}def ＋ 当該 scope の旧名候補列
///              （s=0 → [balloonp0def, balloons] / s=1 → [balloonp1def, balloonk] / s≧2 → [balloonp{s}def]）
///   KeroNamed: 正典が名指しする相方系列の旧名候補列（[balloonk]）
///   Default  : 正規名 {base}p0def ＋ scope 0 の旧名候補列（[balloonp0def, balloons]）
/// ```
/// 各段では正規名を先頭・旧名を後続に置く。結果は scope 0 が `balloonp0def` → `balloons`、
/// scope 1 が `balloonp1def` → `balloonk` → `balloonp0def` → `balloons`、scope n（n≧2）が
/// `balloonp{n}def` → `balloonk` → `balloonp0def` → `balloons`。
///
/// 相方系列の段は正典が名指しした系列で、scope 1 の解決へ再帰的に縮退するものではないので、
/// scope 2 以上の連鎖に `balloonp1def` は含めない。scope は最後まで数値のみで扱い（`Sakura`/`Kero` 等の
/// 2 値列挙も `\h`/`\u` も正準表現としない）、n≧2 を含む scope 番号一般で解決規則を定義する。
pub(crate) fn prefix_chain(family: &SeriesFamily, scope: u32) -> Vec<SeriesPrefix> {
    let mut chain: Vec<SeriesPrefix> = Vec::new();

    chain.push(SeriesPrefix {
        prefix: family.canonical(scope),
        tier: ChainTier::Own,
    });
    for name in family.legacy(scope) {
        chain.push(SeriesPrefix {
            prefix: (*name).to_string(),
            tier: ChainTier::Own,
        });
    }

    if scope >= 2 {
        for name in family.scope1_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::KeroNamed,
            });
        }
    }

    if scope >= 1 {
        chain.push(SeriesPrefix {
            prefix: family.canonical(0),
            tier: ChainTier::Default,
        });
        for name in family.scope0_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::Default,
            });
        }
    }

    chain
}

/// 解決済みの 1 面（連鎖探索の採用結果）。
///
/// 解決の一時値で、構築後に保持しない（上書きファイル名は本モジュールの規則でいつでも再導出できる）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFace {
    /// 面 ID（`{接頭辞}{ID}.png` の `{ID}` ＝ surface id）。
    pub surface_id: u32,
    /// 採用接頭辞（連鎖内で最初に画像が存在したもの）。
    pub prefix: String,
    /// 採用接頭辞が連鎖内で担っていた役割（scope≧1 での [`ChainTier::Default`] 採用は本体側への縮退で、
    /// 警告記録の判定に使う）。
    pub tier: ChainTier,
    /// 実ファイル名（**原形保持**——実 WIC デコードが実パスを読むため大小を正規化しない）。
    pub file_name: String,
}

impl ResolvedFace {
    /// 採用接頭辞に対応する面別上書きファイル名 `{採用接頭辞}{面 ID}s.txt`。
    ///
    /// 上書き層は採用した画像に対応するものを引く（scope 1 が `balloonk0` を採ったなら `balloonk0s.txt`。
    /// ID 単位フォールバックで後段接頭辞へ落ちた面も、その接頭辞の同一 ID の上書き層が対応する）。
    /// 導出元は実ファイル名の [`Self::file_name`] でなく連鎖上の接頭辞 [`Self::prefix`]
    /// （実ファイル名の大小は資産側の都合で揺れるため）。
    pub fn override_file_name(&self) -> String {
        format!("{}{}s.txt", self.prefix, self.surface_id)
    }
}

/// `{prefix}{数字列}.png`（大小無視）なら数字列そのものを返す。面でなければ `None`。
///
/// 判定は厳密 3 段——接頭辞の完全一致 strip（大小無視）・`.png` の strip・残余の全数字化（空でなく、
/// すべて ASCII 数字）。接頭辞が完全一致なので、`balloonc0.png`（入力ウィンドウ用）を `balloonk` の面と
/// 誤認しない。3 段目を `u32::parse` 任せにせず全数字を検査するのは、`parse` が符号
/// （`balloons+0.png` の `+0`）を受理するため。
///
/// 数でなく数字列を返すのは、シェル側の面画像判定（[`crate::shell_target::select_surface_images`]・
/// 接頭辞 `surface`）と同じ 3 段を共有するため。シェル側は `u32` に収まらない数字列を「桁溢れ」として
/// 別に数える必要があり、数へ畳むとその区別が付かない。
pub(crate) fn face_digits_of(prefix: &str, name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let digits = lower
        .strip_prefix(prefix.to_ascii_lowercase().as_str())?
        .strip_suffix(FRAME_SUFFIX)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(digits.to_string())
}

/// `{prefix}{ID}.png`（大小無視）なら面 ID を返す。バルーン面でなければ `None`。
///
/// [`face_digits_of`] の数字列を 10 進数として読むだけ（先頭の 0 は無視）。`u32` に収まらない数字列は面と認めない。
fn face_id_of(prefix: &str, name: &str) -> Option<u32> {
    face_digits_of(prefix, name)?.parse::<u32>().ok()
}

/// ファイル名リストと接頭辞連鎖から採用面列を決める純核（fs 非依存）。
///
/// 1. 面 ID 集合の和: 連鎖内の全接頭辞にまたがって面 ID を集める。ある接頭辞にしか存在しない ID も候補に入る。
/// 2. ID 単位の連鎖走査: 各 ID について連鎖を先頭から辿り、最初に画像が存在した接頭辞の面を採用する。
///    ある ID の欠落は当該 ID の採用先を後段へずらすだけで、系列全体は切り替えない（`balloonk0` があり
///    `balloonk1` が無ければ、scope 1 の面 0 は `balloonk0`・面 1 は `balloons1`）。
///
/// 連鎖の末尾は常に族の scope 0 旧名（`balloons`）なので、先頭側接頭辞の画像を 1 枚も含まないバルーンでは
/// 全 scope が `balloons` 系列へ収束する。
///
/// 戻りは surface id 昇順で、ディレクトリ走査順に依存しない。同一 (ID, 接頭辞) に大小違いの複数ファイルが
/// 併存する病的入力では、ファイル名の辞書順最小を採る。
pub(crate) fn select_faces<S: AsRef<str>>(
    names: &[S],
    chain: &[SeriesPrefix],
) -> Vec<ResolvedFace> {
    // (面 ID, 連鎖位置) → 実ファイル名。BTreeMap のキー順が「ID 昇順・連鎖先頭優先」と
    // 一致するため、和の構築と ID 単位走査を 1 本の走査で両立できる。
    let mut hits: BTreeMap<(u32, usize), String> = BTreeMap::new();
    for name in names {
        let name = name.as_ref();
        // 1 つの名前が複数の接頭辞に一致し得る（族の旧名が入れ子な場合）ため全段を見る。
        for (index, candidate) in chain.iter().enumerate() {
            let Some(id) = face_id_of(&candidate.prefix, name) else {
                continue;
            };
            hits.entry((id, index))
                .and_modify(|adopted| {
                    if name < adopted.as_str() {
                        *adopted = name.to_string();
                    }
                })
                .or_insert_with(|| name.to_string());
        }
    }

    let mut faces: Vec<ResolvedFace> = Vec::new();
    for ((surface_id, index), file_name) in hits {
        if faces.last().is_some_and(|f| f.surface_id == surface_id) {
            continue;
        }
        faces.push(ResolvedFace {
            surface_id,
            prefix: chain[index].prefix.clone(),
            tier: chain[index].tier,
            file_name,
        });
    }
    faces
}

/// `balloon_dir` 直下のファイル名を 1 回の走査で集める（面判定は行わない）。
///
/// 連鎖内の接頭辞ごとに走査を繰り返さないため、全名を返して選択は [`select_faces`] に任せる。非 UTF-8 名は
/// 面のファイル名規約（ASCII の接頭辞＋数字＋拡張子）を満たし得ないのでこの段で落とす。
/// 走査の失敗は log-first で [`BalloonLoadError::List`] を返す。1 件を飛ばす `warn!` は `records` が出すときだけ、
/// 走査失敗の `error!` は常に出す。
fn enumerate_file_names(
    balloon_dir: &Path,
    records: LoadRecords,
) -> Result<Vec<String>, BalloonLoadError> {
    let read_dir = std::fs::read_dir(balloon_dir).map_err(|e| {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            error = %e,
            "balloon: バルーンディレクトリの走査に失敗"
        );
        BalloonLoadError::List {
            path: balloon_dir.to_path_buf(),
            source: e,
        }
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                if records.emits() {
                    tracing::warn!(error = %e, "balloon: ディレクトリエントリの取得に失敗（スキップ）");
                }
                continue;
            }
        };
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// `balloon_dir` から当該 `scope` の採用面列を解決する（全消費者共通の単一施行点）。
///
/// `prefix_chain` で連鎖を導出し、`enumerate_file_names` で走査を 1 回だけ行い、`select_faces` で面 ID 単位の
/// 連鎖探索を行う。戻りは surface id 昇順で決定論的。
///
/// # 面 0 必在契約
///
/// 面 ID 0 が連鎖のどの接頭辞でも解決できなければ、`error!` で記録して [`BalloonLoadError::NoFace0`] を返す。
/// 権威側の 1 箇所で施行するので、消費者（placement / boot）が独自に面 0 の有無を判定する必要はなく、
/// 無言で空のバルーンを表示するログ無し経路を作らない。失敗は消費者側の縮退経路（バルーン未配線・起動窓を
/// 開けない告知と終了）へそのまま伝播し、プロセス終了ポリシーは本関数で変えない。
///
/// # 観測
///
/// - 解決完了時に `info!` で scope・連鎖・採用面一覧 `(id, prefix, file)` を記録する。
/// - scope 1 以上で [`ChainTier::Default`]（本体側）へ縮退した面は、scope・面 ID・採用ファイルを面ごとに
///   `warn!` で記録する。失敗ではなく正典準拠のフォールバックの観測で、解決は成功として続行する。
pub fn resolve_balloon_faces(
    balloon_dir: &Path,
    scope: u32,
) -> Result<Vec<ResolvedFace>, BalloonLoadError> {
    resolve_balloon_faces_with(balloon_dir, scope, LoadRecords::Emit)
}

/// [`resolve_balloon_faces`] と同じ中身で解決し、読み込みの記録（縮退の `warn!`・解決の完了の
/// `info!`・一覧の 1 件を飛ばす `warn!`）を出さない（採寸と windowposition の読み）。
/// 戻り値は [`resolve_balloon_faces`] と等しい。
///
/// # Errors
///
/// [`resolve_balloon_faces`] と同じ 2 つ。走査の失敗と面 0 が解決できない失敗の `error!` は出す。
pub fn resolve_balloon_faces_quiet(
    balloon_dir: &Path,
    scope: u32,
) -> Result<Vec<ResolvedFace>, BalloonLoadError> {
    resolve_balloon_faces_with(balloon_dir, scope, LoadRecords::Quiet)
}

/// 系列の解決の中身（1 本）。読み込みの記録は `records` が出すときだけ出す。
fn resolve_balloon_faces_with(
    balloon_dir: &Path,
    scope: u32,
    records: LoadRecords,
) -> Result<Vec<ResolvedFace>, BalloonLoadError> {
    let chain = prefix_chain(&BALLOON_FAMILY, scope);
    let names = enumerate_file_names(balloon_dir, records)?;
    let faces = select_faces(&names, &chain);

    let chain_view: Vec<&str> = chain.iter().map(|p| p.prefix.as_str()).collect();

    if !faces.iter().any(|f| f.surface_id == 0) {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            scope = scope,
            chain = ?chain_view,
            resolved_faces = faces.len(),
            "balloon: 面 ID 0 が連鎖のどの接頭辞でも解決できない"
        );
        return Err(BalloonLoadError::NoFace0 {
            path: balloon_dir.to_path_buf(),
            scope,
        });
    }
    if !records.emits() {
        return Ok(faces);
    }

    // scope 0 はデフォルト段を持たない（自身が最終受け皿）ので縮退しない。
    if scope >= 1 {
        for face in faces.iter().filter(|f| f.tier == ChainTier::Default) {
            tracing::warn!(
                scope = scope,
                surface_id = face.surface_id,
                prefix = %face.prefix,
                file = %face.file_name,
                "balloon: 面がデフォルト定義側（本体側）の系列へ縮退した"
            );
        }
    }

    let faces_view: Vec<(u32, &str, &str)> = faces
        .iter()
        .map(|f| (f.surface_id, f.prefix.as_str(), f.file_name.as_str()))
        .collect();
    tracing::info!(
        balloon_dir = %balloon_dir.display(),
        scope = scope,
        chain = ?chain_view,
        faces = ?faces_view,
        "balloon: scope 別バルーン系列の解決が完了"
    );

    Ok(faces)
}

/// バルーン記述ファイルを文字コード解決つきで読む（既定 Ansi・宣言優先＝emo2 は `charset,UTF-8`）。
///
/// 読めたテキストか失敗の [`std::io::Error`] をそのまま返す。必要なログレベルが層ごとに異なる
/// （基層は `warn!`・上書き層の不在は `debug!`）ので、レベル判断は呼び出し側に任せる。
fn read_decoded(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read(path).map(|bytes| decode(&bytes, DefaultEncoding::Ansi))
}

/// 基層（`descript.txt`）を寛容に読む。読取失敗は `warn!`＋`None`（空層で継続）。
///
/// 基層の欠落はバルーン既定設定が丸ごと得られない事態で、正常縮退ではない。ゆえに `debug!` へ降格しない。
pub(crate) fn read_descript_layer(path: &Path, records: LoadRecords) -> Option<String> {
    match read_decoded(path) {
        Ok(text) => Some(text),
        Err(err) => {
            if records.emits() {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "balloon: バルーン既定設定（descript.txt）の読取に失敗（空層で継続）"
                );
            }
            None
        }
    }
}

/// 面別上書き層（`{採用接頭辞}{ID}s.txt`）を寛容に読む。
///
/// - 不在（[`std::io::ErrorKind::NotFound`]）→ `debug!`。上書き層を持たない面は正常で、既定設定のみで定義が確定する。
///   相方側のバルーンが上書き層を持たないだけで毎起動 `warn!` が鳴らないようにする。
/// - その他の入出力エラー（権限・I/O 障害等）→ `warn!`。資産配置の異常で、既定設定のみで継続するが観測可能に残す。
fn read_face_override_layer(path: &Path, scope: u32, file_name: &str) -> Option<String> {
    match read_decoded(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                scope = scope,
                file = %file_name,
                path = %path.display(),
                "balloon: 面別上書き設定が無いため既定設定のみを用いる（正常縮退）"
            );
            None
        }
        Err(err) => {
            tracing::warn!(
                scope = scope,
                file = %file_name,
                path = %path.display(),
                error = %err,
                "balloon: 面別上書き設定の読取に失敗（既定設定のみで継続）"
            );
            None
        }
    }
}

/// 当該 scope 専用のマージ済みバルーン定義を組む（全消費者共通の単一施行点）。
///
/// 基層 `descript.txt` へ、`face0` が採用した面に対応する面別上書きファイル `{採用接頭辞}{ID}s.txt`
/// （[`ResolvedFace::override_file_name`]）を重ねた 2 層を、`areka_parsers::balloon::parse_str` へそのまま渡す。
/// マージ規則（後勝ち・未指定項目の基層継承）はパーサ側の契約で、本関数は層を用意するだけ。
///
/// # 失敗の扱い（log-first）
///
/// `parse_str` は `Result` を返さず panic しない寛容写像なので、本関数も常に [`BalloonModel`] を返す
/// （読取失敗は空層で継続＝欠落キーは当該スカラ `None`）。層ごとのログレベルは `read_descript_layer`／
/// `read_face_override_layer` のとおりで、上書き層の不在のみ `debug!`（正常縮退）。
///
/// # scope 引数について
///
/// scope は [`ResolvedFace`] から逆算できない（本体側へ縮退した相方の面は採用接頭辞が `balloons` になり、
/// scope 0 の面と区別が付かない）ので明示的に受け取る。
///
/// # 観測
///
/// 確定した `windowposition` / `validrect` の実値を scope・採用上書きファイル名とともに `info!` で記録する。
/// placement からは scope あたり 1 行、boot は `load_scope_balloon_definitions` から面ごとに（面 0 と、自前の
/// 設定ファイルを持つ面）1 行ずつ出し、面 0 の行は placement の行と値が一致する。
pub fn load_scope_balloon_model(
    balloon_dir: &Path,
    scope: u32,
    face0: &ResolvedFace,
) -> BalloonModel {
    let descript =
        read_descript_layer(&balloon_dir.join(DESCRIPT_TXT), LoadRecords::Emit).unwrap_or_default();
    scope_balloon_model_with(balloon_dir, scope, face0, &descript)
}

/// [`load_scope_balloon_model`] の核。基層の本文は呼び手が 1 回読んで渡す（面ごとに読み直さない）。
pub(crate) fn scope_balloon_model_with(
    balloon_dir: &Path,
    scope: u32,
    face0: &ResolvedFace,
    descript: &str,
) -> BalloonModel {
    let override_name = face0.override_file_name();
    let face_override =
        read_face_override_layer(&balloon_dir.join(&override_name), scope, &override_name);

    let model = areka_parsers::balloon::parse_str(descript, face_override.as_deref());

    let wp = model.windowposition();
    let vr = model.validrect();
    tracing::info!(
        balloon_dir = %balloon_dir.display(),
        scope = scope,
        file = %override_name,
        windowposition_x = ?wp.x(),
        windowposition_y = ?wp.y(),
        validrect_top = ?vr.top(),
        validrect_bottom = ?vr.bottom(),
        validrect_left = ?vr.left(),
        validrect_right = ?vr.right(),
        "balloon: scope 別バルーン定義の 2 層マージが確定"
    );

    model
}

/// 採用面列から synthetic surfaces.txt テキストを生成する。
///
/// 各面は `surface{ID}` ブロックに単一 overlay element（`element0,overlay,{実ファイル名},0,0`）として転記する。
/// `areka_parsers::shell::parse` が受理する文法に従う。element path は [`ResolvedFace::file_name`]
/// （原形保持の実ファイル名）で、連鎖上の接頭辞ではない（bake が `base_dir.join(rel)` で実パスを開くため）。
fn synthetic_surfaces_txt(faces: &[ResolvedFace]) -> String {
    let mut text = String::new();
    for face in faces {
        let (id, file_name) = (face.surface_id, &face.file_name);
        text.push_str(&format!(
            "surface{id}\n{{\nelement0,overlay,{file_name},0,0\n}}\n\n"
        ));
    }
    text
}

/// 当該 `scope` のバルーン面をシェルと同一の compose/present 経路へ載せ、`(EmoWorld, AtlasTable)` を返す。
///
/// 系列解決（[`resolve_balloon_faces`]）と透過の宣言の読み（[`load_balloon_use_self_alpha`]）を内包する
/// 1 スコープ用の薄いラッパで、構築本体は [`build_balloon_target_from_faces`]。World の面は当該 scope が解決した
/// 系列の面で、全 scope が `balloons` 系列へ畳み込まれることはない（scope 1 が `balloonk0.png` を採ったなら
/// World の面 0 はその画像）。
///
/// 解決済み面列を既に持つ消費者は、二重列挙を避けるため [`build_balloon_target_from_faces`] を直接呼べる。
///
/// 失敗（面 0 不在・ディレクトリ走査失敗・bake 脱落）は log-first で真因をログし、原因ごとの
/// [`BalloonLoadError`] を返す。
pub fn build_balloon_target(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    scope: u32,
) -> Result<(EmoWorld, AtlasTable), BalloonLoadError> {
    let faces = resolve_balloon_faces(balloon_dir, scope)?;
    let use_self_alpha = load_balloon_use_self_alpha(balloon_dir);
    build_balloon_target_from_faces(balloon_dir, decoder, &faces, use_self_alpha)
}

/// バルーンの透過の宣言（`use_self_alpha`）を読む。
///
/// バルーン 1 つの読み込みにつき 1 回呼ぶ（スコープのループの外）。記録はここで 1 度だけ出る。
/// 読むのは基層の `descript.txt` だけで、面ごとの設定ファイル（`balloons0s.txt` など）と
/// [`load_scope_balloon_model`] の 2 層の重ね合わせは通さないので、上書きは構造として起きない。
/// シェルのキー `seriko.use_self_alpha` は読まない。
///
/// 同じ関数がフォルダ直下の `.pna`（拡張子の大小を区別しない）を数え、0 でなければ数つきの
/// `warn!` を 1 行出す。`.pna` は表示に使わない。数には areka が読まない絵（矢印など）に
/// 添えた物も含む。
///
/// `descript.txt` が読めないときは `read_descript_layer` の `warn!` を通して宣言なしとし、
/// 一覧が取れないときは `enumerate_file_names` の `error!` を通して `.pna` の数を 0 として続ける
/// （続く系列の解決が同じ理由で失敗を返す）。
pub fn load_balloon_use_self_alpha(balloon_dir: &Path) -> UseSelfAlpha {
    load_balloon_use_self_alpha_with(balloon_dir, LoadRecords::Emit)
}

/// [`load_balloon_use_self_alpha`] と同じ中身で読み、読み込みの記録（descript.txt が読めない
/// `warn!`・透過の宣言の記録・`.pna` の `warn!`・一覧の 1 件を飛ばす `warn!`）を出さない（採寸の読み）。
/// 戻り値は [`load_balloon_use_self_alpha`] と等しい。フォルダの走査の失敗の `error!` は出す。
pub fn load_balloon_use_self_alpha_quiet(balloon_dir: &Path) -> UseSelfAlpha {
    load_balloon_use_self_alpha_with(balloon_dir, LoadRecords::Quiet)
}

/// 透過の宣言の読みの中身（1 本）。読み込みの記録は `records` が出すときだけ出す。
fn load_balloon_use_self_alpha_with(balloon_dir: &Path, records: LoadRecords) -> UseSelfAlpha {
    let descript = read_descript_layer(&balloon_dir.join(DESCRIPT_TXT), records);
    let use_self_alpha = crate::self_alpha::read_use_self_alpha(
        "balloon",
        "use_self_alpha",
        balloon_dir,
        descript.as_deref(),
        records,
    );
    let ignored_pna = enumerate_file_names(balloon_dir, records)
        .unwrap_or_default()
        .iter()
        .filter(|name| {
            name.to_ascii_lowercase().ends_with(".pna") && balloon_dir.join(name).is_file()
        })
        .count();
    if records.emits() && ignored_pna != 0 {
        tracing::warn!(
            balloon_dir = %balloon_dir.display(),
            ignored_pna,
            "balloon: フォルダ直下の .pna を使わずに表示する（数は areka が読まない絵に添えた物も含む）"
        );
    }
    use_self_alpha
}

/// 解決済み面列から `(EmoWorld, AtlasTable)` を組む（構築本体）。
///
/// 採用面列 → synthetic surfaces.txt → `shell::parse` → `bake`（透過は引数 `use_self_alpha`）→
/// `EmoWorld::build`＋`bind_atlas` と、直 WIC バイパス無しでシェルと同一機構に載せる。得た組を
/// `attach_target` に渡すだけでシェルと同じ提示経路へ乗る。
///
/// どの scope の面列かは呼び出し側が決める。透過の宣言も呼び出し側が [`load_balloon_use_self_alpha`] で
/// スコープのループの外で 1 回読んで渡す（スコープごとに読むと記録がスコープの数だけ出る）。
/// 面列が空／bake がエラーなら log-first で真因をログし [`BalloonLoadError::Empty`]／
/// [`BalloonLoadError::Bake`] を返す。
pub fn build_balloon_target_from_faces(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    faces: &[ResolvedFace],
    use_self_alpha: UseSelfAlpha,
) -> Result<(EmoWorld, AtlasTable), BalloonLoadError> {
    build_balloon_target_from_faces_with_limits(
        balloon_dir,
        decoder,
        faces,
        use_self_alpha,
        AnimationLimits::from_env(),
    )
}

/// [`build_balloon_target_from_faces`] の動く絵の上限を外から渡す版（記録も失敗も同じ・上限だけが
/// 違う）。環境変数に左右されないテストのための口。
///
/// # Errors
///
/// [`build_balloon_target_from_faces`] と同じ。
pub fn build_balloon_target_from_faces_with_limits(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    faces: &[ResolvedFace],
    use_self_alpha: UseSelfAlpha,
    limits: AnimationLimits,
) -> Result<(EmoWorld, AtlasTable), BalloonLoadError> {
    build_from_faces_core(
        balloon_dir,
        decoder,
        faces,
        use_self_alpha,
        LoadRecords::Emit,
        limits,
    )
}

/// [`build_balloon_target_from_faces`] と同じ中身で、焼く段と面の表の記録を出さない入口（採寸の面 0）。
/// 戻り値は同じ。面列が空の失敗と bake の脱落の `error!` は出す。
pub fn build_balloon_target_from_faces_quiet(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    faces: &[ResolvedFace],
    use_self_alpha: UseSelfAlpha,
) -> Result<(EmoWorld, AtlasTable), BalloonLoadError> {
    build_from_faces_core(
        balloon_dir,
        decoder,
        faces,
        use_self_alpha,
        LoadRecords::Quiet,
        // 採寸の読み手は動く絵の道に入らないので、上限は使われない（`bake_quiet` が環境から取る）。
        AnimationLimits::from_env(),
    )
}

/// 組み立ての中身（1 本）。`records` は焼く段と面の表の記録を出すかどうかだけを切り替える。
fn build_from_faces_core(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    faces: &[ResolvedFace],
    use_self_alpha: UseSelfAlpha,
    records: LoadRecords,
    limits: AnimationLimits,
) -> Result<(EmoWorld, AtlasTable), BalloonLoadError> {
    if faces.is_empty() {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            "balloon: 採用面列が空（構築する面が 1 枚も無い）"
        );
        return Err(BalloonLoadError::Empty {
            path: balloon_dir.to_path_buf(),
        });
    }

    let text = synthetic_surfaces_txt(faces);
    let shell = areka_parsers::shell::parse(&text);

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: balloon_dir,
        alpha_params: AlphaParams { use_self_alpha },
    };
    let baked = if records.emits() {
        bake_with_limits(&[set], decoder, PackConfig::default(), limits)
    } else {
        bake_quiet(&[set], decoder, PackConfig::default())
    };

    // 採用面は解決済みの固定小集合ゆえ全枚デコード成功が前提＝脱落は制作者ミス/配置不備の兆候。
    if !baked.errors.is_empty() {
        for err in &baked.errors {
            tracing::error!(
                balloon_dir = %balloon_dir.display(),
                error = %err,
                "balloon: 面画像の bake に失敗"
            );
        }
        return Err(BalloonLoadError::Bake {
            path: balloon_dir.to_path_buf(),
        });
    }

    // アトラス束縛は SetId(0)・resolve は本呼び出し限り。
    let mut world = if records.emits() {
        EmoWorld::build(&shell)
    } else {
        EmoWorld::build_with_images_quiet(&shell, &BTreeMap::new())
    };
    world.bind_atlas(&baked.table, SetId(0));

    Ok((world, baked.table))
}

#[cfg(test)]
#[path = "balloon_alpha_tests.rs"]
mod alpha_tests;
#[cfg(test)]
#[path = "balloon_model_tests.rs"]
mod model_tests;
#[cfg(test)]
#[path = "balloon_quiet_tests.rs"]
mod quiet_tests;
#[cfg(test)]
#[path = "balloon_series_tests.rs"]
mod series_tests;
#[cfg(test)]
#[path = "balloon_target_tests.rs"]
mod target_tests;
#[cfg(test)]
#[path = "balloon_test_support.rs"]
pub(crate) mod test_support;
