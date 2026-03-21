use crate::domain::dependency::DependencyItem;
use crate::domain::release::{ReleaseCacheKey, ReleaseContextState, ReleaseContextStore};
use crate::infra::pm::PackageManager;
use the_other_tui_markdown::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    /// Use the "latest" version from the registry
    Latest,
    /// Use the "wanted" version (max satisfying current range)
    Wanted,
}

impl TargetMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::Wanted => "wanted",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Latest => Self::Wanted,
            Self::Wanted => Self::Latest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// Preserve the semver operator (^ or ~) from the declared range
    PreserveRange,
    /// Pin to exact version
    Exact,
}

impl WriteMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PreserveRange => "preserve-range",
            Self::Exact => "exact",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::PreserveRange => Self::Exact,
            Self::Exact => Self::PreserveRange,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    #[allow(dead_code)]
    Scanning,
    Ready,
    Reviewing,
    Applying,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Table,
    #[allow(dead_code)]
    Details,
}

pub struct AppState {
    pub project_name: String,
    pub project_path: std::path::PathBuf,
    pub package_manager: PackageManager,
    pub target_mode: TargetMode,
    pub write_mode: WriteMode,
    pub items: Vec<DependencyItem>,
    pub selected_row: usize,
    pub release_contexts: ReleaseContextStore,
    pub phase: Phase,
    #[allow(dead_code)]
    pub active_pane: ActivePane,
    /// Logs from the apply step
    pub apply_log: Vec<String>,
    #[allow(dead_code)]
    pub error_message: Option<String>,

    /// When true, the full release notes modal is displayed over the main view.
    pub show_notes_modal: bool,
    /// Scroll offset inside the release notes modal (in lines).
    pub modal_scroll: u16,
    /// Markdown renderer built once at startup, themed to depick's palette.
    pub md_renderer: Renderer,
}

impl AppState {
    pub fn new(
        project_name: String,
        project_path: std::path::PathBuf,
        package_manager: PackageManager,
        items: Vec<DependencyItem>,
    ) -> Self {
        Self {
            project_name,
            project_path,
            package_manager,
            target_mode: TargetMode::Latest,
            write_mode: WriteMode::PreserveRange,
            items,
            selected_row: 0,
            release_contexts: ReleaseContextStore::default(),
            phase: Phase::Ready,
            active_pane: ActivePane::Table,
            apply_log: Vec::new(),
            error_message: None,

            show_notes_modal: false,
            modal_scroll: 0,
            md_renderer: crate::ui::theme::md_renderer(),
        }
    }

    /// Open the release notes modal, resetting its scroll position.
    pub fn open_modal(&mut self) {
        self.show_notes_modal = true;
        self.modal_scroll = 0;
    }

    /// Close the release notes modal.
    pub fn close_modal(&mut self) {
        self.show_notes_modal = false;
        self.modal_scroll = 0;
    }

    /// Scroll the modal down by `n` lines, clamped to `max`.
    pub fn modal_scroll_down(&mut self, n: u16, max: u16) {
        self.modal_scroll = (self.modal_scroll + n).min(max);
    }

    /// Scroll the modal up by `n` lines.
    pub fn modal_scroll_up(&mut self, n: u16) {
        self.modal_scroll = self.modal_scroll.saturating_sub(n);
    }

    /// Returns true if the focused package has loaded release notes available.
    pub fn has_loaded_notes(&self) -> bool {
        use crate::domain::release::ReleaseContextState;
        let Some(item) = self.items.get(self.selected_row) else {
            return false;
        };
        let key = ReleaseCacheKey::from_item(item);
        matches!(
            self.release_contexts.entries.get(&key),
            Some(ReleaseContextState::Loaded(_))
        )
    }

    pub fn selected_item(&self) -> Option<&DependencyItem> {
        self.items.get(self.selected_row)
    }

    pub fn checked_items(&self) -> Vec<&DependencyItem> {
        self.items.iter().filter(|i| i.checked).collect()
    }

    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_row + 1 < self.items.len() {
            self.selected_row += 1;
        }
    }

    pub fn toggle_current(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_row) {
            item.checked = !item.checked;
        }
    }

    pub fn select_all(&mut self) {
        for item in &mut self.items {
            item.checked = true;
        }
    }

    pub fn invert_selection(&mut self) {
        for item in &mut self.items {
            item.checked = !item.checked;
        }
    }

    pub fn selected_count(&self) -> usize {
        self.items.iter().filter(|i| i.checked).count()
    }

    #[allow(dead_code)]
    pub fn release_context_state(&self) -> Option<&ReleaseContextState> {
        let item = self.items.get(self.selected_row)?;
        let key = ReleaseCacheKey::from_item(item);
        self.release_contexts.entries.get(&key)
    }
}
