use crate::config::APP_ID;
use crate::fl;
use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{
    CosmicConfigEntry, {self},
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::vec;

static HOME: LazyLock<AppGroup> = LazyLock::new(|| AppGroup {
    name: "cosmic-library-home".to_string(),
    icon: "user-home-symbolic".to_string(),
    filter: FilterType::None,
    layout: Vec::new(),
});

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FilterType {
    /// A list of application IDs to include in the group.
    AppIds(Vec<String>),
    Categories {
        categories: Vec<String>,
        /// The ID of applications which may not match the categories, but should be included anyway.
        exclude: Vec<String>,
        /// The ID of applications which should be excluded from the results.
        include: Vec<String>,
    },
    /// No filter is applied.
    /// This is intended for use with Home.
    None,
}

impl Default for FilterType {
    fn default() -> Self {
        FilterType::AppIds(Vec::new())
    }
}

impl Ord for FilterType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (FilterType::AppIds(_), FilterType::AppIds(_)) => std::cmp::Ordering::Equal,
            (FilterType::None, FilterType::None) => std::cmp::Ordering::Equal,
            (FilterType::Categories { .. }, FilterType::Categories { .. }) => {
                std::cmp::Ordering::Equal
            }
            (FilterType::Categories { .. } | FilterType::None, FilterType::AppIds(_)) => {
                std::cmp::Ordering::Less
            }
            (FilterType::AppIds(_), FilterType::Categories { .. } | FilterType::None) => {
                std::cmp::Ordering::Greater
            }
            (FilterType::Categories { .. }, FilterType::None) => std::cmp::Ordering::Greater,
            (FilterType::None, FilterType::Categories { .. }) => std::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for FilterType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Object holding the state
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppGroup {
    pub name: String,
    pub icon: String,
    pub filter: FilterType,
    /// App IDs in user-chosen tile slots. Empty entries are intentional gaps.
    #[serde(default)]
    pub layout: Vec<Option<String>>,
    // pub popup: bool,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LaunchpadPage {
    pub name: String,
    /// App IDs in user-chosen tile slots. Empty entries are intentional gaps.
    #[serde(default)]
    pub layout: Vec<Option<String>>,
}

impl PartialOrd for AppGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AppGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self.filter, &other.filter) {
            (FilterType::AppIds(_), FilterType::AppIds(_)) => {
                self.name.to_lowercase().cmp(&other.name.to_lowercase())
            }
            (FilterType::Categories { categories, .. }, FilterType::AppIds(_)) => {
                if let Some(cat_name) = categories.first() {
                    cat_name.to_lowercase().cmp(&other.name.to_lowercase())
                } else {
                    self.name.to_lowercase().cmp(&other.name.to_lowercase())
                }
            }
            (FilterType::AppIds(_), FilterType::Categories { categories, .. }) => {
                if let Some(other_name) = categories.first() {
                    self.name.to_lowercase().cmp(&other_name.to_lowercase())
                } else {
                    self.name.to_lowercase().cmp(&other.name.to_lowercase())
                }
            }
            (a, b) => a.cmp(b),
        }
    }
}

impl AppGroup {
    pub fn name(&self) -> String {
        if &self.name == "cosmic-library-home" {
            fl!("cosmic-library-home")
        } else if &self.name == "cosmic-office" {
            fl!("cosmic-office")
        } else if &self.name == "cosmic-system" {
            fl!("cosmic-system")
        } else if &self.name == "cosmic-utilities" {
            fl!("cosmic-utilities")
        } else {
            self.name.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry)]
pub struct AppLibraryConfig {
    pub(crate) groups: Vec<AppGroup>,
    /// Icon size in logical pixels. The surrounding tile scales with it.
    #[serde(default = "default_icon_size")]
    pub(crate) icon_size: u16,
    /// Persistent spatial layout for apps that are not in a named group.
    #[serde(default)]
    pub(crate) home_layout: Vec<Option<String>>,
    /// Additional spatial Launchpad pages. `home_layout` remains the first page
    /// so existing installations upgrade without losing their arrangement.
    #[serde(default)]
    pub(crate) launchpad_pages: Vec<LaunchpadPage>,
    #[serde(default = "default_home_page_name")]
    pub(crate) home_page_name: String,
    /// Apps intentionally omitted from Other Apps. They remain searchable.
    #[serde(default)]
    pub(crate) hidden_apps: Vec<String>,
    #[serde(default = "default_board_width")]
    pub(crate) board_width: u16,
    #[serde(default = "default_board_height")]
    pub(crate) board_height: u16,
    #[serde(default = "default_tile_gap")]
    pub(crate) tile_gap: u16,
    #[serde(default = "default_true")]
    pub(crate) show_labels: bool,
    #[serde(default = "default_true")]
    pub(crate) show_tooltips: bool,
    #[serde(default = "default_true")]
    pub(crate) kinetic_scrolling: bool,
    /// Percentage of the final finger-driven movement carried into the coast.
    #[serde(default = "default_momentum_strength")]
    pub(crate) momentum_strength: u16,
    /// Percentage of the standard coast duration.
    #[serde(default = "default_scroll_glide")]
    pub(crate) scroll_glide: u16,
    /// Delay before the app infers that the fingers have lifted from the trackpad.
    #[serde(default = "default_release_delay")]
    pub(crate) release_delay_ms: u16,
}

const fn default_icon_size() -> u16 {
    72
}
fn default_home_page_name() -> String {
    "Launchpad".to_string()
}

const fn default_board_width() -> u16 {
    1200
}
const fn default_board_height() -> u16 {
    690
}
const fn default_tile_gap() -> u16 {
    8
}
const fn default_true() -> bool {
    true
}
const fn default_momentum_strength() -> u16 {
    300
}
const fn default_scroll_glide() -> u16 {
    260
}
const fn default_release_delay() -> u16 {
    4
}

impl AppLibraryConfig {
    pub fn version() -> u64 {
        1
    }

    pub fn helper() -> Option<cosmic_config::Config> {
        cosmic_config::Config::new(APP_ID, Self::version()).ok()
    }

    pub fn home() -> &'static AppGroup {
        &HOME
    }

    pub fn page_count(&self) -> usize {
        1 + self.launchpad_pages.len()
    }

    pub fn page_name(&self, page: usize) -> &str {
        if page == 0 {
            &self.home_page_name
        } else {
            self.launchpad_pages
                .get(page - 1)
                .map_or("Launchpad", |page| page.name.as_str())
        }
    }

    pub fn set_page_name(&mut self, page: usize, name: String) {
        if page == 0 {
            self.home_page_name = name;
        } else if let Some(page) = self.launchpad_pages.get_mut(page - 1) {
            page.name = name;
        }
    }

    pub fn page_layout(&self, page: usize) -> &[Option<String>] {
        if page == 0 {
            &self.home_layout
        } else {
            self.launchpad_pages
                .get(page - 1)
                .map_or(&[], |page| page.layout.as_slice())
        }
    }

    fn page_layout_mut(&mut self, page: usize) -> Option<&mut Vec<Option<String>>> {
        if page == 0 {
            Some(&mut self.home_layout)
        } else {
            self.launchpad_pages
                .get_mut(page - 1)
                .map(|page| &mut page.layout)
        }
    }

    pub fn add_page(&mut self) -> usize {
        let page_number = self.page_count() + 1;
        self.launchpad_pages.push(LaunchpadPage {
            name: format!("Page {page_number}"),
            layout: Vec::new(),
        });
        self.page_count() - 1
    }

    pub fn remove_empty_page(&mut self, page: usize) -> bool {
        if page == 0 {
            return false;
        }
        let Some(candidate) = self.launchpad_pages.get(page - 1) else {
            return false;
        };
        if candidate.layout.iter().any(Option::is_some) {
            return false;
        }
        self.launchpad_pages.remove(page - 1);
        true
    }

    pub fn place_entry_on_page(&mut self, page: usize, slot: usize, id: &str) {
        let old_slot = self
            .page_layout(page)
            .iter()
            .position(|entry| entry.as_deref() == Some(id));
        let displaced = self.page_layout(page).get(slot).cloned().flatten();

        self.home_layout
            .iter_mut()
            .chain(
                self.launchpad_pages
                    .iter_mut()
                    .flat_map(|page| page.layout.iter_mut()),
            )
            .for_each(|entry| {
                if entry.as_deref() == Some(id) {
                    *entry = None;
                }
            });

        let Some(layout) = self.page_layout_mut(page) else {
            return;
        };
        if layout.len() <= slot {
            layout.resize(slot + 1, None);
        }
        layout[slot] = Some(id.to_string());

        if let Some(displaced) = displaced.filter(|displaced| displaced != id) {
            if let Some(old_slot) = old_slot.filter(|old_slot| *old_slot != slot) {
                if layout.len() <= old_slot {
                    layout.resize(old_slot + 1, None);
                }
                layout[old_slot] = Some(displaced);
            } else if let Some(empty_slot) = layout.iter().position(Option::is_none) {
                layout[empty_slot] = Some(displaced);
            } else {
                layout.push(Some(displaced));
            }
        }
        while layout.last().is_some_and(Option::is_none) {
            layout.pop();
        }
    }

    pub fn remove_entry_from_page(&mut self, page: usize, id: &str) {
        let Some(layout) = self.page_layout_mut(page) else {
            return;
        };
        for entry in layout.iter_mut() {
            if entry.as_deref() == Some(id) {
                *entry = None;
            }
        }
        while layout.last().is_some_and(Option::is_none) {
            layout.pop();
        }
    }

    pub fn pinned_ids(&self) -> impl Iterator<Item = &str> {
        self.home_layout
            .iter()
            .chain(
                self.launchpad_pages
                    .iter()
                    .flat_map(|page| page.layout.iter()),
            )
            .filter_map(Option::as_deref)
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden_apps.iter().any(|hidden| hidden == id)
    }

    pub fn set_hidden(&mut self, id: &str, hidden: bool) {
        self.hidden_apps.retain(|candidate| candidate != id);
        if hidden {
            self.hidden_apps.push(id.to_string());
        }
    }

    pub fn add(&mut self, name: String) {
        self.groups.push(AppGroup {
            name,
            icon: "folder-symbolic".to_string(),
            filter: FilterType::AppIds(Vec::new()),
            layout: Vec::new(),
        });
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.groups.len() {
            self.groups.remove(i);
        }
    }

    pub fn layout(&self, group: Option<usize>) -> &[Option<String>] {
        group
            .and_then(|i| self.groups.get(i))
            .map_or(self.home_layout.as_slice(), |group| group.layout.as_slice())
    }

    fn layout_mut(&mut self, group: Option<usize>) -> &mut Vec<Option<String>> {
        if let Some(group) = group.and_then(|i| self.groups.get_mut(i)) {
            &mut group.layout
        } else {
            &mut self.home_layout
        }
    }

    pub fn set_layout(&mut self, group: Option<usize>, mut layout: Vec<Option<String>>) {
        while layout.last().is_some_and(Option::is_none) {
            layout.pop();
        }
        *self.layout_mut(group) = layout;
    }

    pub fn place_entry(&mut self, group: Option<usize>, slot: usize, id: &str) {
        let old_position = std::iter::once((None, &self.home_layout))
            .chain(
                self.groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| (Some(index), &group.layout)),
            )
            .find_map(|(group, layout)| {
                layout
                    .iter()
                    .position(|entry| entry.as_deref() == Some(id))
                    .map(|slot| (group, slot))
            });

        let displaced = self.layout(group).get(slot).cloned().flatten();
        self.home_layout
            .iter_mut()
            .chain(
                self.groups
                    .iter_mut()
                    .flat_map(|group| group.layout.iter_mut()),
            )
            .for_each(|entry| {
                if entry.as_deref() == Some(id) {
                    *entry = None;
                }
            });

        let layout = self.layout_mut(group);
        if layout.len() <= slot {
            layout.resize(slot + 1, None);
        }
        layout[slot] = Some(id.to_string());

        if let Some(displaced) = displaced.filter(|displaced| displaced != id) {
            if let Some((old_group, old_slot)) = old_position
                .filter(|(old_group, old_slot)| *old_group == group && *old_slot != slot)
            {
                debug_assert_eq!(old_group, group);
                if layout.len() <= old_slot {
                    layout.resize(old_slot + 1, None);
                }
                layout[old_slot] = Some(displaced);
            } else if let Some(empty_slot) = layout.iter().position(Option::is_none) {
                layout[empty_slot] = Some(displaced);
            } else {
                layout.push(Some(displaced));
            }
        }
        while layout.last().is_some_and(Option::is_none) {
            layout.pop();
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn clear_layout_entry(&mut self, id: &str) {
        self.home_layout
            .iter_mut()
            .chain(
                self.groups
                    .iter_mut()
                    .flat_map(|group| group.layout.iter_mut()),
            )
            .for_each(|entry| {
                if entry.as_deref() == Some(id) {
                    *entry = None;
                }
            });
        while self.home_layout.last().is_some_and(Option::is_none) {
            self.home_layout.pop();
        }
        for group in &mut self.groups {
            while group.layout.last().is_some_and(Option::is_none) {
                group.layout.pop();
            }
        }
    }

    pub fn set_name(&mut self, i: usize, name: String) {
        if let Some(group) = self.groups.get_mut(i) {
            group.name = name;
        }
    }

    pub fn remove_entry(&mut self, group: Option<usize>, id: &str) {
        let Some(group) = group.and_then(|i| self.groups.get_mut(i)) else {
            return;
        };
        match &mut group.filter {
            FilterType::AppIds(ids) => ids.retain(|conf_id| conf_id != id),
            FilterType::Categories {
                exclude, include, ..
            } => {
                include.retain(|conf_id| conf_id != id);
                exclude.retain(|conf_id| conf_id != id);
                exclude.push(id.to_string());
            }
            FilterType::None => {}
        }
    }

    pub fn add_entry(&mut self, group: Option<usize>, id: &str) {
        if let Some(group) = group.and_then(|i| self.groups.get_mut(i)) {
            match &mut group.filter {
                FilterType::AppIds(ids) => {
                    if ids.iter().all(|s| s != id) {
                        ids.push(id.to_string());
                    }
                }
                FilterType::Categories {
                    exclude, include, ..
                } => {
                    include.retain(|conf_id| conf_id != id);
                    exclude.retain(|conf_id| conf_id != id);
                    include.push(id.to_string());
                }
                FilterType::None => {}
            }
        } else {
            for group in &mut self.groups {
                match &mut group.filter {
                    FilterType::AppIds(ids) => {
                        ids.retain(|conf_id| conf_id != id);
                    }
                    FilterType::Categories {
                        exclude, include, ..
                    } => {
                        include.retain(|conf_id| conf_id != id);
                        if exclude.iter().all(|conf_id| conf_id != id) {
                            exclude.push(id.to_string());
                        }
                    }
                    FilterType::None => {}
                }
            }
        }
    }
}

impl Default for AppLibraryConfig {
    fn default() -> Self {
        AppLibraryConfig {
            icon_size: default_icon_size(),
            home_layout: Vec::new(),
            launchpad_pages: Vec::new(),
            home_page_name: default_home_page_name(),
            hidden_apps: Vec::new(),
            board_width: default_board_width(),
            board_height: default_board_height(),
            tile_gap: default_tile_gap(),
            show_labels: true,
            show_tooltips: true,
            kinetic_scrolling: true,
            momentum_strength: default_momentum_strength(),
            scroll_glide: default_scroll_glide(),
            release_delay_ms: default_release_delay(),
            groups: vec![
                AppGroup {
                    name: "cosmic-office".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["Office".to_string()],
                        include: vec![
                            "org.gnome.Totem".to_string(),
                            "org.gnome.eog".to_string(),
                            "simple-scan".to_string(),
                            "thunderbird".to_string(),
                        ],
                        exclude: Vec::new(),
                    },
                    layout: Vec::new(),
                },
                AppGroup {
                    name: "cosmic-system".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["System".to_string()],
                        include: vec![
                            "gnome-language-selector".to_string(),
                            "im-config".to_string(),
                            "org.freedesktop.IBus.Setup".to_string(),
                            "system76-driver".to_string(),
                        ],
                        exclude: vec![
                            "com.system76.CosmicStore".to_string(),
                            "com.system76.CosmicTerm".to_string(),
                        ],
                    },
                    layout: Vec::new(),
                },
                AppGroup {
                    name: "cosmic-utilities".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["Utility".to_string()],
                        include: vec!["nm-connection-editor".to_string()],
                        exclude: vec![
                            "com.system76.CosmicEdit".to_string(),
                            "com.system76.CosmicFiles".to_string(),
                        ],
                    },
                    layout: Vec::new(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppLibraryConfig;

    #[test]
    fn spatial_slot_is_preserved_and_moves_cleanly() {
        let mut config = AppLibraryConfig::default();
        config.place_entry(None, 5, "test.app");
        assert_eq!(config.home_layout.len(), 6);
        assert_eq!(config.home_layout[5].as_deref(), Some("test.app"));

        config.place_entry(None, 2, "test.app");
        assert_eq!(config.home_layout[2].as_deref(), Some("test.app"));
        assert!(config.home_layout.get(5).is_none());
    }

    #[test]
    fn moving_between_groups_clears_the_old_slot() {
        let mut config = AppLibraryConfig::default();
        config.place_entry(Some(0), 4, "test.app");
        config.place_entry(Some(1), 7, "test.app");

        assert!(config.groups[0].layout[4].is_none());
        assert_eq!(config.groups[1].layout[7].as_deref(), Some("test.app"));
    }

    #[test]
    fn moving_one_tile_does_not_reflow_the_others() {
        let mut config = AppLibraryConfig::default();
        config.set_layout(
            None,
            vec![Some("one".into()), Some("two".into()), Some("three".into())],
        );

        config.place_entry(None, 5, "one");
        assert_eq!(config.home_layout[1].as_deref(), Some("two"));
        assert_eq!(config.home_layout[2].as_deref(), Some("three"));
        assert_eq!(config.home_layout[5].as_deref(), Some("one"));
    }

    #[test]
    fn dropping_on_an_occupied_slot_swaps_tiles() {
        let mut config = AppLibraryConfig::default();
        config.set_layout(
            None,
            vec![Some("one".into()), Some("two".into()), Some("three".into())],
        );

        config.place_entry(None, 2, "one");
        assert_eq!(config.home_layout[0].as_deref(), Some("three"));
        assert_eq!(config.home_layout[1].as_deref(), Some("two"));
        assert_eq!(config.home_layout[2].as_deref(), Some("one"));
    }

    #[test]
    fn favorite_can_be_added_and_removed() {
        let mut config = AppLibraryConfig::default();
        config.place_entry(None, 0, "favorite.app");
        assert_eq!(config.home_layout[0].as_deref(), Some("favorite.app"));

        config.clear_layout_entry("favorite.app");
        assert!(config.home_layout.is_empty());
    }

    #[test]
    fn apps_move_between_launchpad_pages_without_duplicates() {
        let mut config = AppLibraryConfig::default();
        config.place_entry_on_page(0, 3, "favorite.app");
        let second_page = config.add_page();
        config.place_entry_on_page(second_page, 1, "favorite.app");

        assert!(!config.page_layout(0).iter().any(Option::is_some));
        assert_eq!(
            config.page_layout(second_page)[1].as_deref(),
            Some("favorite.app")
        );
        assert_eq!(config.pinned_ids().count(), 1);
    }

    #[test]
    fn only_empty_extra_pages_can_be_removed() {
        let mut config = AppLibraryConfig::default();
        let page = config.add_page();
        config.place_entry_on_page(page, 0, "favorite.app");
        assert!(!config.remove_empty_page(page));

        config.remove_entry_from_page(page, "favorite.app");
        assert!(config.remove_empty_page(page));
        assert_eq!(config.page_count(), 1);
    }

    #[test]
    fn hidden_apps_can_be_restored() {
        let mut config = AppLibraryConfig::default();
        config.set_hidden("utility.app", true);
        assert!(config.is_hidden("utility.app"));

        config.set_hidden("utility.app", false);
        assert!(!config.is_hidden("utility.app"));
    }
}
