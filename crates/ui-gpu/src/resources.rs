// [WFGY] Zone: TRANSIT | delta_s: 0.4 | κ: Low | η: Low | Action: Table de ressources opaques pour les futurs pipelines de media (image/video/3D)
use std::any::Any;
use std::collections::HashMap;

/// Table de ressources tenues par l'application, indexées par le
/// `resource_id` opaque porté par un `MediaSpec` côté `ui-widgets`.
///
/// `ui-gpu` ne connaît aucun type de ressource concret (texture décodée
/// depuis un PNG, frame vidéo, scène 3D...) : chaque [`crate::MediaPipeline`]
/// sait quel type il attend et le récupère lui-même via `get::<T>`.
#[derive(Default)]
pub struct ResourceTable {
    entries: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ResourceTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, id: impl Into<String>, value: T) {
        self.entries.insert(id.into(), Box::new(value));
    }

    /// `None` si l'id est absent OU si le type stocké ne correspond pas à
    /// `T` (erreur de programmation d'un pipeline, pas une panique).
    pub fn get<T: Any + Send + Sync>(&self, id: &str) -> Option<&T> {
        self.entries.get(id).and_then(|boxed| boxed.downcast_ref::<T>())
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_typed_value() {
        let mut table = ResourceTable::new();
        table.insert("cover", 42u32);
        assert_eq!(table.get::<u32>("cover"), Some(&42));
    }

    #[test]
    fn wrong_type_yields_none_instead_of_panicking() {
        let mut table = ResourceTable::new();
        table.insert("cover", 42u32);
        assert_eq!(table.get::<String>("cover"), None);
    }

    #[test]
    fn missing_id_yields_none() {
        let table = ResourceTable::new();
        assert_eq!(table.get::<u32>("does-not-exist"), None);
    }
}
