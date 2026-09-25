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

/// Visual presentation variant for buttons and interactive controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Default solid cyber-glass button with distinct background and border
    #[default]
    Default,
    /// Glowing cyan/green primary call-to-action button (e.g. Play, Mode Débutant, Active Tab)
    Primary,
    /// Subtle translucent tool/menu button with subtle border
    Ghost,
    /// Danger / destructive action button (e.g. Stop, Delete)
    Danger,
    /// Accent secondary / purple cyber button (e.g. Mode Avancé, Shaders)
    Secondary,
}

/// Placement direction for floating tooltips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipPlacement {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
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
        variant: ButtonVariant,
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
        shortcut: Option<String>,
        placement: TooltipPlacement,
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
    /// Rotary potentiometer knob for precision continuous parameter tuning.
    Knob {
        id: WidgetId,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        label: Option<String>,
        unit: Option<String>,
    },
    /// High-performance GPU multi-series data chart (Area / Line / Sparkline).
    TimeSeriesChart {
        id: WidgetId,
        title: Option<String>,
        series: Vec<ChartSeries>,
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
        show_grid: bool,
        show_legend: bool,
        inspected_point: Option<(usize, usize)>,
    },
    /// Interactive visual Node Graph canvas with vector Bézier connections, typed sockets, and movable nodes.
    NodeGraph {
        id: WidgetId,
        nodes: Vec<GraphNodeSpec>,
        connections: Vec<GraphConnectionSpec>,
        pan: [f32; 2],
        zoom: f32,
        connecting_from: Option<(String, usize)>,
    },
    /// Multi-tag input chip selector with removal buttons.
    TagInput {
        id: WidgetId,
        tags: Vec<String>,
        placeholder: String,
        active_tag: Option<usize>,
    },
    /// Syntax-highlighted code editor with line numbering and syntax tokens.
    CodeEditor {
        id: WidgetId,
        text: String,
        language: String,
        line_numbers: bool,
        focused: bool,
        cursor: usize,
        selection: Option<(usize, usize)>,
    },
    /// High-performance GPU Bar Chart / Histogram.
    BarChart {
        id: WidgetId,
        title: Option<String>,
        bars: Vec<BarItem>,
        max_value: Option<f32>,
        horizontal: bool,
    },
    /// Circular radial progress meter / gauge indicator.
    RadialMeter {
        id: WidgetId,
        label: Option<String>,
        value: f32,
        min: f32,
        max: f32,
        unit: Option<String>,
        color: Option<[f32; 4]>,
    },
    /// Generic Drag & Drop target area with dashed neon border, hover glow, and file details.
    DropZone {
        id: WidgetId,
        label: String,
        hint: Option<String>,
        accepted_extensions: Vec<String>,
        hovered: bool,
        dropped_file: Option<(String, u64)>,
    },
    /// Circular User Avatar with image or initials monogram, neon ring, and presence status.
    Avatar {
        id: WidgetId,
        resource_id: Option<String>,
        initials: Option<String>,
        status: AvatarStatus,
        size: f32,
        ring_color: Option<[f32; 4]>,
        glow: bool,
    },
    /// Linear step sequence progress indicator.
    Stepper {
        id: WidgetId,
        steps: Vec<StepItem>,
        current_step: usize,
    },
    /// Beveled cyber keyboard key badge.
    Kbd {
        text: String,
    },
    /// Interactive filter/tag chip with optional icon, dismiss button, and selection state.
    Chip {
        id: WidgetId,
        label: String,
        icon: Option<IconKind>,
        color_pip: Option<[f32; 4]>,
        selected: bool,
        dismissible: bool,
        variant: ChipVariant,
    },
    /// Shimmering loading placeholder skeleton.
    Skeleton {
        radius: Option<f32>,
        shimmer: bool,
    },
    /// Multi-segment proportional progress bar.
    MultiProgressBar {
        id: WidgetId,
        segments: Vec<ProgressSegment>,
        show_labels: bool,
    },
    /// Star / Heart / Diamond rating widget.
    Rating {
        id: WidgetId,
        value: u8,
        max: u8,
        glyph: RatingGlyph,
        readonly: bool,
    },
    /// Vertical chronological event timeline.
    Timeline {
        id: WidgetId,
        items: Vec<TimelineItem>,
    },
    Card {
        bg: Option<[f32; 4]>,
        border: Option<[f32; 4]>,
        radius: Option<f32>,
    },
    Panel {
        bg: Option<[f32; 4]>,
        border: Option<[f32; 4]>,
    },
}

/// State of an individual step in a Stepper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepState {
    #[default]
    Pending,
    Active,
    Completed,
    Error,
}

/// Single step descriptor for a Stepper workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct StepItem {
    pub label: String,
    pub description: Option<String>,
    pub state: StepState,
}

/// Visual style variant for a Chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipVariant {
    #[default]
    Default,
    Primary,
    Outline,
    Success,
    Warning,
    Danger,
}

/// Single segment in a MultiProgressBar.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSegment {
    pub label: String,
    pub value: f32,
    pub color: [f32; 4],
}

/// Visual shape glyph for a Rating widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RatingGlyph {
    #[default]
    Star,
    Heart,
    Diamond,
}

impl RatingGlyph {
    pub fn glyph(&self, filled: bool) -> &'static str {
        match self {
            RatingGlyph::Star => if filled { "★" } else { "☆" },
            RatingGlyph::Heart => if filled { "♥" } else { "♡" },
            RatingGlyph::Diamond => if filled { "◆" } else { "◇" },
        }
    }
}

/// Status / severity for a Timeline event node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineStatus {
    #[default]
    Default,
    Success,
    Warning,
    Danger,
    Active,
}

impl TimelineStatus {
    pub fn color(&self) -> [f32; 4] {
        match self {
            TimelineStatus::Default => [0.0, 0.85, 1.0, 1.0], // Cyan
            TimelineStatus::Success => [0.15, 0.92, 0.45, 1.0], // Emerald
            TimelineStatus::Warning => [1.0, 0.78, 0.12, 1.0], // Amber
            TimelineStatus::Danger => [1.0, 0.28, 0.32, 1.0], // Ruby
            TimelineStatus::Active => [0.75, 0.35, 0.95, 1.0], // Neon Violet
        }
    }
}

/// Single event item in a vertical Timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItem {
    pub time: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TimelineStatus,
}

/// Presence and activity status indicator for user avatars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarStatus {
    #[default]
    None,
    Online,
    Away,
    Busy,
    Offline,
}

impl AvatarStatus {
    pub fn color(&self) -> Option<[f32; 4]> {
        match self {
            AvatarStatus::None => None,
            AvatarStatus::Online => Some([0.15, 0.92, 0.45, 1.0]), // Vibrant neon green
            AvatarStatus::Away => Some([1.0, 0.78, 0.12, 1.0]),   // Amber gold
            AvatarStatus::Busy => Some([1.0, 0.25, 0.30, 1.0]),   // Ruby red
            AvatarStatus::Offline => Some([0.50, 0.55, 0.65, 1.0]), // Slate gray
        }
    }

    pub fn indicator_color(&self) -> Option<[f32; 4]> {
        self.color()
    }
}

/// Single bar entry for the GPU BarChart.
#[derive(Debug, Clone, PartialEq)]
pub struct BarItem {
    pub label: String,
    pub value: f32,
    pub color: [f32; 4],
}

/// Type classification for node graph sockets, governing color coding and compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketType {
    Signal,
    Data,
    Texture,
    Flow,
    Custom(String),
}

impl SocketType {
    pub fn default_color(&self) -> [f32; 4] {
        match self {
            SocketType::Signal => [0.0, 0.85, 1.0, 1.0],     // Cyan
            SocketType::Data => [0.2, 0.8, 0.4, 1.0],       // Green
            SocketType::Texture => [0.95, 0.6, 0.1, 1.0],    // Amber
            SocketType::Flow => [0.65, 0.35, 0.95, 1.0],    // Violet
            SocketType::Custom(_) => [0.8, 0.8, 0.9, 1.0],  // Light Slate
        }
    }
}

/// Socket descriptor for inputs or outputs on a node graph element.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphSocket {
    pub name: String,
    pub socket_type: SocketType,
    pub color: Option<[f32; 4]>,
    pub is_output: bool,
}

/// Full specification of an interactive node placed on a node graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNodeSpec {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub inputs: Vec<GraphSocket>,
    pub outputs: Vec<GraphSocket>,
    pub header_color: Option<[f32; 4]>,
    pub selected: bool,
}

/// Connection link between two sockets on a node graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphConnectionSpec {
    pub from_node: String,
    pub from_socket: usize,
    pub to_node: String,
    pub to_socket: usize,
    pub color: Option<[f32; 4]>,
    pub flow_active: bool,
}

/// Data series for the GPU TimeSeriesChart.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    pub color: [f32; 4],
    pub points: Vec<[f32; 2]>,
    pub filled: bool,
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
                | WidgetKind::CustomPaint { .. }
                | WidgetKind::Knob { .. }
                | WidgetKind::TimeSeriesChart { .. }
                | WidgetKind::NodeGraph { .. }
                | WidgetKind::TagInput { .. }
                | WidgetKind::CodeEditor { .. }
                | WidgetKind::BarChart { .. }
                | WidgetKind::RadialMeter { .. }
                | WidgetKind::DropZone { .. }
                | WidgetKind::Avatar { .. }
                | WidgetKind::Stepper { .. }
                | WidgetKind::Chip { .. }
                | WidgetKind::Rating { .. }
                | WidgetKind::Timeline { .. }
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
            | WidgetKind::CustomPaint { id, .. }
            | WidgetKind::Knob { id, .. }
            | WidgetKind::TimeSeriesChart { id, .. }
            | WidgetKind::NodeGraph { id, .. }
            | WidgetKind::TagInput { id, .. }
            | WidgetKind::CodeEditor { id, .. }
            | WidgetKind::BarChart { id, .. }
            | WidgetKind::RadialMeter { id, .. }
            | WidgetKind::DropZone { id, .. }
            | WidgetKind::Avatar { id, .. }
            | WidgetKind::Stepper { id, .. }
            | WidgetKind::Chip { id, .. }
            | WidgetKind::Rating { id, .. }
            | WidgetKind::Timeline { id, .. } => Some(InteractionKey {
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
            | WidgetKind::Card { .. }
            | WidgetKind::Panel { .. }
            | WidgetKind::AudioVisualizer { .. }
            | WidgetKind::Kbd { .. }
            | WidgetKind::Skeleton { .. }
            | WidgetKind::MultiProgressBar { .. } => None,
        }
    }
}
