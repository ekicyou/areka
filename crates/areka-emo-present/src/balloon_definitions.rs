//! スコープのバルーンの面ごとの定義の束（balloon-scroll-arrows 要件 1.3・1.5・1.7・
//! 設計「emo-present / ScopeBalloonDefinitions」）。
//!
//! 読み込みの時に 1 度だけ、採用した面ごとに既存の [`load_scope_balloon_model`] を呼んで
//! 面の設定ファイルを重ねたモデルを作り、その面の矢印の絵と束ねる（2 層の重ね方の権威は増やさない）。

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use areka_emo_atlas::{AlphaParams, ElementDecoder};
use areka_parsers::balloon::BalloonModel;

use crate::balloon::{ResolvedFace, load_scope_balloon_model};
use crate::balloon_marker::{
    ARROW_FAMILY, MARKER_FAMILY, MarkerImage, MarkerPair, resolve_marker_pair,
    resolve_single_marker,
};

/// 面 1 つの定義（面の設定ファイルを重ねたモデル＋その面の矢印の絵）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceDefinition {
    pub model: BalloonModel,
    pub arrows: MarkerPair,
}

/// スコープの面の番号 → 定義。面 0 は必ず在る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeBalloonDefinitions {
    scope: u32,
    face0: FaceDefinition,
    others: BTreeMap<u32, FaceDefinition>,
    /// 面の番号 → 行の中の印の絵（面 0 以外は自分の設定ファイルを持つ面だけ）。
    inline_markers: BTreeMap<u32, Option<Arc<MarkerImage>>>,
}

impl ScopeBalloonDefinitions {
    /// 面の番号の定義。無い番号は面 0 の定義を返し `debug!` を残す。
    pub fn for_face(&self, face: u32) -> &FaceDefinition {
        if face == 0 {
            return &self.face0;
        }
        self.others.get(&face).unwrap_or_else(|| {
            tracing::debug!(
                scope = self.scope,
                face,
                "balloon_definitions: 面の定義が無いので面 0 の定義を使う"
            );
            &self.face0
        })
    }

    pub fn face0(&self) -> &FaceDefinition {
        &self.face0
    }

    /// 面の行の中の印の絵（無い面は面 0 の値・絵が無いバルーンは `None`・script-small-tags 要件 5.2〜5.4）。
    pub fn inline_marker_for_face(&self, face: u32) -> Option<Arc<MarkerImage>> {
        self.inline_markers
            .get(&face)
            .or_else(|| self.inline_markers.get(&0))
            .cloned()
            .flatten()
    }

    /// 面 0 だけの束（ファイルを読まずに組む呼び手・テストの支え用）。
    pub fn single(scope: u32, face0: FaceDefinition) -> Self {
        Self {
            scope,
            face0,
            others: BTreeMap::new(),
            inline_markers: BTreeMap::new(),
        }
    }
}

/// 採用した面ごとの定義の束を作る（fs と記録の入口・読み込みの時だけ呼ぶ）。
///
/// 矢印の絵は `arrow.filename` ごとに 1 回だけ解いて面どうしで共有する。読めなかった座標の鍵は
/// `(鍵, 値)` で重複を除き、スコープにつき 1 回ずつ `warn!`（descript.txt の値は全部の面に重なる）。
pub fn load_scope_balloon_definitions(
    balloon_dir: &Path,
    scope: u32,
    faces: &[ResolvedFace],
    decoder: &impl ElementDecoder,
    alpha: AlphaParams,
) -> ScopeBalloonDefinitions {
    let mut arrows_by_name: HashMap<Option<String>, MarkerPair> = HashMap::new();
    let mut issues: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut defs: BTreeMap<u32, FaceDefinition> = BTreeMap::new();
    let mut markers_by_name: HashMap<Option<String>, Option<Arc<MarkerImage>>> = HashMap::new();
    let mut inline_markers: BTreeMap<u32, Option<Arc<MarkerImage>>> = BTreeMap::new();
    for face in faces {
        // 自分の設定ファイルを持たない面は面 0 の定義のまま（要件 1.6・for_face が面 0 へ落とす）。
        if face.surface_id != 0 && !balloon_dir.join(face.override_file_name()).exists() {
            continue;
        }
        let model = load_scope_balloon_model(balloon_dir, scope, face);
        let keys = model.arrow_keys();
        issues.extend(keys.issues.iter().map(|i| (i.key, i.value.clone())));
        let arrows = arrows_by_name
            .entry(keys.filename.clone())
            .or_insert_with(|| {
                resolve_marker_pair(
                    &[balloon_dir],
                    &ARROW_FAMILY,
                    scope,
                    keys.filename.as_deref(),
                    decoder,
                    alpha,
                )
            })
            .clone();
        let marker_name = model.marker_filename().map(str::to_string);
        let marker = markers_by_name
            .entry(marker_name.clone())
            .or_insert_with(|| {
                let found = resolve_single_marker(
                    &[balloon_dir],
                    &MARKER_FAMILY,
                    scope,
                    marker_name.as_deref(),
                    decoder,
                    alpha,
                );
                if found.explicit_missing {
                    tracing::warn!(
                        balloon_dir = %balloon_dir.display(),
                        scope,
                        name = %marker_name.as_deref().unwrap_or_default(),
                        "balloon_definitions: 書いた印の絵が無いので既定の名前の順で探し直す"
                    );
                }
                found.image
            })
            .clone();
        inline_markers.insert(face.surface_id, marker);
        defs.insert(face.surface_id, FaceDefinition { model, arrows });
    }
    // 読み込み 1 回につき 1 回だけ（名前の違う面が並んでも・フレームごとには出さない・要件 5.4）。
    if inline_markers.values().any(Option::is_none) {
        tracing::info!(
            balloon_dir = %balloon_dir.display(),
            scope,
            "balloon_definitions: 印の絵が無い面がある（既定の丸を描く）"
        );
    }
    for (key, value) in &issues {
        tracing::warn!(
            balloon_dir = %balloon_dir.display(),
            scope,
            key,
            value = %value,
            "balloon_definitions: 矢印の座標の値が数として読めない（書かれていないものとして扱う）"
        );
    }

    // 面 0 必在は resolve_balloon_faces の契約。破られたときも panic せず記録して続ける。
    let face0 = defs.remove(&0).unwrap_or_else(|| {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            scope,
            faces = faces.len(),
            "balloon_definitions: 面 0 の定義が無い（先頭の面か空の定義で代える）"
        );
        defs.values()
            .next()
            .cloned()
            .unwrap_or_else(|| FaceDefinition {
                model: areka_parsers::balloon::parse_str("", None),
                arrows: MarkerPair::default(),
            })
    });
    ScopeBalloonDefinitions {
        scope,
        face0,
        others: defs,
        inline_markers,
    }
}

#[cfg(test)]
#[path = "balloon_definitions_tests.rs"]
mod tests;
