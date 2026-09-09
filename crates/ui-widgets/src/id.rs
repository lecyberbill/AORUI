// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Semantic stable widget identifier decoupled from ephemeral Taffy NodeId
/// Stable semantic identifier for a widget (e.g. `"submit_button"`), chosen
/// by the application — unlike `taffy::NodeId`, which changes across tree reconstructions.
/// This `WidgetId` is emitted in [`ui_core::UiEvent`] during interaction dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub String);

impl WidgetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for WidgetId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for WidgetId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for WidgetId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for WidgetId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for WidgetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WidgetId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
