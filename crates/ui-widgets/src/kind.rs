// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Widget vocabulary payload for LayoutTree nodes
use crate::id::WidgetId;
use crate::media::MediaKind;

/// Badge status indicator for list items.
#[derive(Debug, Clone, PartialEq)]
pub enum ListItemBadge {
    None,
    Success,
    Warning,
    Active(String),
    Custom { text: String, color: [f32; 4] },
}

/// Status and severity level for Toast notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

/// Orientation for resizable split views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    Horizontal,
    Vertical,
}

/// Orientation for interactive sliders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

/// Visual style and shape for progress indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressKind {
    Horizontal,
    Vertical,
    Ring,
    Pie,
}

/// Built-in scalable cyber iconography glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Folder,
    FolderOpen,
    File,
    FileCode,
    Settings,
    Search,
    Close,
    Check,
    Alert,
    Shield,
    Terminal,
    Cpu,
    Network,
    Lock,
    Unlock,
    Play,
    Pause,
    Refresh,
    Copy,
    Trash,
    Eye,
    EyeOff,
    ChevronDown,
    ChevronRight,
    Plus,
    Minus,
}

impl IconKind {
    pub fn glyph(&self) -> &'static str {
        match self {
            IconKind::Folder => "◫",
            IconKind::FolderOpen => "◩",
            IconKind::File => "≡",
            IconKind::FileCode => "</>",
            IconKind::Settings => "⚙",
            IconKind::Search => "⌕",
            IconKind::Close => "✕",
            IconKind::Check => "✓",
            IconKind::Alert => "▲",
            IconKind::Shield => "⛨",
            IconKind::Terminal => ">_",
            IconKind::Cpu => "⚡",
            IconKind::Network => "⬡",
            IconKind::Lock => "⚿",
            IconKind::Unlock => "⚿",
            IconKind::Play => "▸",
            IconKind::Pause => "‖",
            IconKind::Refresh => "↻",
            IconKind::Copy => "❐",
            IconKind::Trash => "⌫",
            IconKind::Eye => "◎",
            IconKind::EyeOff => "Ø",
            IconKind::ChevronDown => "▾",
            IconKind::ChevronRight => "▸",
            IconKind::Plus => "+",
            IconKind::Minus => "−",
        }
    }
}

/// Declarative widget content payload attached to a [`ui_layout::LayoutTree`] node.
/// This is the generic payload `V` of the layout engine — `ui-layout` carries it
/// without inspecting its contents.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Container,

    Button {
        id: WidgetId,
        label: String,
        enabled: bool,
    },
    IconButton {
        id: WidgetId,
        icon: IconKind,
        enabled: bool,
    },
    Icon {
        kind: IconKind,
        size: f32,
        color: Option<[f32; 4]>,
    },
    Label {
        text: String,
        muted: bool,
    },
    Checkbox {
        id: WidgetId,
        checked: bool,
    },
    TextInput {
        id: WidgetId,
        value: String,
        placeholder: String,
        focused: bool,
        cursor: usize,
        selection: Option<(usize, usize)>,
    },
    TextArea {
        id: WidgetId,
        value: String,
        placeholder: String,
        focused: bool,
        line_numbers: bool,
        cursor: usize,
        selection: Option<(usize, usize)>,
    },
    PasswordInput {
        id: WidgetId,
        value: String,
        placeholder: String,
        focused: bool,
        revealed: bool,
        cursor: usize,
    },
    NumberInput {
        id: WidgetId,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        precision: usize,
        focused: bool,
        enabled: bool,
    },

    Window {
        id: WidgetId,
        title: String,
    },
    WindowCloseButton {
        owner: WidgetId,
    },

    MenuBar,
    MenuPopover,

    TabItem {
        owner: WidgetId,
        index: usize,
        label: String,
        active: bool,
    },
    ListItem {
        owner: WidgetId,
        index: usize,
        text: String,
        selected: bool,
        badge: ListItemBadge,
    },
    SegmentItem {
        owner: WidgetId,
        index: usize,
        label: String,
        selected: bool,
    },
    RadioButton {
        id: WidgetId,
        group_id: String,
        label: String,
        selected: bool,
    },

    TableHeader {
        owner: WidgetId,
        column_index: usize,
        title: String,
        sorted_asc: Option<bool>,
    },
    TableCell {
        owner: WidgetId,
        row_index: usize,
        column_index: usize,
        text: String,
        badge: ListItemBadge,
        selected: bool,
    },
    AccordionHeader {
        id: WidgetId,
        title: String,
        subtitle: Option<String>,
        expanded: bool,
    },

    MenuBarItem {
        owner: WidgetId,
        index: usize,
        label: String,
        active: bool,
    },
    MenuItem {
        owner: WidgetId,
        id: WidgetId,
        label: String,
        shortcut: Option<String>,
        enabled: bool,
    },
    Dropdown {
        id: WidgetId,
        label: String,
        selected_text: String,
        open: bool,
    },
    Toast {
        id: WidgetId,
        title: String,
        message: String,
        kind: ToastKind,
    },
    Tooltip {
        text: String,
    },

    Splitter {
        owner: WidgetId,
        orientation: SplitOrientation,
    },
    TreeNode {
        owner: WidgetId,
        id: WidgetId,
        label: String,
        depth: usize,
        is_dir: bool,
        expanded: bool,
        selected: bool,
    },

    Toggle {
        id: WidgetId,
        active: bool,
    },
    Slider {
        id: WidgetId,
        min: f32,
        max: f32,
        value: f32,
        orientation: SliderOrientation,
    },
    ProgressBar {
        progress: f32,
        kind: ProgressKind,
        label: Option<String>,
    },
    MetricCard {
        title: String,
        value: String,
        delta: Option<(String, bool)>,
    },
    Divider {
        vertical: bool,
    },
    BreadcrumbItem {
        owner: WidgetId,
        index: usize,
        id: String,
        label: String,
        is_last: bool,
    },
    PaginationItem {
        owner: WidgetId,
        page: usize,
        label: String,
        active: bool,
        disabled: bool,
    },
    Badge {
        label: String,
        badge: ListItemBadge,
    },
    ColorSwatch {
        id: WidgetId,
        color: [f32; 4],
        label: Option<String>,
    },
    ColorPicker {
        id: WidgetId,
        color: [f32; 4],
        space: crate::color::ColorSpace,
    },

    Modal {
        id: WidgetId,
        title: String,
    },
    ModalBackdrop {
        owner: WidgetId,
    },

    Palette {
        id: WidgetId,
        title: String,
        folded: bool,
    },
    PaletteHeader {
        owner: WidgetId,
        title: String,
        folded: bool,
    },
    PaletteFoldButton {
        owner: WidgetId,
        folded: bool,
    },
    PaletteCloseButton {
        owner: WidgetId,
    },
    ResizeGrip {
        owner: WidgetId,
    },

    ScrollView {
        id: WidgetId,
        offset: [f32; 2],
    },
    Scrollbar {
        id: WidgetId,
        content_size: f32,
        viewport_size: f32,
        offset: f32,
    },
    Media {
        id: WidgetId,
        kind: MediaKind,
        resource_id: String,
        fit: crate::media::MediaFit,
        radius: Option<f32>,
    },
    VideoPlayer {
        id: WidgetId,
        resource_id: String,
        playing: bool,
        progress: f32,
        duration_sec: f32,
        volume: f32,
    },
    AudioVisualizer {
        id: WidgetId,
        values: Vec<f32>,
        peak: f32,
    },
    CustomPaint {
        id: WidgetId,
        commands: Vec<crate::paint::PaintCommand>,
    },
}

/// Stable identity of an interactive widget across frames, used
/// for hover and pressed state tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteractionKey {
    pub widget_id: WidgetId,
    pub index: Option<usize>,
}

impl WidgetKind {
    /// `true` if a pointer click on this node produces a [`ui_core::UiEvent`]
    /// (dispatched by `WidgetTree::dispatch_click`).
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            WidgetKind::Button { .. }
                | WidgetKind::IconButton { .. }
                | WidgetKind::Checkbox { .. }
                | WidgetKind::Toggle { .. }
                | WidgetKind::Slider { .. }
                | WidgetKind::TextInput { .. }
                | WidgetKind::TextArea { .. }
                | WidgetKind::PasswordInput { .. }
                | WidgetKind::NumberInput { enabled: true, .. }
                | WidgetKind::RadioButton { .. }
                | WidgetKind::SegmentItem { .. }
                | WidgetKind::WindowCloseButton { .. }
                | WidgetKind::ModalBackdrop { .. }
                | WidgetKind::TabItem { .. }
                | WidgetKind::ListItem { .. }
                | WidgetKind::TableHeader { .. }
                | WidgetKind::TableCell { .. }
                | WidgetKind::AccordionHeader { .. }
                | WidgetKind::BreadcrumbItem { is_last: false, .. }
                | WidgetKind::PaginationItem {
                    disabled: false,
                    ..
                }
                | WidgetKind::ColorSwatch { .. }
                | WidgetKind::ColorPicker { .. }
                | WidgetKind::MenuBarItem { .. }
                | WidgetKind::MenuItem { enabled: true, .. }
                | WidgetKind::Dropdown { .. }
                | WidgetKind::Toast { .. }
                | WidgetKind::Splitter { .. }
                | WidgetKind::TreeNode { .. }
                | WidgetKind::PaletteHeader { .. }
                | WidgetKind::PaletteFoldButton { .. }
                | WidgetKind::PaletteCloseButton { .. }
                | WidgetKind::ResizeGrip { .. }
                | WidgetKind::VideoPlayer { .. }
                | WidgetKind::Media { .. }
        )
    }

    /// Stable interaction key for this node, if interactive.
    pub fn interaction_key(&self) -> Option<InteractionKey> {
        match self {
            WidgetKind::Button { id, .. }
            | WidgetKind::IconButton { id, .. }
            | WidgetKind::Checkbox { id, .. }
            | WidgetKind::Toggle { id, .. }
            | WidgetKind::Slider { id, .. }
            | WidgetKind::TextInput { id, .. }
            | WidgetKind::TextArea { id, .. }
            | WidgetKind::PasswordInput { id, .. }
            | WidgetKind::NumberInput {
                id, enabled: true, ..
            }
            | WidgetKind::RadioButton { id, .. }
            | WidgetKind::Dropdown { id, .. }
            | WidgetKind::AccordionHeader { id, .. }
            | WidgetKind::ColorSwatch { id, .. }
            | WidgetKind::ColorPicker { id, .. }
            | WidgetKind::Toast { id, .. }
            | WidgetKind::VideoPlayer { id, .. }
            | WidgetKind::Media { id, .. }
            | WidgetKind::CustomPaint { id, .. } => Some(InteractionKey {
                widget_id: id.clone(),
                index: None,
            }),
            WidgetKind::MenuItem { id, .. } | WidgetKind::TreeNode { id, .. } => {
                Some(InteractionKey {
                    widget_id: id.clone(),
                    index: None,
                })
            }
            WidgetKind::WindowCloseButton { owner }
            | WidgetKind::ModalBackdrop { owner }
            | WidgetKind::Splitter { owner, .. }
            | WidgetKind::PaletteHeader { owner, .. }
            | WidgetKind::PaletteFoldButton { owner, .. }
            | WidgetKind::PaletteCloseButton { owner, .. }
            | WidgetKind::ResizeGrip { owner, .. } => Some(InteractionKey {
                widget_id: owner.clone(),
                index: None,
            }),
            WidgetKind::Scrollbar { id, .. } => Some(InteractionKey {
                widget_id: id.clone(),
                index: None,
            }),
            WidgetKind::TabItem { owner, index, .. }
            | WidgetKind::ListItem { owner, index, .. }
            | WidgetKind::SegmentItem { owner, index, .. }
            | WidgetKind::MenuBarItem { owner, index, .. }
            | WidgetKind::BreadcrumbItem { owner, index, .. }
            | WidgetKind::PaginationItem {
                owner, page: index, ..
            }
            | WidgetKind::TableHeader {
                owner,
                column_index: index,
                ..
            } => Some(InteractionKey {
                widget_id: owner.clone(),
                index: Some(*index),
            }),
            WidgetKind::TableCell {
                owner, row_index, ..
            } => Some(InteractionKey {
                widget_id: owner.clone(),
                index: Some(*row_index),
            }),
            WidgetKind::Container
            | WidgetKind::Icon { .. }
            | WidgetKind::MenuBar
            | WidgetKind::MenuPopover
            | WidgetKind::Label { .. }
            | WidgetKind::Badge { .. }
            | WidgetKind::Window { .. }
            | WidgetKind::Modal { .. }
            | WidgetKind::Palette { .. }
            | WidgetKind::Divider { .. }
            | WidgetKind::ProgressBar { .. }
            | WidgetKind::MetricCard { .. }
            | WidgetKind::ScrollView { .. }
            | WidgetKind::Tooltip { .. }
            | WidgetKind::NumberInput { enabled: false, .. }
            | WidgetKind::AudioVisualizer { .. } => None,
        }
    }
}
