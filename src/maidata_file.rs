use crate::span::Sp;

/// Top-level maidata file model containing metadata and per-difficulty chart data.
#[derive(Clone, Debug, Default)]
pub struct Maidata {
    title: String,
    artist: String,

    fallback_designer: Option<String>,
    fallback_offset: Option<f64>,
    fallback_single_message: Option<String>,

    // XXX: is wholebpm mandatory?
    _star_bpm: Option<f64>,

    difficulties: Vec<BeatmapData>,
}

impl Maidata {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn iter_difficulties(&self) -> impl Iterator<Item = AssociatedBeatmapData> {
        self.difficulties
            .iter()
            .map(move |diff| AssociatedBeatmapData {
                global: self,
                map: diff,
            })
    }

    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub(crate) fn set_artist(&mut self, artist: String) {
        self.artist = artist;
    }

    pub(crate) fn set_fallback_designer(&mut self, designer: String) {
        self.fallback_designer = Some(designer);
    }

    pub(crate) fn set_fallback_offset(&mut self, offset: f64) {
        self.fallback_offset = Some(offset);
    }

    pub(crate) fn set_fallback_single_message(&mut self, message: String) {
        self.fallback_single_message = Some(message);
    }

    pub(crate) fn set_difficulties(&mut self, difficulties: Vec<BeatmapData>) {
        let mut diffs = difficulties;
        diffs.sort_by_key(|x| x.difficulty());
        self.difficulties = diffs;
    }
}

/// Per-difficulty chart data within a maidata file.
#[derive(Clone, Debug)]
pub struct BeatmapData {
    difficulty: crate::Difficulty,
    designer: Option<String>,
    offset: Option<f64>,
    level: Option<crate::Level>,
    insns: Vec<Sp<crate::insn::RawInsn>>,
    single_message: Option<String>,
}

impl BeatmapData {
    pub(crate) fn difficulty(&self) -> crate::Difficulty {
        self.difficulty
    }

    pub(crate) fn default_with_difficulty(difficulty: crate::Difficulty) -> Self {
        Self {
            difficulty,
            designer: None,
            offset: None,
            level: None,
            insns: vec![],
            single_message: None,
        }
    }

    pub(crate) fn set_designer(&mut self, designer: String) {
        self.designer = Some(designer);
    }

    pub(crate) fn set_offset(&mut self, offset: f64) {
        self.offset = Some(offset);
    }

    pub(crate) fn set_insns(&mut self, insns: Vec<Sp<crate::insn::RawInsn>>) {
        self.insns = insns;
    }

    pub(crate) fn set_level(&mut self, level: crate::Level) {
        self.level = Some(level);
    }

    pub(crate) fn set_single_message(&mut self, message: String) {
        self.single_message = Some(message);
    }
}

/// A borrowed view combining global maidata metadata with a specific difficulty's data.
pub struct AssociatedBeatmapData<'a> {
    global: &'a Maidata,
    map: &'a BeatmapData,
}

impl<'a> AssociatedBeatmapData<'a> {
    pub fn difficulty(&self) -> crate::Difficulty {
        self.map.difficulty
    }

    pub fn designer(&self) -> Option<&str> {
        self.map
            .designer
            .as_deref()
            .or(self.global.fallback_designer.as_deref())
    }

    pub fn offset(&self) -> Option<f64> {
        self.map.offset.or(self.global.fallback_offset)
    }

    pub fn level(&self) -> Option<crate::Level> {
        self.map.level
    }

    pub fn iter_insns(&self) -> impl Iterator<Item = &Sp<crate::insn::RawInsn>> {
        self.map.insns.iter()
    }

    pub fn single_message(&self) -> Option<&str> {
        self.map
            .single_message
            .as_deref()
            .or(self.global.fallback_single_message.as_deref())
    }
}
