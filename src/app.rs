use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use clap::Parser;
use cosmic::iced::event::wayland::OutputEvent;
use cosmic::iced::platform_specific::shell::commands::layer_surface::set_padding;
use cosmic::iced::runtime::platform_specific::wayland::CornerRadius;
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::IcedMargin;
use cosmic::iced::runtime::{Action, platform_specific, task};
use cosmic::iced::window;
use cosmic::surface::action::{LiveSettings, app_layer_shell, simple_layer_shell, simple_popup};
use cosmic::widget::menu::menu_column::MenuColumn;
use cosmic::widget::reorderable_flex_row;
use cosmic::widget::space::horizontal;
use cosmic::{
    Element,
    app::{Core, CosmicFlags, Settings, Task},
    cctk::sctk::{
        self,
        shell::wlr_layer::{Anchor, KeyboardInteractivity},
    },
    cosmic_config::{Config, ConfigGet, ConfigSet, CosmicConfigEntry},
    cosmic_theme::Spacing,
    dbus_activation,
    desktop::{DesktopEntryData, IconSourceExt, fde::PathSource, load_desktop_file},
    iced::{
        self, Alignment, Color, Length, Limits, Size, Subscription,
        event::{listen_with, wayland::OverlapNotifyEvent},
        executor,
        id::Id,
        /*wayland::actions::{
            data_device::ActionInner,
        },*/
        widget::{
            column, container, mouse_area, row,
            rule::horizontal as horizontal_rule,
            scrollable::{AbsoluteOffset, RelativeOffset},
        },
        window::Event as WindowEvent,
    },
    iced::{
        core::{
            Border, Padding, Rectangle, Shadow,
            alignment::Vertical,
            event::{
                PlatformSpecific,
                wayland::{self, LayerEvent},
            },
            keyboard::{Key, key::Named},
            widget::operation::{
                self,
                focusable::{find_focused, focus},
            },
            window::Id as SurfaceId,
        },
        platform_specific::shell::wayland::commands::{
            self,
            activation::request_token,
            layer_surface::{destroy_layer_surface, get_layer_surface},
            overlap_notify::overlap_notify,
            popup::destroy_popup,
        },
        runtime::{
            self as iced_runtime,
            dnd::end_dnd,
            platform_specific::wayland::{
                layer_surface::SctkLayerSurfaceSettings,
                popup::{SctkPopupSettings, SctkPositioner},
            },
        },
        widget::stack,
    },
    keyboard_nav,
    theme::{self, Button, TextInput},
    widget::{
        self,
        autosize::autosize,
        button::{self, Catalog as ButtonStyleSheet},
        divider,
        dnd_destination::dnd_destination_for_data,
        icon::{self, from_name},
        scrollable, search_input, space, svg, text, text_input, tooltip,
    },
};
use cosmic_app_list_config::AppListConfig;
use cosmic_settings_config::shortcuts::{
    self, Action as ShortcutAction, Binding as ShortcutBinding, Modifier as ShortcutModifier,
    Shortcuts,
};
use itertools::Itertools;
use log::error;
use sctk::shell::wlr_layer;
use serde::{Deserialize, Serialize};
use switcheroo_control::Gpu;

use crate::app_group::{AppGroup, AppLibraryConfig};
use crate::fl;
use crate::subscriptions::desktop_files::desktop_files;
use crate::widgets::application::{AppletString, ApplicationButton};

// popovers should show options, but also the desktop info options
// should be a way to add apps to groups
// should be a way to remove apps from groups

static SEARCH_ID: LazyLock<Id> = LazyLock::new(|| Id::new("search"));
static EDIT_GROUP_ID: LazyLock<Id> = LazyLock::new(|| Id::new("edit_group"));
static NEW_GROUP_ID: LazyLock<Id> = LazyLock::new(|| Id::new("new_group"));
static SUBMIT_DELETE_ID: LazyLock<Id> = LazyLock::new(|| Id::new("cancel_delete"));

static CREATE_NEW: LazyLock<String> = LazyLock::new(|| fl!("create-new"));
static ADD_GROUP: LazyLock<String> = LazyLock::new(|| fl!("add-group"));
static SEARCH_PLACEHOLDER: LazyLock<String> = LazyLock::new(|| fl!("search-placeholder"));
static NEW_GROUP_PLACEHOLDER: LazyLock<String> = LazyLock::new(|| fl!("new-group-placeholder"));
static SAVE: LazyLock<String> = LazyLock::new(|| fl!("save"));
static CANCEL: LazyLock<String> = LazyLock::new(|| fl!("cancel"));
static RUN: LazyLock<String> = LazyLock::new(|| fl!("run"));
static FLATPAK: LazyLock<String> = LazyLock::new(|| fl!("flatpak"));
static LOCAL: LazyLock<String> = LazyLock::new(|| fl!("local"));
static NIX: LazyLock<String> = LazyLock::new(|| fl!("nix"));
static SNAP: LazyLock<String> = LazyLock::new(|| fl!("snap"));
static SYSTEM: LazyLock<String> = LazyLock::new(|| fl!("system"));

static NEW_GROUP_WINDOW_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
static NEW_GROUP_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);
static DELETE_GROUP_WINDOW_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
static DELETE_GROUP_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);
pub(crate) static MENU_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
pub(crate) static MENU_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<ApplicationsTasks>,
}

#[cfg(test)]
mod responsive_grid_tests {
    use std::sync::Arc;

    use cosmic::desktop::DesktopEntryData;

    use super::CosmicAppLibrary;

    #[test]
    fn wider_board_creates_more_columns() {
        let mut app = CosmicAppLibrary::default();
        let initial = app.effective_columns();
        app.config.board_width = 1800;
        assert!(app.effective_columns() > initial);
    }

    #[test]
    fn larger_tiles_reduce_the_automatic_column_count() {
        let mut app = CosmicAppLibrary::default();
        let initial = app.effective_columns();
        app.config.icon_size = 128;
        assert!(app.effective_columns() < initial);
    }

    #[test]
    fn taller_board_exposes_more_drop_rows() {
        let mut app = CosmicAppLibrary::default();
        let initial = app.responsive_extra_rows();
        app.config.board_height = 1000;
        assert!(app.responsive_extra_rows() > initial);
    }

    #[test]
    fn automatic_grid_reserves_the_clipped_right_lane() {
        let app = CosmicAppLibrary::default();
        assert_eq!(app.effective_columns(), 7);
    }

    #[test]
    fn search_is_case_insensitive_and_matches_categories() {
        let entries = vec![
            Arc::new(DesktopEntryData {
                name: "Firefox".into(),
                categories: vec!["Network".into()],
                ..Default::default()
            }),
            Arc::new(DesktopEntryData {
                name: "Calculator".into(),
                categories: vec!["Utility".into()],
                ..Default::default()
            }),
        ];

        let (name_matches, _) =
            CosmicAppLibrary::board_entries(&Default::default(), "FIRE", &entries);
        let (category_matches, _) =
            CosmicAppLibrary::board_entries(&Default::default(), "utility", &entries);

        assert_eq!(name_matches[0].name, "Firefox");
        assert_eq!(category_matches[0].name, "Calculator");
    }

    #[test]
    fn search_results_do_not_include_empty_layout_slots() {
        let app = CosmicAppLibrary {
            search_value: "term".into(),
            ..Default::default()
        };

        assert_eq!(app.spatial_slots(None, &[2, 5]), vec![Some(2), Some(5)]);
    }
}

impl CosmicFlags for Args {
    type SubCommand = ApplicationsTasks;
    type Args = Vec<String>;

    fn action(&self) -> Option<&ApplicationsTasks> {
        self.subcommand.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, clap::Subcommand)]
pub enum ApplicationsTasks {
    #[clap(about = "Start app-library with an input")]
    Input { input: Option<String> },
    #[clap(about = "Close app-library if open")]
    Close,
    #[clap(about = "Run a standalone instance (not single-instance)")]
    Run,
}

impl Display for ApplicationsTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::ser::to_string(self).unwrap())
    }
}

impl FromStr for ApplicationsTasks {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::de::from_str(s)
    }
}

pub fn run() -> cosmic::iced::Result {
    let args = Args::parse();
    let settings = Settings::default()
        .antialiasing(true)
        .client_decorations(true)
        .debug(false)
        .default_text_size(16.0)
        .scale_factor(1.0)
        .no_main_window(true)
        .exit_on_close(false);

    // Use standalone run if requested, otherwise use single-instance
    if matches!(args.subcommand, Some(ApplicationsTasks::Run)) {
        cosmic::app::run::<CosmicAppLibrary>(settings, args)
    } else {
        cosmic::app::run_single_instance::<CosmicAppLibrary>(settings, args)
    }
}

pub struct AppSource(PathSource);

impl AppSource {
    pub fn as_icon(&self) -> Option<widget::icon::Handle> {
        let name = match &self.0 {
            PathSource::Local | PathSource::LocalDesktop => "app-source-local-symbolic",
            PathSource::System | PathSource::SystemLocal => "app-source-system-symbolic",
            PathSource::LocalFlatpak | PathSource::SystemFlatpak => "app-source-flatpak",
            PathSource::SystemSnap => "app-source-snap",
            PathSource::Nix | PathSource::LocalNix => "app-source-nix",
            PathSource::Other(_) => return None,
        };
        let handle = crate::icon_cache::icon_cache_handle(name, 16);
        Some(handle)
    }
}

impl<'a> From<&'a Path> for AppSource {
    fn from(path: &'a Path) -> Self {
        AppSource(PathSource::guess_from(path))
    }
}

impl Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.7}",
            match &self.0 {
                PathSource::Local | PathSource::LocalDesktop => LOCAL.as_str(),
                PathSource::SystemFlatpak | PathSource::LocalFlatpak => FLATPAK.as_str(),
                PathSource::SystemSnap => SNAP.as_str(),
                PathSource::Nix | PathSource::LocalNix => NIX.as_str(),
                PathSource::System | PathSource::SystemLocal => SYSTEM.as_str(),
                PathSource::Other(s) => s.as_str(),
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceState {
    Visible,
    Hidden,
    WaitingToBeShown,
}

struct CosmicAppLibrary {
    search_value: String,
    entry_path_input: Vec<Arc<DesktopEntryData>>,
    /// Group associated with each displayed entry. `None` is the ungrouped/Home section.
    entry_groups: Vec<Option<usize>>,
    all_entries: Vec<Arc<DesktopEntryData>>,
    menu: Option<usize>,
    helper: Option<Config>,
    config: AppLibraryConfig,
    active_page: usize,
    cur_group: Option<usize>,
    locale: Option<String>,
    edit_name: Option<String>,
    settings_open: bool,
    super_shortcut_enabled: bool,
    super_shortcut_error: Option<String>,
    new_group: Option<String>,
    dnd_icon: Option<usize>,
    offer_group: Option<Option<usize>>,
    waiting_for_filtered: bool,
    scroll_offset: f32,
    momentum_step: f32,
    last_trackpad_scroll: Option<Instant>,
    last_momentum_tick: Option<Instant>,
    core: Core,
    group_to_delete: Option<usize>,
    gpus: Option<Vec<Gpu>>,
    last_hide: Option<Instant>,
    duplicates: HashMap<PathBuf, (AppSource, Option<widget::icon::Handle>)>,
    app_list_config: AppListConfig,
    overlap: HashMap<String, Rectangle>,
    margin: f32,
    size: iced::Size,
    needs_clear: bool,
    focused_id: Option<widget::Id>,
    entry_ids: Vec<widget::Id>,
    entry_icon_handles: Vec<widget::icon::Handle>,
    scrollable_id: widget::Id,
    surface_state: SurfaceState,
    hand_over: String,
    group_keys: Vec<u64>,
    next_group_key: u64,
    dummy_id: Option<window::Id>,
}

impl Default for CosmicAppLibrary {
    fn default() -> Self {
        Self {
            search_value: Default::default(),
            entry_path_input: Default::default(),
            entry_groups: Default::default(),
            all_entries: Default::default(),
            menu: Default::default(),
            helper: Default::default(),
            config: Default::default(),
            active_page: Default::default(),
            cur_group: Default::default(),
            locale: Default::default(),
            edit_name: Default::default(),
            settings_open: Default::default(),
            super_shortcut_enabled: Default::default(),
            super_shortcut_error: Default::default(),
            new_group: Default::default(),
            dnd_icon: Default::default(),
            offer_group: Default::default(),
            waiting_for_filtered: Default::default(),
            scroll_offset: Default::default(),
            momentum_step: Default::default(),
            last_trackpad_scroll: Default::default(),
            last_momentum_tick: Default::default(),
            core: Default::default(),
            group_to_delete: Default::default(),
            gpus: Default::default(),
            last_hide: Default::default(),
            duplicates: Default::default(),
            app_list_config: Default::default(),
            overlap: Default::default(),
            margin: Default::default(),
            size: Size::ZERO,
            needs_clear: Default::default(),
            focused_id: Default::default(),
            entry_ids: Default::default(),
            entry_icon_handles: Default::default(),
            scrollable_id: widget::Id::unique(),
            surface_state: SurfaceState::Hidden,
            hand_over: String::default(),
            group_keys: Default::default(),
            next_group_key: Default::default(),
            dummy_id: None,
        }
    }
}

async fn try_get_gpus() -> Option<Vec<Gpu>> {
    let connection = zbus::Connection::system().await.ok()?;
    let proxy = switcheroo_control::SwitcherooControlProxy::new(&connection)
        .await
        .ok()?;

    if !proxy.has_dual_gpu().await.ok()? {
        return None;
    }

    let gpus = proxy.get_gpus().await.ok()?;
    if gpus.is_empty() {
        return None;
    }
    Some(gpus)
}

impl CosmicAppLibrary {
    fn create_dummy_layer_surface(&mut self) -> Task<Message> {
        self.needs_clear = true;
        let id = window::Id::unique();
        self.dummy_id = Some(id);
        cosmic::surface::surface_task(simple_layer_shell::<Message>(
            || LiveSettings {
                padding: Some(IcedMargin::default()),
                corners: Some(CornerRadius::default()),
                blur: Some(false),
            },
            move || {
                SctkLayerSurfaceSettings {
                    id,
                    layer: wlr_layer::Layer::Bottom,
                    keyboard_interactivity: wlr_layer::KeyboardInteractivity::None,
                    input_zone: Some(Vec::new()),
                    anchor: wlr_layer::Anchor::TOP,
                    output:
                        cosmic::iced::runtime::platform_specific::wayland::layer_surface::IcedOutput::Active,
                    namespace: "cosmic_launcher_dummy".into(),
                    margin: IcedMargin::default(),
                    size: Some((Some(1200), Some(200))),
                    exclusive_zone: -1,
                    size_limits: Limits::NONE,
                }
            },
            None::<fn() -> Element<'static, cosmic::Action<Message>>>,
        ))
    }

    pub fn activate(&mut self) -> Task<Message> {
        if matches!(self.surface_state, SurfaceState::Visible) {
            return self.hide();
        } else if matches!(self.surface_state, SurfaceState::Hidden)
            && self
                .last_hide
                .is_none_or(|i| i.elapsed() >= Duration::from_millis(100))
        {
            self.surface_state = SurfaceState::WaitingToBeShown;
            self.settings_open = false;
            self.edit_name = None;
            self.search_value = "".to_string();
            self.scroll_offset = 0.0;
            self.momentum_step = 0.0;
            self.last_trackpad_scroll = None;
            self.last_momentum_tick = None;
            self.cur_group = None;
            self.load_apps();
            self.needs_clear = true;
            let fetch_gpus = Task::perform(try_get_gpus(), |gpus| {
                cosmic::Action::App(Message::GpuUpdate(gpus))
            });
            return Task::batch(vec![
                cosmic::surface::surface_task(app_layer_shell(
                    |app: &CosmicAppLibrary| LiveSettings {
                        padding: Some(app.layer_padding()),
                        corners: None,
                        blur: None,
                    },
                    move |_: &mut CosmicAppLibrary| SctkLayerSurfaceSettings {
                        id: SurfaceId::RESERVED,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::all(),
                        namespace: "app-library".into(),
                        size: Some((None, None)),
                        exclusive_zone: -1,
                        ..Default::default()
                    },
                    None,
                )),
                fetch_gpus,
            ]);
        }
        Task::none()
    }

    fn handle_overlap(&mut self) -> Task<Message> {
        let mid_height = self.size.height / 2.;
        self.margin = 0.;

        for o in self.overlap.values() {
            if self.margin + mid_height < o.y
                || self.margin > o.y + o.height
                || mid_height < o.y + o.height
            {
                continue;
            }

            self.margin = o.y + o.height;
        }
        let mut cmds = Vec::with_capacity(2);
        // set the padding
        let margin = self.layer_padding();
        cmds.push(set_padding::<()>(SurfaceId::RESERVED, margin).discard());
        cmds.push(
            if self.core.system_theme().cosmic().frosted_system_interface {
                task::effect(Action::PlatformSpecific(
                    platform_specific::Action::Wayland(
                        cosmic::iced::runtime::platform_specific::wayland::Action::BlurSurface(
                            SurfaceId::RESERVED,
                            Some(vec![Rectangle {
                                x: 0.,
                                y: 0.,
                                width: f32::MAX,
                                height: f32::MAX,
                            }]),
                        ),
                    ),
                ))
            } else {
                task::effect(Action::PlatformSpecific(
                    platform_specific::Action::Wayland(
                        cosmic::iced::runtime::platform_specific::wayland::Action::BlurSurface(
                            SurfaceId::RESERVED,
                            None,
                        ),
                    ),
                ))
            },
        );
        Task::batch(cmds)
    }

    fn layer_padding(&self) -> IcedMargin {
        IcedMargin {
            #[allow(clippy::cast_possible_truncation)]
            top: self.margin as i32 + 16,
            left: ((self.size.width - 1200.) / 2.).max(0.) as i32,
            right: ((self.size.width - 1200.) / 2.).max(0.) as i32,
            bottom: (self.size.height - 690. - 16. - self.margin).max(0.) as i32,
        }
    }

    /// Update entry IDs and their icon handles.
    fn update_entry_metadata(&mut self) {
        self.entry_ids = (0..self.entry_path_input.len())
            .map(|_| widget::Id::unique())
            .collect();

        self.entry_icon_handles = self
            .entry_path_input
            .iter()
            .map(|e| e.icon.as_cosmic_icon())
            .collect();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum GroupRowKey {
    Home,
    Custom(u64),
    NewGroup,
}

#[derive(Clone, Debug)]
enum Message {
    Activate,
    UpdateFocused(Option<widget::Id>),
    InputChanged(String),
    TypeToSearch(String),
    KeyboardNav(keyboard_nav::Action),
    PrevRow,
    NextRow,
    Layer(LayerEvent, SurfaceId),
    Hide,
    ActivateApp(usize, Option<usize>),
    StartCurAppFocus,
    ActivationToken(Option<String>, String, String, Option<usize>, bool),
    SelectGroup(Option<usize>),
    ReorderGroup(Vec<GroupRowKey>),
    Delete(usize),
    ConfirmDelete,
    CancelDelete,
    StartEditName(String),
    EditName(String),
    SubmitName,
    StartNewGroup,
    NewGroup(String),
    SubmitNewGroup,
    CancelNewGroup,
    LoadApps,
    FilterApps(String, Vec<Arc<DesktopEntryData>>, Vec<Option<usize>>),
    OpenContextMenu(Rectangle, usize),
    CloseContextMenu,
    SelectAction(MenuAction),
    StartDrag(usize),
    FinishDrag,
    CancelDrag,
    StartDndOffer(Option<usize>),
    FinishDndOffer(Option<usize>, Option<DesktopEntryData>),
    FinishDndSlot(Option<usize>, usize, Option<DesktopEntryData>),
    LeaveDndOffer(Option<usize>),
    ToggleSettings,
    CloseSettings,
    PreviousPage,
    NextPage,
    AddPage,
    RemoveEmptyPage,
    RenamePage(String),
    RestoreHiddenApps,
    AdjustIconSize(i16),
    AdjustBoardWidth(i16),
    AdjustBoardHeight(i16),
    AdjustTileGap(i16),
    ToggleLabels,
    ToggleTooltips,
    ToggleKineticScrolling,
    ToggleSuperShortcut,
    AdjustMomentumStrength(i16),
    AdjustScrollGlide(i16),
    AdjustReleaseDelay(i16),
    ScrollYOffset(f32),
    TrackpadScroll(f32, Instant),
    MomentumTick(Instant),
    GpuUpdate(Option<Vec<Gpu>>),
    PinToAppTray(usize),
    UnPinFromAppTray(usize),
    AppListConfig(AppListConfig),
    Opened(Size, SurfaceId),
    Overlap(OverlapNotifyEvent),
    Output(OutputEvent),
}

#[derive(Clone, Debug)]
enum MenuAction {
    Pin,
    Remove,
    HideFromOtherApps,
    ShowInOtherApps,
    DesktopAction(String),
    OpenInStore(String),
}

pub fn menu_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
) -> cosmic::widget::Button<'a, Message> {
    cosmic::widget::button::custom(content)
        .class(Button::MenuItem)
        .padding(menu_control_padding())
        .width(Length::Fill)
}

pub fn menu_control_padding() -> Padding {
    let theme = cosmic::theme::active();
    let cosmic = theme.cosmic();
    [cosmic.space_xxs(), cosmic.space_m()].into()
}

impl CosmicAppLibrary {
    fn super_binding() -> ShortcutBinding {
        ShortcutBinding::new(ShortcutModifier::Super, None)
    }

    fn is_launchpad_shortcut(action: &ShortcutAction) -> bool {
        let ShortcutAction::Spawn(command) = action else {
            return false;
        };
        shlex::Shlex::new(command).next().is_some_and(|executable| {
            Path::new(&executable)
                .file_name()
                .is_some_and(|name| name == "cosmic-application-board")
        })
    }

    fn store_component_id(desktop_id: &str) -> &str {
        desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id)
    }

    fn read_super_shortcut() -> Result<bool, String> {
        let context = shortcuts::context().map_err(|error| error.to_string())?;
        let custom = context.get_local::<Shortcuts>("custom").unwrap_or_default();
        Ok(custom
            .0
            .get(&Self::super_binding())
            .is_some_and(Self::is_launchpad_shortcut))
    }

    fn set_super_shortcut(enabled: bool) -> Result<bool, String> {
        let context = shortcuts::context().map_err(|error| error.to_string())?;
        let mut custom = context.get_local::<Shortcuts>("custom").unwrap_or_default();
        let binding = Self::super_binding();

        if enabled {
            let executable = std::env::current_exe()
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .into_owned();
            custom.0.insert(binding, ShortcutAction::Spawn(executable));
        } else if custom
            .0
            .get(&binding)
            .is_some_and(Self::is_launchpad_shortcut)
        {
            custom.0.remove(&binding);
        }

        context
            .set("custom", custom)
            .map_err(|error| error.to_string())?;
        Self::read_super_shortcut()
    }

    fn application_tile_with_id<'a>(
        &'a self,
        i: usize,
        widget_id: widget::Id,
    ) -> Element<'a, Message> {
        let Some((entry, icon_handle)) = self
            .entry_path_input
            .get(i)
            .zip(self.entry_icon_handles.get(i))
        else {
            return space::horizontal().into();
        };

        let gpu_idx = self.gpus.as_ref().map(|gpus| {
            if entry.prefers_dgpu {
                gpus.iter().position(|gpu| !gpu.default).unwrap_or(0)
            } else {
                gpus.iter().position(|gpu| gpu.default).unwrap_or(0)
            }
        });
        let duplicate = entry
            .path
            .as_ref()
            .and_then(|path| self.duplicates.get(path));
        let selected = self.menu.is_some_and(|menu| menu == i);
        let settings_open = self.settings_open;

        let application = ApplicationButton::new(
            widget_id,
            &entry.name,
            icon_handle.clone(),
            &entry.path,
            move |rect| {
                if settings_open {
                    Message::CloseSettings
                } else {
                    Message::OpenContextMenu(rect, i)
                }
            },
            if settings_open {
                Some(Message::CloseSettings)
            } else if self.menu.is_none() {
                Some(Message::ActivateApp(i, gpu_idx))
            } else if selected {
                Some(Message::CloseContextMenu)
            } else {
                None
            },
            duplicate,
            selected,
            (!settings_open && self.menu.is_none()).then_some(Message::StartDrag(i)),
            (!settings_open && self.menu.is_none()).then_some(Message::FinishDrag),
            (!settings_open && self.menu.is_none()).then_some(Message::CancelDrag),
            self.config.icon_size as f32,
            self.config.show_labels,
        );

        if self.config.show_tooltips {
            tooltip(
                application,
                text::body(&entry.name),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .delay(Duration::from_millis(450))
            .into()
        } else {
            application.into()
        }
    }

    fn pinned_indices(&self) -> Vec<usize> {
        self.config
            .page_layout(self.active_page)
            .iter()
            .filter_map(Option::as_deref)
            .filter_map(|app_id| {
                self.entry_path_input
                    .iter()
                    .position(|entry| entry.id == app_id)
            })
            .collect()
    }

    fn spatial_slots(&self, group: Option<usize>, indices: &[usize]) -> Vec<Option<usize>> {
        let columns = self.effective_columns();
        let extra_rows = self.responsive_extra_rows();
        if !self.search_value.is_empty() {
            return indices.iter().copied().map(Some).collect();
        }

        let layout = group.map_or_else(
            || self.config.page_layout(self.active_page),
            |group| self.config.layout(Some(group)),
        );
        let mut slots = vec![None; layout.len()];
        let mut placed = std::collections::HashSet::new();
        for (slot, app_id) in layout.iter().enumerate() {
            let Some(app_id) = app_id.as_deref() else {
                continue;
            };
            if let Some(index) = indices.iter().copied().find(|index| {
                self.entry_path_input
                    .get(*index)
                    .is_some_and(|entry| entry.id == app_id)
            }) {
                slots[slot] = Some(index);
                placed.insert(index);
            }
        }
        slots.extend(
            indices
                .iter()
                .copied()
                .filter(|index| !placed.contains(index))
                .map(Some),
        );
        let rows = slots.len().div_ceil(columns).max(1);
        slots.resize((rows + extra_rows) * columns, None);
        slots
    }

    fn tile_cell_size(&self) -> f32 {
        self.config.icon_size as f32 + if self.config.show_labels { 72.0 } else { 24.0 }
    }

    fn effective_columns(&self) -> usize {
        let usable_width = f32::from(self.config.board_width).max(320.0) - 96.0;
        let pitch = self.tile_cell_size() + f32::from(self.config.tile_gap);
        // `usable_width` already reserves room for section padding, rounded window
        // edges, and the scrollbar. Removing another full tile lane leaves the
        // centered grid unnecessarily narrow.
        ((usable_width / pitch).floor() as usize).max(2)
    }

    fn responsive_extra_rows(&self) -> usize {
        let extra_height = self.config.board_height.saturating_sub(600);
        let pitch = self.tile_cell_size() + f32::from(self.config.tile_gap);
        (f32::from(extra_height) / pitch).floor().max(1.0) as usize
    }

    fn layout_snapshot(&self, group: Option<usize>) -> Vec<Option<String>> {
        let indices: Vec<usize> = if group.is_none() {
            self.pinned_indices()
        } else {
            self.entry_groups
                .iter()
                .enumerate()
                .filter_map(|(index, entry_group)| (*entry_group == group).then_some(index))
                .collect()
        };
        let mut layout: Vec<Option<String>> = self
            .spatial_slots(group, &indices)
            .into_iter()
            .map(|index| {
                index.and_then(|index| {
                    self.entry_path_input
                        .get(index)
                        .map(|entry| entry.id.clone())
                })
            })
            .collect();
        while layout.last().is_some_and(Option::is_none) {
            layout.pop();
        }
        layout
    }

    fn board_view<'a>(&'a self) -> Element<'a, Message> {
        let Spacing {
            space_none: _,
            space_xxs,
            space_xs,
            space_s,
            space_l,
            space_xxl,
            ..
        } = theme::spacing();

        let settings = tooltip(
            button::custom(
                row![
                    icon::icon(from_name("preferences-system-symbolic").into())
                        .width(Length::Fixed(22.0))
                        .height(Length::Fixed(22.0)),
                    text::body("Settings")
                ]
                .spacing(space_xxs)
                .align_y(Alignment::Center),
            )
            .padding([space_xs, space_s])
            .on_press(Message::ToggleSettings),
            text("Board settings"),
            tooltip::Position::Bottom,
        );
        let search_status = if self.search_value.trim().is_empty() {
            String::new()
        } else {
            let count = self.entry_path_input.len();
            format!("{count} {}", if count == 1 { "match" } else { "matches" })
        };
        let top_row = row![
            search_input(SEARCH_PLACEHOLDER.as_str(), self.search_value.as_str())
                .on_input(Message::InputChanged)
                .on_paste(Message::InputChanged)
                .on_submit(|_| Message::StartCurAppFocus)
                .style(TextInput::Search)
                .width(Length::Fixed(400.0))
                .size(14)
                .id(SEARCH_ID.clone()),
            text(search_status).size(14),
            space::horizontal(),
            settings,
        ]
        .spacing(space_xxs)
        .padding([space_s, space_xxl])
        .align_y(Alignment::Center);

        let adjustment = |label: &'a str,
                          value: String,
                          decrease: Message,
                          increase: Message|
         -> Element<'a, Message> {
            row![
                text(label).width(Length::Fill),
                button::text("−").on_press(decrease),
                container(text(value))
                    .width(Length::Fixed(64.0))
                    .center_x(Length::Fill),
                button::text("+").on_press(increase),
            ]
            .spacing(space_xxs)
            .align_y(Alignment::Center)
            .into()
        };

        let settings_panel: Element<'a, Message> = if self.settings_open {
            let setting_cell = |content: Element<'a, Message>| {
                container(content)
                    .padding([0, space_xs])
                    .width(Length::FillPortion(1))
            };
            let labels = row![
                text("Application labels").width(Length::Fill),
                button::text(if self.config.show_labels {
                    "Shown"
                } else {
                    "Hidden"
                })
                .on_press(Message::ToggleLabels)
            ]
            .align_y(Alignment::Center);
            let tooltips = row![
                text("Hover name tooltip").width(Length::Fill),
                button::text(if self.config.show_tooltips {
                    "On"
                } else {
                    "Off"
                })
                .on_press(Message::ToggleTooltips)
            ]
            .align_y(Alignment::Center);
            let kinetic_toggle = row![
                text("Trackpad momentum").width(Length::Fill),
                button::text(if self.config.kinetic_scrolling {
                    "On"
                } else {
                    "Off"
                })
                .on_press(Message::ToggleKineticScrolling)
            ]
            .align_y(Alignment::Center);
            let settings_content = column![
                text("Board settings").size(18),
                row![
                    setting_cell(adjustment(
                        "Tile size",
                        format!("{} px", self.config.icon_size),
                        Message::AdjustIconSize(-8),
                        Message::AdjustIconSize(8)
                    )),
                    setting_cell(adjustment(
                        "Board width",
                        format!("{} px", self.config.board_width),
                        Message::AdjustBoardWidth(-100),
                        Message::AdjustBoardWidth(100)
                    )),
                ]
                .spacing(space_s),
                row![
                    setting_cell(adjustment(
                        "Board height",
                        format!("{} px", self.config.board_height),
                        Message::AdjustBoardHeight(-50),
                        Message::AdjustBoardHeight(50)
                    )),
                    setting_cell(adjustment(
                        "Tile spacing",
                        format!("{} px", self.config.tile_gap),
                        Message::AdjustTileGap(-2),
                        Message::AdjustTileGap(2)
                    )),
                    setting_cell(labels.into()),
                ]
                .spacing(space_s),
                row![
                    setting_cell(tooltips.into()),
                    setting_cell(kinetic_toggle.into()),
                ]
                .spacing(space_s),
                text("Launchpad pages").size(18),
                row![
                    setting_cell(
                        row![
                            text("Page name"),
                            text_input("Page name", self.config.page_name(self.active_page))
                                .on_input(Message::RenamePage)
                                .width(Length::Fill)
                        ]
                        .spacing(space_xs)
                        .align_y(Alignment::Center)
                        .into()
                    ),
                    setting_cell(
                        row![
                            button::text("Add page").on_press(Message::AddPage),
                            button::text("Remove empty page").on_press_maybe(
                                (self.active_page > 0
                                    && !self
                                        .config
                                        .page_layout(self.active_page)
                                        .iter()
                                        .any(Option::is_some))
                                .then_some(Message::RemoveEmptyPage)
                            )
                        ]
                        .spacing(space_xs)
                        .into()
                    ),
                ]
                .spacing(space_s),
                row![
                    setting_cell(
                        text(format!(
                            "Hidden from Other Apps: {}",
                            self.config.hidden_apps.len()
                        ))
                        .into()
                    ),
                    setting_cell(
                        button::text("Restore all hidden apps")
                            .on_press_maybe(
                                (!self.config.hidden_apps.is_empty())
                                    .then_some(Message::RestoreHiddenApps)
                            )
                            .into()
                    ),
                ]
                .spacing(space_s),
                text("Kinetic scrolling").size(18),
                row![
                    setting_cell(adjustment(
                        "Momentum strength",
                        format!("{}%", self.config.momentum_strength),
                        Message::AdjustMomentumStrength(-10),
                        Message::AdjustMomentumStrength(10)
                    )),
                    setting_cell(adjustment(
                        "Glide",
                        format!("{}%", self.config.scroll_glide),
                        Message::AdjustScrollGlide(-10),
                        Message::AdjustScrollGlide(10)
                    )),
                ]
                .spacing(space_s),
                row![
                    setting_cell(adjustment(
                        "Release response",
                        format!("{} ms", self.config.release_delay_ms),
                        Message::AdjustReleaseDelay(-2),
                        Message::AdjustReleaseDelay(2)
                    )),
                    setting_cell(
                        text("Lower response values start coasting sooner.")
                            .size(13)
                            .into()
                    ),
                ]
                .spacing(space_s),
                text("Desktop integration").size(18),
                row![
                    setting_cell(
                        row![
                            text("Super key opens Launchpad").width(Length::Fill),
                            button::text(if self.super_shortcut_enabled {
                                "On"
                            } else {
                                "Off"
                            })
                            .on_press(Message::ToggleSuperShortcut)
                        ]
                        .align_y(Alignment::Center)
                        .into()
                    ),
                    setting_cell(
                        text(
                            self.super_shortcut_error
                                .as_deref()
                                .unwrap_or("Change this anytime without using the terminal.")
                        )
                        .size(13)
                        .into()
                    ),
                ]
                .spacing(space_s),
            ]
            .spacing(space_xs)
            .width(Length::Fill);
            let settings_width = (f32::from(self.config.board_width) - 96.0).clamp(700.0, 1080.0);
            container(
                container(scrollable(settings_content).height(Length::Shrink))
                    .max_height(360.0)
                    .padding([space_s, space_l])
                    .class(theme::Container::Card)
                    .width(Length::Fixed(settings_width)),
            )
            .center_x(Length::Fill)
            .width(Length::Fill)
            .into()
        } else {
            space::vertical().height(Length::Fixed(0.0)).into()
        };

        let columns = self.effective_columns();
        let cell_size = self.tile_cell_size();
        let pinned_indices = self.pinned_indices();
        let pinned_slots = self.spatial_slots(None, &pinned_indices);
        let mut pinned_rows: Vec<Element<'a, Message>> = Vec::new();
        for (row_index, chunk) in pinned_slots.chunks(columns).enumerate() {
            let mut cells: Vec<Element<'a, Message>> = Vec::new();
            for (column_index, entry_index) in chunk.iter().enumerate() {
                let slot = row_index * columns + column_index;
                let content: Element<'a, Message> = entry_index.map_or_else(
                    || {
                        if self.dnd_icon.is_some() {
                            container(text("+").size(18))
                                .center_x(Length::Fill)
                                .center_y(Length::Fill)
                                .class(theme::Container::Card)
                                .into()
                        } else {
                            space::horizontal().into()
                        }
                    },
                    |index| {
                        let id = self
                            .entry_path_input
                            .get(index)
                            .map_or_else(widget::Id::unique, |entry| {
                                widget::Id::new(format!("pinned-{}", entry.id))
                            });
                        self.application_tile_with_id(index, id)
                    },
                );
                let cell = container(content)
                    .width(Length::Fixed(cell_size))
                    .height(Length::Fixed(cell_size));
                let destination =
                    dnd_destination_for_data::<AppletString, Message>(cell, move |data, _| {
                        Message::FinishDndSlot(
                            None,
                            slot,
                            data.and_then(|data| load_desktop_file(&[], data.0)),
                        )
                    })
                    .drag_id(slot as u64)
                    .on_enter(move |_, _, _| Message::StartDndOffer(None))
                    .on_leave(move || Message::LeaveDndOffer(None));
                cells.push(destination.into());
            }
            pinned_rows.push(
                container(row(cells).spacing(self.config.tile_gap))
                    .center_x(Length::Fill)
                    .into(),
            );
        }

        let pinned_hint: Element<'a, Message> = if pinned_indices.is_empty() {
            text(if self.search_value.trim().is_empty() {
                "Drag applications from Other Apps into this area to add them to this page."
            } else {
                "No Launchpad apps match your search."
            })
            .size(14)
            .into()
        } else {
            space::vertical().height(Length::Fixed(0.0)).into()
        };
        let page_count = self.config.page_count();
        let page_header = row![
            text(self.config.page_name(self.active_page))
                .size(22)
                .width(Length::Fill),
            button::text("‹").on_press_maybe((page_count > 1).then_some(Message::PreviousPage)),
            text(format!("{} / {}", self.active_page + 1, page_count)).size(14),
            button::text("›").on_press_maybe((page_count > 1).then_some(Message::NextPage)),
            button::text("+").on_press(Message::AddPage),
        ]
        .spacing(space_xs)
        .align_y(Alignment::Center);
        let pinned_section = container(
            column![
                page_header,
                pinned_hint,
                column(pinned_rows)
                    .spacing(self.config.tile_gap)
                    .width(Length::Fill)
            ]
            .spacing(space_xs),
        )
        .padding(space_s)
        .width(Length::Fill)
        .class(if self.offer_group == Some(None) {
            theme::Container::Card
        } else {
            theme::Container::Background
        });

        let active_page_ids: std::collections::HashSet<&str> = self
            .config
            .page_layout(self.active_page)
            .iter()
            .filter_map(Option::as_deref)
            .collect();
        let pinned_ids: std::collections::HashSet<&str> = self.config.pinned_ids().collect();
        let searching = !self.search_value.trim().is_empty();
        let all_indices: Vec<usize> = self
            .entry_path_input
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let id = entry.id.as_str();
                (!active_page_ids.contains(id)
                    && (searching || (!pinned_ids.contains(id) && !self.config.is_hidden(id))))
                .then_some(index)
            })
            .collect();
        let mut all_rows: Vec<Element<'a, Message>> = Vec::new();
        for chunk in all_indices.chunks(columns) {
            let cells: Vec<Element<'a, Message>> = chunk
                .iter()
                .map(|index| {
                    let id = self
                        .entry_path_input
                        .get(*index)
                        .map_or_else(widget::Id::unique, |entry| {
                            widget::Id::new(format!("all-apps-{}", entry.id))
                        });
                    container(self.application_tile_with_id(*index, id))
                        .width(Length::Fixed(cell_size))
                        .height(Length::Fixed(cell_size))
                        .into()
                })
                .collect();
            all_rows.push(
                container(row(cells).spacing(self.config.tile_gap))
                    .center_x(Length::Fill)
                    .into(),
            );
        }
        let all_apps_hint: Element<'a, Message> = if all_indices.is_empty() {
            text(if self.search_value.trim().is_empty() {
                "Every available app is already in Launchpad."
            } else {
                "No other apps match your search."
            })
            .size(14)
            .into()
        } else {
            space::vertical().height(Length::Fixed(0.0)).into()
        };
        let all_apps_section = container(
            column![
                text("Other Apps").size(22),
                all_apps_hint,
                column(all_rows)
                    .spacing(self.config.tile_gap)
                    .width(Length::Fill)
            ]
            .spacing(space_xs),
        )
        .padding(space_s)
        .width(Length::Fill)
        .class(theme::Container::Background);

        let sections: Vec<Element<'a, Message>> =
            vec![pinned_section.into(), all_apps_section.into()];

        let board = mouse_area(
            scrollable(
                column(sections)
                    .spacing(space_l)
                    .padding([space_xs, space_xxl, space_xxl, space_xxl]),
            )
            .on_scroll(|viewport| Message::ScrollYOffset(viewport.absolute_offset().y))
            .id(self.scrollable_id.clone())
            .height(Length::Fill),
        )
        .on_press(Message::CloseSettings);

        let settings_overlay = column![
            space::vertical().height(Length::Fixed(f32::from(space_xs))),
            settings_panel,
            space::vertical().height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);
        let board_with_settings = stack![board, settings_overlay]
            .width(Length::Fill)
            .height(Length::Fill);
        let content = column![top_row, divider::horizontal::light(), board_with_settings];
        let window = container(content)
            .height(Length::Fixed(self.config.board_height as f32))
            .max_height(self.config.board_height as f32)
            .max_width(self.config.board_width as f32)
            .class(theme::Container::Custom(Box::new(|theme| {
                let cosmic = theme.cosmic();
                let radii = cosmic
                    .radius_s()
                    .map(|radius| if radius < 4.0 { radius } else { radius + 4.0 });
                container::Style {
                    text_color: Some(cosmic.on_bg_color().into()),
                    icon_color: Some(cosmic.on_bg_color().into()),
                    background: Some(Color::from(cosmic.background(theme.transparent).base).into()),
                    border: Border {
                        radius: radii.into(),
                        width: 1.0,
                        color: cosmic.bg_divider().into(),
                    },
                    shadow: Shadow::default(),
                    snap: true,
                }
            })))
            .center_x(Length::Fill)
            .width(Length::Fixed(self.config.board_width as f32));

        stack![
            mouse_area(
                container(space::horizontal().width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .on_press(Message::Hide),
            column![
                space::vertical().height(Length::Fixed(self.margin + 16.0)),
                mouse_area(window).on_press(Message::CloseContextMenu),
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn current_group(&self) -> &AppGroup {
        match self.cur_group {
            None => AppLibraryConfig::home(),
            Some(i) => &self.config.groups[i],
        }
    }

    pub fn load_apps(&mut self) {
        let xdg_current_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        self.all_entries = cosmic::desktop::load_applications(
            self.locale.as_slice(),
            false,
            xdg_current_desktop.as_deref(),
        )
        .filter(|d| d.exec.is_some())
        .map(Arc::new)
        .collect();
        self.all_entries.sort_by(|a, b| a.name.cmp(&b.name));

        let (entries, groups) =
            Self::board_entries(&self.config, &self.search_value, &self.all_entries);
        self.entry_path_input = entries;
        self.entry_groups = groups;

        // collect duplicates
        self.duplicates.clear();
        self.duplicates = self
            .all_entries
            .iter()
            .enumerate()
            .fold(
                (std::mem::take(&mut self.duplicates), 0, "", ""),
                |(mut dups, cur_count, cur_name, cur_id): (HashMap<_, _>, usize, &str, &str),
                 (i, e)| {
                    if cur_name.to_lowercase().trim() == e.name.to_lowercase().trim()
                        || e.id == cur_id
                    {
                        if cur_count == 1 {
                            // insert previous entry
                            if let Some(path) = self.all_entries[i - 1].path.as_ref() {
                                let source = AppSource::from(path.as_ref());
                                let icon_handle = source.as_icon();
                                dups.insert(path.clone(), (source, icon_handle));
                            }
                        }
                        if let Some(path) = e.path.as_ref() {
                            let source = AppSource::from(path.as_ref());
                            let icon_handle = source.as_icon();
                            dups.insert(path.clone(), (source, icon_handle));
                        }
                        (dups, cur_count + 1, cur_name, cur_id)
                    } else {
                        (dups, 1, e.name.as_str(), e.id.as_str())
                    }
                },
            )
            .0;
        self.update_entry_metadata();
    }

    fn filter_apps(&mut self) -> Task<Message> {
        let config = self.config.clone();
        let all_entries = self.all_entries.clone();
        let input = self.search_value.clone();
        if !self.waiting_for_filtered {
            self.waiting_for_filtered = true;
            iced::Task::perform(
                async move {
                    let (apps, groups) =
                        CosmicAppLibrary::board_entries(&config, &input, &all_entries);
                    (input, apps, groups)
                },
                |(input, apps, groups)| Message::FilterApps(input, apps, groups),
            )
            .map(cosmic::Action::App)
        } else {
            iced::Task::none()
        }
    }

    fn board_entries(
        _config: &AppLibraryConfig,
        input: &str,
        all_entries: &[Arc<DesktopEntryData>],
    ) -> (Vec<Arc<DesktopEntryData>>, Vec<Option<usize>>) {
        let query = input.to_lowercase();
        let mut entries: Vec<_> = all_entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry.name.to_lowercase().contains(&query)
                    || entry
                        .categories
                        .iter()
                        .any(|category| category.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        let groups = vec![None; entries.len()];
        (entries, groups)
    }

    pub fn hide(&mut self) -> Task<Message> {
        if !matches!(self.surface_state, SurfaceState::Visible) {
            return Task::none();
        }
        // cancel existing dnd if it exists then try again...
        if self.dnd_icon.take().is_some() {
            return Task::batch(vec![
                end_dnd(),
                Task::perform(async {}, |_| cosmic::Action::App(Message::Hide)),
            ]);
        }
        self.focused_id = None;
        self.entry_ids.clear();
        self.entry_icon_handles.clear();
        self.new_group = None;
        self.search_value.clear();
        self.edit_name = None;
        self.cur_group = None;
        self.settings_open = false;
        self.menu = None;
        self.group_to_delete = None;
        self.scroll_offset = 0.0;
        self.momentum_step = 0.0;
        self.last_trackpad_scroll = None;
        self.last_momentum_tick = None;
        self.surface_state = SurfaceState::Hidden;
        self.hand_over.clear();

        iced::Task::batch(vec![
            destroy_popup(*MENU_ID),
            destroy_layer_surface(*NEW_GROUP_WINDOW_ID),
            destroy_layer_surface(*DELETE_GROUP_WINDOW_ID),
            destroy_layer_surface(SurfaceId::RESERVED),
        ])
    }

    fn activate_app(
        &mut self,
        i: usize,
        gpu_idx: Option<usize>,
    ) -> Task<<Self as cosmic::Application>::Message> {
        self.edit_name = None;
        if let Some(de) = self.entry_path_input.get(i) {
            let app_id = de.id.clone();
            let exec = de.exec.clone().unwrap();
            let terminal = de.terminal;
            request_token(
                Some(String::from(<Self as cosmic::Application>::APP_ID)),
                Some(SurfaceId::RESERVED),
            )
            .map(move |t| {
                cosmic::Action::App(Message::ActivationToken(
                    t,
                    app_id.clone(),
                    exec.clone(),
                    gpu_idx,
                    terminal,
                ))
            })
        } else {
            Task::none()
        }
    }
}

impl cosmic::Application for CosmicAppLibrary {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = Args;
    const APP_ID: &'static str = crate::config::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        self.hide()
    }

    fn update(&mut self, message: Message) -> Task<Self::Message> {
        match message {
            Message::Activate => {
                return self.activate();
            }
            Message::Output(event) => {
                if matches!(event, OutputEvent::Created(_) | OutputEvent::InfoUpdate(_))
                    && self.dummy_id.is_none()
                {
                    return self.create_dummy_layer_surface();
                }
            }
            Message::UpdateFocused(id) => {
                self.focused_id = id;
                let i = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .unwrap_or(0);
                let y =
                    ((i / 7) as f32 / ((self.entry_path_input.len() / 7) as f32).max(1.)).max(0.0);

                return iced_runtime::task::widget(operation::scrollable::snap_to(
                    self.scrollable_id.clone(),
                    RelativeOffset {
                        x: None,
                        y: Some(y),
                    },
                ));
            }
            Message::KeyboardNav(message) => match message {
                keyboard_nav::Action::FocusNext => {
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_next()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                keyboard_nav::Action::FocusPrevious => {
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_previous()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                keyboard_nav::Action::Escape => {
                    return self.on_escape();
                }
                keyboard_nav::Action::Search => return self.on_search(),

                keyboard_nav::Action::Fullscreen => {}
            },

            Message::PrevRow => {
                let mut i = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .unwrap_or(self.entry_ids.len().saturating_add(6));
                if i == 0 {
                    self.focused_id = None;

                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_previous()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                i = i.saturating_sub(7);
                let y =
                    ((i / 7) as f32 / ((self.entry_path_input.len() / 7) as f32).max(1.)).max(0.0);

                let Some(focused) = self.entry_ids.get(i).cloned() else {
                    return Task::none();
                };
                self.focused_id = Some(focused.clone());
                return Task::batch(vec![
                    iced_runtime::task::widget(focus(focused))
                        .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    iced_runtime::task::widget(operation::scrollable::snap_to(
                        self.scrollable_id.clone(),
                        RelativeOffset {
                            x: None,
                            y: Some(y),
                        },
                    )),
                ]);
            }
            Message::NextRow => {
                let mut i: i32 = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .map(|i| i as i32)
                    .unwrap_or(-7);
                if i == self.entry_ids.len() as i32 - 1 {
                    self.focused_id = None;
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_next()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                i += 7;
                i = i.min(self.entry_ids.len() as i32 - 1);
                let Some(focused) = self.entry_ids.get(i as usize).cloned() else {
                    return Task::none();
                };
                self.focused_id = Some(focused.clone());
                let y =
                    ((i / 7) as f32 / ((self.entry_path_input.len() / 7) as f32).max(1.)).max(0.0);

                return Task::batch(vec![
                    iced_runtime::task::widget(operation::scrollable::snap_to(
                        self.scrollable_id.clone(),
                        RelativeOffset {
                            x: None,
                            y: Some(y),
                        },
                    )),
                    iced_runtime::task::widget(focus(focused))
                        .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                ]);
            }
            Message::InputChanged(value) => {
                self.search_value = value;
                return self.filter_apps();
            }
            Message::TypeToSearch(value) => {
                if value.is_empty()
                    || (self.search_value.is_empty() && value.chars().all(char::is_whitespace))
                {
                    return Task::none();
                }
                self.settings_open = false;
                self.menu = None;
                self.search_value.push_str(&value);
                return Task::batch(vec![
                    destroy_popup(*MENU_ID),
                    self.filter_apps(),
                    text_input::focus(SEARCH_ID.clone()),
                ]);
            }
            Message::Layer(e, id) => {
                match e {
                    LayerEvent::Focused if self.menu.is_none() => {
                        if id == SurfaceId::RESERVED {
                            return text_input::focus(SEARCH_ID.clone()).chain(
                                iced_runtime::task::widget(find_focused()).map(|id| {
                                    cosmic::Action::App(Message::UpdateFocused(Some(id)))
                                }),
                            );
                        } else if id == *DELETE_GROUP_WINDOW_ID {
                            return button::focus(SUBMIT_DELETE_ID.clone());
                        } else if id == *NEW_GROUP_WINDOW_ID {
                            return text_input::focus(NEW_GROUP_ID.clone());
                        }
                    }
                    LayerEvent::Unfocused => {
                        self.last_hide = Some(Instant::now());
                        if matches!(self.surface_state, SurfaceState::Visible)
                            && id == SurfaceId::RESERVED
                            && self.menu.is_none()
                            && self.new_group.is_none()
                            && self.group_to_delete.is_none()
                        {
                            return self.hide();
                        }
                    }
                    LayerEvent::Done if id == SurfaceId::RESERVED => {
                        // no need for commands here
                        _ = self.hide();
                    }
                    _ => {}
                }
            }
            Message::Hide => {
                return self.hide();
            }
            Message::ActivateApp(i, gpu_idx) => {
                return self.activate_app(i, gpu_idx);
            }
            Message::StartCurAppFocus => {
                let i = if self
                    .focused_id
                    .as_ref()
                    .is_some_and(|cur_focus| cur_focus == &*SEARCH_ID)
                {
                    0
                } else {
                    self.focused_id
                        .as_ref()
                        .and_then(|focus| self.entry_ids.iter().position(|id| focus == id))
                        .unwrap_or_default()
                };
                let gpu_idx = None;
                return self.activate_app(i, gpu_idx);
            }
            Message::ActivationToken(token, app_id, exec, gpu_idx, terminal) => {
                let mut env_vars = Vec::new();
                if let Some(token) = token {
                    env_vars.push(("XDG_ACTIVATION_TOKEN".to_string(), token.clone()));
                    env_vars.push(("DESKTOP_STARTUP_ID".to_string(), token));
                }
                if let (Some(gpus), Some(idx)) = (self.gpus.as_ref(), gpu_idx) {
                    env_vars.extend(gpus[idx].environment.clone());
                }
                tokio::spawn(async move {
                    cosmic::desktop::spawn_desktop_exec(exec, env_vars, Some(&app_id), terminal)
                        .await
                });
                return self.update(Message::Hide);
            }
            Message::SelectGroup(group) => {
                self.edit_name = None;
                self.search_value.clear();
                self.cur_group = group;
                self.scroll_offset = 0.0;
                self.scrollable_id = Id::new(format!("group-{}", group.unwrap_or(usize::MAX)));
                let mut cmds = vec![self.filter_apps()];
                if self.cur_group.is_none() {
                    cmds.push(text_input::focus(SEARCH_ID.clone()));
                }
                return iced::Task::batch(cmds);
            }
            Message::ReorderGroup(new_order) => {
                let prev_selected_key =
                    self.cur_group.and_then(|i| self.group_keys.get(i).copied());

                let reorder_keys: Vec<u64> = new_order
                    .into_iter()
                    .filter_map(|key| match key {
                        GroupRowKey::Custom(k) => Some(k),
                        GroupRowKey::Home | GroupRowKey::NewGroup => None,
                    })
                    .collect();

                if reorder_keys.len() != self.config.groups.len() {
                    return Task::none();
                }

                let key_to_index: HashMap<u64, usize> = self
                    .group_keys
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| (k, i))
                    .collect();

                let reordered: Vec<crate::app_group::AppGroup> = reorder_keys
                    .iter()
                    .filter_map(|k| {
                        key_to_index
                            .get(k)
                            .and_then(|&i| self.config.groups.get(i).cloned())
                    })
                    .collect();

                if reordered.len() != self.config.groups.len() {
                    return Task::none();
                }

                self.config.groups = reordered;
                self.group_keys = reorder_keys.clone();

                if let Some(key) = prev_selected_key {
                    self.cur_group = reorder_keys.iter().position(|&k| k == key);
                }

                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::LoadApps => {
                return self.filter_apps();
            }
            Message::Delete(group) => {
                self.group_to_delete = Some(group);
                return Task::batch(vec![
                    get_layer_surface(SctkLayerSurfaceSettings {
                        id: *DELETE_GROUP_WINDOW_ID,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::empty(),
                        namespace: "dialog".into(),
                        size: None,
                        ..Default::default()
                    }),
                    button::focus(SUBMIT_DELETE_ID.clone()),
                ]);
            }
            Message::EditName(name) => {
                self.edit_name = Some(name);
            }
            Message::SubmitName => {
                if let Some(name) = self.edit_name.take()
                    && let Some(i) = self.cur_group
                {
                    self.config.set_name(i, name);
                }
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::StartEditName(name) => {
                self.edit_name = Some(name);
                return text_input::focus(EDIT_GROUP_ID.clone());
            }
            Message::StartNewGroup => {
                if self.new_group.is_some() {
                    return Task::none();
                }
                self.new_group = Some(String::new());
                return Task::batch(vec![
                    get_layer_surface(SctkLayerSurfaceSettings {
                        id: *NEW_GROUP_WINDOW_ID,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::empty(),
                        namespace: "dialog".into(),
                        size: None,
                        ..Default::default()
                    }),
                    text_input::focus(NEW_GROUP_ID.clone()),
                ]);
            }
            Message::NewGroup(group_name) => {
                self.new_group = Some(group_name);
            }
            Message::SubmitNewGroup => {
                if let Some(group_name) = self.new_group.take() {
                    self.config.add(group_name);
                    self.group_keys.push(self.next_group_key);
                    self.next_group_key += 1;
                }
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return Task::batch([
                    destroy_layer_surface(*NEW_GROUP_WINDOW_ID),
                    self.filter_apps(),
                ]);
            }
            Message::CancelNewGroup => {
                self.new_group = None;
                return destroy_layer_surface(*NEW_GROUP_WINDOW_ID);
            }
            Message::OpenContextMenu(rect, i) => {
                if self.menu.take().is_some() {
                    return destroy_popup(*MENU_ID);
                } else {
                    self.menu = Some(i);
                    let offset = self.scroll_offset as i32;
                    return cosmic::surface::surface_task(simple_popup(
                        LiveSettings::default,
                        move || {
                            SctkPopupSettings {
                        parent: SurfaceId::RESERVED,
                        id: *MENU_ID,
                        positioner: SctkPositioner {
                            size: None,
                            size_limits: Limits::NONE.min_width(1.0).min_height(1.0).max_width(300.0).max_height(800.0),
                            anchor_rect: Rectangle {
                                x: rect.x as i32,
                                y: rect.y as i32 - offset,
                                width: rect.width as i32,
                                height: rect.height as i32,
                            },
                            anchor:
                                sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Anchor::Right,
                            gravity: sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Gravity::Right,
                            reactive: true,
                            ..Default::default()
                        },
                        grab: false,
                        parent_size: None,
                        close_with_children: true,
                        input_zone: None,
                    }
                        },
                        None::<Box<fn() -> cosmic::Element<'static, cosmic::Action<Message>>>>,
                    ));
                }
            }
            Message::CloseContextMenu => {
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::SelectAction(action) => {
                let mut tasks = vec![commands::popup::destroy_popup(*MENU_ID)];
                if let Some((info, _entry_group)) = self.menu.take().and_then(|i| {
                    self.entry_path_input
                        .get(i)
                        .zip(self.entry_groups.get(i).copied())
                }) {
                    match action {
                        MenuAction::Pin => {
                            let slot = self
                                .config
                                .page_layout(self.active_page)
                                .iter()
                                .position(Option::is_none)
                                .unwrap_or(self.config.page_layout(self.active_page).len());
                            self.config
                                .place_entry_on_page(self.active_page, slot, &info.id);
                            if let Some(helper) = self.helper.as_ref()
                                && let Err(err) = self.config.write_entry(helper)
                            {
                                error!("{:?}", err);
                            }
                            tasks.push(self.filter_apps());
                        }
                        MenuAction::Remove => {
                            self.config
                                .remove_entry_from_page(self.active_page, &info.id);
                            if let Some(helper) = self.helper.as_ref()
                                && let Err(err) = self.config.write_entry(helper)
                            {
                                error!("{:?}", err);
                            }
                            tasks.push(self.filter_apps());
                        }
                        MenuAction::HideFromOtherApps => {
                            self.config.set_hidden(&info.id, true);
                            if let Some(helper) = self.helper.as_ref()
                                && let Err(err) = self.config.write_entry(helper)
                            {
                                error!("{:?}", err);
                            }
                            tasks.push(self.filter_apps());
                        }
                        MenuAction::ShowInOtherApps => {
                            self.config.set_hidden(&info.id, false);
                            if let Some(helper) = self.helper.as_ref()
                                && let Err(err) = self.config.write_entry(helper)
                            {
                                error!("{:?}", err);
                            }
                            tasks.push(self.filter_apps());
                        }
                        MenuAction::DesktopAction(exec) => {
                            let mut exec = shlex::Shlex::new(&exec);

                            let mut cmd = match exec.next() {
                                Some(cmd) if !cmd.contains('=') => {
                                    tokio::process::Command::new(cmd)
                                }
                                _ => return Task::none(),
                            };
                            for arg in exec {
                                // TODO handle "%" args here if necessary?
                                if !arg.starts_with('%') {
                                    cmd.arg(arg);
                                }
                            }
                            let _ = cmd.spawn();
                            return self.hide();
                        }
                        MenuAction::OpenInStore(desktop_id) => {
                            let component_id = Self::store_component_id(&desktop_id);
                            let _ = tokio::process::Command::new("cosmic-store")
                                .arg(format!("appstream:{component_id}"))
                                .spawn();
                            return self.hide();
                        }
                    }
                }
                return cosmic::Task::batch(tasks);
            }
            Message::StartDrag(i) => {
                self.dnd_icon = Some(i);
            }
            Message::FinishDrag => {
                // A drag only changes state when a pinned-grid slot accepts it.
                // Cancelling or dropping elsewhere must not unpin the app.
                self.dnd_icon = None;
            }
            Message::CancelDrag => {
                self.dnd_icon = None;
            }
            Message::StartDndOffer(group) => {
                self.offer_group = Some(group);
            }
            // Retained for the stock folder view below. The board itself only
            // uses slot-specific targets, which prevents the parent group from
            // stealing a drop.
            Message::FinishDndOffer(group, entry) => {
                self.offer_group = None;
                let Some(entry) = entry else {
                    return Task::none();
                };
                self.config.add_entry(group, &entry.id);
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
                return self.filter_apps();
            }
            Message::FinishDndSlot(group, slot, entry) => {
                self.offer_group = None;
                if group.is_none() {
                    let entry = self
                        .dnd_icon
                        .take()
                        .and_then(|index| self.entry_path_input.get(index).cloned())
                        .or_else(|| entry.map(Arc::new));
                    let Some(entry) = entry else {
                        return Task::none();
                    };
                    self.config
                        .place_entry_on_page(self.active_page, slot, &entry.id);
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    return self.filter_apps();
                }
                let source_group_hint = self
                    .dnd_icon
                    .and_then(|index| self.entry_groups.get(index).copied());
                let destination_snapshot = self.layout_snapshot(group);
                let source_snapshot = source_group_hint
                    .filter(|source_group| *source_group != group)
                    .map(|source_group| (source_group, self.layout_snapshot(source_group)));
                self.config.set_layout(group, destination_snapshot);
                if let Some((source_group, snapshot)) = source_snapshot {
                    self.config.set_layout(source_group, snapshot);
                }
                let source = self.dnd_icon.take().and_then(|i| {
                    self.entry_path_input
                        .get(i)
                        .cloned()
                        .zip(self.entry_groups.get(i).copied())
                });
                let Some((entry, source_group)) =
                    source.or_else(|| entry.map(|entry| (Arc::new(entry), None)))
                else {
                    return Task::none();
                };
                if source_group != group {
                    self.config.remove_entry(source_group, &entry.id);
                    self.config.add_entry(group, &entry.id);
                }
                self.config.place_entry(group, slot, &entry.id);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::LeaveDndOffer(group) => {
                self.offer_group = self.offer_group.filter(|g| *g != group);
            }
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
            }
            Message::CloseSettings => {
                self.settings_open = false;
            }
            Message::PreviousPage => {
                let count = self.config.page_count();
                self.active_page = (self.active_page + count - 1) % count;
                self.scroll_offset = 0.0;
                self.scrollable_id = Id::new(format!("launchpad-page-{}", self.active_page));
            }
            Message::NextPage => {
                self.active_page = (self.active_page + 1) % self.config.page_count();
                self.scroll_offset = 0.0;
                self.scrollable_id = Id::new(format!("launchpad-page-{}", self.active_page));
            }
            Message::AddPage => {
                self.active_page = self.config.add_page();
                self.scroll_offset = 0.0;
                self.scrollable_id = Id::new(format!("launchpad-page-{}", self.active_page));
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::RemoveEmptyPage => {
                if self.config.remove_empty_page(self.active_page) {
                    self.active_page = self.active_page.saturating_sub(1);
                    self.scrollable_id = Id::new(format!("launchpad-page-{}", self.active_page));
                    if let Some(helper) = self.helper.as_ref() {
                        let _ = self.config.write_entry(helper);
                    }
                }
            }
            Message::RenamePage(name) => {
                self.config.set_page_name(self.active_page, name);
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::RestoreHiddenApps => {
                self.config.hidden_apps.clear();
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
                return self.filter_apps();
            }
            Message::AdjustIconSize(change) => {
                self.config.icon_size =
                    ((self.config.icon_size as i32) + i32::from(change)).clamp(32, 128) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::AdjustBoardWidth(change) => {
                self.config.board_width =
                    ((self.config.board_width as i32) + i32::from(change)).clamp(700, 1800) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::AdjustBoardHeight(change) => {
                self.config.board_height =
                    ((self.config.board_height as i32) + i32::from(change)).clamp(450, 1000) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::AdjustTileGap(change) => {
                self.config.tile_gap =
                    ((self.config.tile_gap as i32) + i32::from(change)).clamp(0, 32) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::ToggleLabels => {
                self.config.show_labels = !self.config.show_labels;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::ToggleTooltips => {
                self.config.show_tooltips = !self.config.show_tooltips;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::ToggleKineticScrolling => {
                self.config.kinetic_scrolling = !self.config.kinetic_scrolling;
                self.momentum_step = 0.0;
                self.last_trackpad_scroll = None;
                self.last_momentum_tick = None;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::ToggleSuperShortcut => {
                match Self::set_super_shortcut(!self.super_shortcut_enabled) {
                    Ok(enabled) => {
                        self.super_shortcut_enabled = enabled;
                        self.super_shortcut_error = None;
                    }
                    Err(error) => {
                        error!("failed to update the Super key shortcut: {error}");
                        self.super_shortcut_error = Some("Could not update the Super key".into());
                    }
                }
            }
            Message::AdjustMomentumStrength(change) => {
                self.config.momentum_strength = ((self.config.momentum_strength as i32)
                    + i32::from(change))
                .clamp(25, 300) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::AdjustScrollGlide(change) => {
                self.config.scroll_glide =
                    ((self.config.scroll_glide as i32) + i32::from(change)).clamp(25, 300) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::AdjustReleaseDelay(change) => {
                self.config.release_delay_ms =
                    ((self.config.release_delay_ms as i32) + i32::from(change)).clamp(4, 40) as u16;
                if let Some(helper) = self.helper.as_ref() {
                    let _ = self.config.write_entry(helper);
                }
            }
            Message::ScrollYOffset(y) => {
                self.scroll_offset = y;
            }
            Message::TrackpadScroll(delta, now) => {
                if !matches!(self.surface_state, SurfaceState::Visible)
                    || !self.config.kinetic_scrolling
                {
                    return Task::none();
                }

                let continuing_gesture = self
                    .last_trackpad_scroll
                    .is_some_and(|last| now - last < Duration::from_millis(100));

                if continuing_gesture {
                    self.momentum_step = self.momentum_step * 0.35 + delta * 0.65;
                } else {
                    self.momentum_step = delta;
                }
                self.last_trackpad_scroll = Some(now);
                self.last_momentum_tick = Some(now);
            }
            Message::MomentumTick(now) => {
                let Some(last_scroll) = self.last_trackpad_scroll else {
                    return Task::none();
                };

                let release_delay = Duration::from_millis(u64::from(self.config.release_delay_ms));
                if now - last_scroll < release_delay {
                    self.last_momentum_tick = Some(now);
                    return Task::none();
                }

                let elapsed = self
                    .last_momentum_tick
                    .map_or(1.0 / 60.0, |last| (now - last).as_secs_f32())
                    .clamp(1.0 / 240.0, 0.05);
                self.last_momentum_tick = Some(now);
                let frame_count = elapsed * 60.0;
                let glide = f32::from(self.config.scroll_glide) / 100.0;
                self.momentum_step *= 0.92_f32.powf(frame_count / glide);

                if self.momentum_step.abs() < 0.25 {
                    self.momentum_step = 0.0;
                    self.last_trackpad_scroll = None;
                    self.last_momentum_tick = None;
                    return Task::none();
                }

                return iced_runtime::task::widget(operation::scrollable::scroll_by(
                    self.scrollable_id.clone(),
                    AbsoluteOffset {
                        x: 0.0,
                        y: self.momentum_step
                            * (f32::from(self.config.momentum_strength) / 100.0)
                            * frame_count,
                    },
                ));
            }
            Message::ConfirmDelete => {
                let mut cmds = vec![destroy_layer_surface(*DELETE_GROUP_WINDOW_ID)];
                if let Some(group) = self.group_to_delete.take() {
                    self.config.remove(group);
                    if group < self.group_keys.len() {
                        self.group_keys.remove(group);
                    }
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    self.cur_group = None;
                    cmds.push(self.filter_apps());
                }
                return Task::batch(cmds);
            }
            Message::CancelDelete => {
                self.group_to_delete = None;
                return destroy_layer_surface(*DELETE_GROUP_WINDOW_ID);
            }
            Message::FilterApps(input, filtered_apps, entry_groups) => {
                self.entry_path_input = filtered_apps;
                self.entry_groups = entry_groups;
                self.update_entry_metadata();

                self.waiting_for_filtered = false;
                if self.search_value != input {
                    return self.filter_apps();
                }
            }
            Message::GpuUpdate(gpus) => {
                self.gpus = gpus;
            }
            Message::PinToAppTray(usize) => {
                let pinned_id = self.entry_path_input.get(usize).map(|e| e.id.clone());
                if let Some((pinned_id, app_list_helper)) = pinned_id
                    .zip(Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok())
                {
                    self.app_list_config.add_pinned(pinned_id, &app_list_helper);
                }
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::UnPinFromAppTray(usize) => {
                let pinned_id = self.entry_path_input.get(usize).map(|e| e.id.clone());
                if let Some((pinned_id, app_list_helper)) = pinned_id
                    .zip(Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok())
                {
                    self.app_list_config
                        .remove_pinned(&pinned_id, &app_list_helper);
                }
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::AppListConfig(config) => {
                self.app_list_config = config;
            }
            Message::Opened(size, window_id) => {
                let mut tasks = Vec::new();
                if let Some(dummy) = self.dummy_id
                    && window_id == dummy
                {
                    tasks.push(overlap_notify(window_id, true));
                } else if self.dummy_id.is_none() {
                    tasks.push(overlap_notify(SurfaceId::RESERVED, true));
                }
                if window_id == SurfaceId::RESERVED {
                    if matches!(self.surface_state, SurfaceState::WaitingToBeShown) {
                        self.surface_state = SurfaceState::Visible;
                    }
                    self.size = size;
                    tasks.push(self.handle_overlap());
                }
                if !self.hand_over.is_empty() {
                    let input = self.hand_over.clone();
                    self.hand_over.clear();
                    tasks.push(self.update(Message::InputChanged(input)));
                }
                return Task::batch(tasks);
            }
            Message::Overlap(overlap_notify_event) => match overlap_notify_event {
                OverlapNotifyEvent::OverlapLayerAdd {
                    identifier,
                    namespace,
                    logical_rect,
                    exclusive,
                    ..
                } => {
                    if exclusive > 0 || namespace == "Dock" || namespace == "Panel" {
                        if self.needs_clear {
                            self.needs_clear = false;
                            self.overlap.clear();
                        }
                        self.overlap.insert(identifier, logical_rect);
                    }
                    return self.handle_overlap();
                }
                OverlapNotifyEvent::OverlapLayerRemove { identifier } => {
                    self.overlap.remove(&identifier);
                    return self.handle_overlap();
                }
                _ => {}
            },
        }
        Task::none()
    }

    fn dbus_activation(&mut self, msg: dbus_activation::Message) -> Task<Self::Message> {
        match msg.msg {
            dbus_activation::Details::Activate => self.activate(),
            dbus_activation::Details::ActivateAction { action, .. } => {
                let Ok(cmd) = ApplicationsTasks::from_str(&action) else {
                    return Task::none();
                };
                match cmd {
                    ApplicationsTasks::Input { input } => {
                        if let Some(input) = input {
                            self.hand_over.push_str(&input);
                        }
                        if self.surface_state == SurfaceState::Hidden {
                            return self.activate();
                        }
                        Task::none()
                    }
                    ApplicationsTasks::Close => self.hide(),
                    // Run is handled at startup, not via D-Bus
                    ApplicationsTasks::Run => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        unimplemented!()
    }

    fn view_window<'a>(&'a self, id: SurfaceId) -> Element<'a, Message> {
        if self.dummy_id.is_some_and(|dummy| dummy == id) {
            return horizontal().into();
        }
        let Spacing {
            space_none,
            space_xxs,
            space_xs,
            space_s,
            space_l,
            space_xxl,
            ..
        } = theme::spacing();

        if id == *MENU_ID {
            let Some((menu, i)) = self
                .menu
                .as_ref()
                .and_then(|i| self.entry_path_input.get(*i).map(|e| (e, i)))
            else {
                return container(space::horizontal())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(1.0))
                    .into();
            };

            let mut list_column = Vec::new();

            if let Some(gpus) = self.gpus.as_ref() {
                for (j, gpu) in gpus.iter().enumerate() {
                    let default_idx = if menu.prefers_dgpu {
                        gpus.iter().position(|gpu| !gpu.default).unwrap_or(0)
                    } else {
                        gpus.iter().position(|gpu| gpu.default).unwrap_or(0)
                    };
                    list_column.push(
                        menu_button(text::body(format!(
                            "{} {}",
                            fl!("run-on", gpu = gpu.name.as_str()),
                            if j == default_idx {
                                fl!("run-on-default")
                            } else {
                                String::new()
                            }
                        )))
                        .on_press(Message::ActivateApp(*i, Some(j)))
                        .into(),
                    )
                }
            } else {
                list_column.push(
                    menu_button(text::body(RUN.clone()))
                        .on_press(Message::ActivateApp(*i, None))
                        .into(),
                );
            }

            if !menu.desktop_actions.is_empty() {
                list_column.push(divider::horizontal::light().into());
                list_column.push(
                    container(text::caption("Application actions"))
                        .padding([space_xxs, space_s])
                        .into(),
                );
                for action in menu.desktop_actions.iter() {
                    list_column.push(
                        menu_button(text::body(&action.name))
                            .on_press(Message::SelectAction(MenuAction::DesktopAction(
                                action.exec.clone(),
                            )))
                            .into(),
                    );
                }
            }

            list_column.push(divider::horizontal::light().into());
            list_column.push(
                menu_button(text::body("Open in COSMIC Store"))
                    .on_press(Message::SelectAction(MenuAction::OpenInStore(
                        menu.id.clone(),
                    )))
                    .into(),
            );

            // add to pinned
            let svg_accent = Rc::new(|theme: &cosmic::Theme| {
                let color = theme.cosmic().accent_color().into();
                svg::Style { color: Some(color) }
            });
            let is_pinned = self.app_list_config.favorites.iter().any(|p| p == &menu.id);
            let pin_to_app_tray = menu_button(
                if is_pinned {
                    row![
                        icon::icon(icon::from_name("checkbox-checked-symbolic").size(16).into())
                            .class(cosmic::theme::Svg::Custom(svg_accent.clone())),
                        text::body(fl!("pin-to-app-tray"))
                    ]
                } else {
                    row![
                        space::horizontal().width(16.0),
                        text::body(fl!("pin-to-app-tray"))
                    ]
                }
                .spacing(space_xxs),
            )
            .on_press(if is_pinned {
                Message::UnPinFromAppTray(*i)
            } else {
                Message::PinToAppTray(*i)
            });
            list_column.push(divider::horizontal::light().into());
            list_column.push(pin_to_app_tray.into());

            let is_favorite = self
                .config
                .page_layout(self.active_page)
                .iter()
                .any(|entry| entry.as_deref() == Some(menu.id.as_str()));
            list_column.push(divider::horizontal::light().into());
            list_column.push(if is_favorite {
                menu_button(text::body("Remove from Launchpad"))
                    .on_press(Message::SelectAction(MenuAction::Remove))
                    .into()
            } else {
                menu_button(text::body("Add to Launchpad"))
                    .on_press(Message::SelectAction(MenuAction::Pin))
                    .into()
            });

            let is_hidden = self.config.is_hidden(&menu.id);
            list_column.push(if is_hidden {
                menu_button(text::body("Show in Other Apps"))
                    .on_press(Message::SelectAction(MenuAction::ShowInOtherApps))
                    .into()
            } else {
                menu_button(text::body("Hide from Other Apps"))
                    .on_press(Message::SelectAction(MenuAction::HideFromOtherApps))
                    .into()
            });

            return autosize(
                container(scrollable(MenuColumn::with_children(list_column))).padding(1),
                MENU_AUTOSIZE_ID.clone(),
            )
            .max_height(800.)
            .max_width(300.)
            .into();
        }
        if id == *NEW_GROUP_WINDOW_ID {
            let Some(group_name) = self.new_group.as_ref() else {
                return container(space::horizontal())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(1.0))
                    .into();
            };
            let dialog = widget::dialog::dialog()
                .title(CREATE_NEW.as_str())
                .control(
                    text_input("", group_name)
                        .label(&*NEW_GROUP_PLACEHOLDER)
                        .on_input(Message::NewGroup)
                        .on_submit(|_| Message::SubmitNewGroup)
                        .width(Length::Fixed(432.0))
                        .size(14)
                        .id(NEW_GROUP_ID.clone()),
                )
                .primary_action(
                    button::custom(text::body(SAVE.as_str()).center().width(Length::Fill))
                        .class(Button::Suggested)
                        .on_press(Message::SubmitNewGroup)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .secondary_action(
                    button::custom(text::body(CANCEL.as_str()).center().width(Length::Fill))
                        .on_press(Message::CancelNewGroup)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .width(Length::Fixed(432.0));

            return autosize(dialog, NEW_GROUP_AUTOSIZE_ID.clone()).into();
        }
        if id == *DELETE_GROUP_WINDOW_ID {
            let dialog = widget::dialog::dialog()
                .icon(icon::from_name("edit-delete-symbolic").size(48))
                .title(fl!("delete-folder"))
                .body(fl!("delete-folder", "msg"))
                .primary_action(
                    button::custom(text::body(fl!("delete")).center().width(Length::Fill))
                        .id(SUBMIT_DELETE_ID.clone())
                        .class(Button::Destructive)
                        .on_press(Message::ConfirmDelete)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .secondary_action(
                    button::custom(text::body(CANCEL.to_string()).center().width(Length::Fill))
                        .on_press(Message::CancelDelete)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .width(Length::Fixed(432.0));

            return autosize(dialog, DELETE_GROUP_AUTOSIZE_ID.clone()).into();
        }

        // The board keeps every group visible at once. The stock folder-based
        // view remains below for now, making upstream rebases easier while the
        // board interaction model is developed.
        let _ = (space_none, space_xs, space_l, space_xxl);
        return self.board_view();

        #[allow(unreachable_code)]
        let cur_group = self.current_group();
        let top_row = if self.cur_group.is_none() {
            row![
                container(
                    search_input(SEARCH_PLACEHOLDER.as_str(), self.search_value.as_str())
                        .on_input(Message::InputChanged)
                        .on_paste(Message::InputChanged)
                        .on_submit(|_| Message::StartCurAppFocus)
                        .style(TextInput::Search)
                        .width(Length::Fixed(400.0))
                        .size(14)
                        .id(SEARCH_ID.clone())
                )
                .align_y(Vertical::Center)
                .height(Length::Fixed(96.0))
            ]
            .align_y(Alignment::Center)
            .spacing(space_xxs)
        } else {
            row![
                space::horizontal().width(Length::FillPortion(1)),
                if let Some(edit_name) = self.edit_name.as_ref() {
                    container(
                        text_input(cur_group.name(), edit_name)
                            .on_input(Message::EditName)
                            .on_paste(Message::EditName)
                            .on_clear(Message::EditName(String::new()))
                            .on_submit(|_| Message::SubmitName)
                            .id(EDIT_GROUP_ID.clone())
                            .width(Length::Fixed(200.0))
                            .size(14),
                    )
                } else {
                    container(text(cur_group.name()).size(24))
                },
                row![
                    space::horizontal(),
                    tooltip(
                        {
                            let mut b = button::custom(
                                icon::icon(icon::from_name("edit-symbolic").into())
                                    .width(Length::Fixed(32.0))
                                    .height(Length::Fixed(32.0)),
                            )
                            .padding(space_xs)
                            .class(Button::Icon);
                            if self.edit_name.is_none() {
                                b = b.on_press(Message::StartEditName(cur_group.name()));
                            }
                            container(b)
                                .height(Length::Fixed(96.0))
                                .align_y(Vertical::Center)
                        },
                        text(fl!("rename")),
                        tooltip::Position::Bottom
                    ),
                    tooltip(
                        container(
                            button::custom(
                                icon::icon(icon::from_name("edit-delete-symbolic").into())
                                    .width(Length::Fixed(32.0))
                                    .height(Length::Fixed(32.0)),
                            )
                            .padding(space_xs)
                            .class(Button::Icon)
                            .on_press_maybe(self.cur_group.map(Message::Delete))
                        )
                        .height(Length::Fixed(96.0))
                        .align_y(Vertical::Center),
                        text(fl!("delete")),
                        tooltip::Position::Bottom
                    )
                ]
                .spacing(space_xxs)
                .width(Length::FillPortion(1))
            ]
            .padding([0, space_l])
            .align_y(Alignment::Center)
        };

        // TODO grid widget in libcosmic
        let app_grid_list: Vec<_> = self
            .entry_path_input
            .iter()
            .zip(self.entry_ids.iter())
            .zip(self.entry_icon_handles.iter())
            .enumerate()
            .map(|(i, ((entry, id), icon_handle))| {
                let gpu_idx = self.gpus.as_ref().map(|gpus| {
                    if entry.prefers_dgpu {
                        gpus.iter().position(|gpu| !gpu.default).unwrap_or(0)
                    } else {
                        gpus.iter().position(|gpu| gpu.default).unwrap_or(0)
                    }
                });
                let dup = entry
                    .path
                    .as_ref()
                    .and_then(|path| self.duplicates.get(path));
                let selected = self.menu.is_some_and(|m| m == i);

                let b = ApplicationButton::new(
                    id.clone(),
                    &entry.name,
                    icon_handle.clone(),
                    &entry.path,
                    move |rect| Message::OpenContextMenu(rect, i),
                    if self.menu.is_none() {
                        Some(Message::ActivateApp(i, gpu_idx))
                    } else if selected {
                        Some(Message::CloseContextMenu)
                    } else {
                        None
                    },
                    // TODO add icon and text if duplicated
                    dup,
                    selected,
                    self.menu.is_none().then_some(Message::StartDrag(i)),
                    self.menu.is_none().then_some(Message::FinishDrag),
                    self.menu.is_none().then_some(Message::CancelDrag),
                    self.config.icon_size as f32,
                    self.config.show_labels,
                );

                b.into()
            })
            .chunks(7)
            .into_iter()
            .map(|row_chunk| {
                let mut new_row = row_chunk.collect_vec();
                let missing = 7 - new_row.len();
                if missing > 0 {
                    new_row.push(
                        iced::widget::space::horizontal()
                            .width(Length::FillPortion(missing.try_into().unwrap()))
                            .into(),
                    );
                }
                row(new_row).spacing(space_xxs).into()
            })
            .collect();

        let app_scrollable = container(
            scrollable(
                column(app_grid_list)
                    .width(Length::Fill)
                    .spacing(space_xxs)
                    // padding on top needed to avoid focus highlight clipping
                    .padding([4, space_xxl, space_xxs, space_xxl]),
            )
            .on_scroll(|viewport| Message::ScrollYOffset(viewport.absolute_offset().y))
            .id(self.scrollable_id.clone())
            .height(Length::Fill),
        )
        .max_height(444.0);

        // TODO use the spacing variables from the theme
        let (group_icon_size, h_padding, group_width) = if self.config.groups.len() + 1 > 15 {
            (16.0, space_xxs, 96.0)
        } else {
            (32.0, space_s, 128.0)
        };
        let group_height =
            group_icon_size + 21.0 + (space_none as f32) + (space_xxs as f32) + (space_s as f32);

        let build_group_button = |group_ref: Option<usize>, group: &crate::app_group::AppGroup| {
            let is_active = self.offer_group == Some(group_ref)
                || (self.cur_group == group_ref && self.offer_group.is_none());
            dnd_destination_for_data::<AppletString, Message>(
                button::custom(
                    column![
                        container(
                            icon::icon(from_name(group.icon.clone()).into())
                                .width(Length::Fixed(group_icon_size))
                                .height(Length::Fixed(group_icon_size))
                        )
                        .padding(space_xxs),
                        text::body(group.name()).width(Length::Shrink)
                    ]
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
                )
                .height(Length::Fixed(group_height))
                .width(Length::Fixed(group_width))
                .class(if is_active {
                    Button::Custom {
                        active: Box::new(|focused, theme| {
                            theme.pressed(focused, false, &Button::IconVertical)
                        }),
                        disabled: Box::new(|theme| theme.disabled(&Button::IconVertical)),
                        hovered: Box::new(|focused, theme| {
                            theme.hovered(focused, false, &Button::IconVertical)
                        }),
                        pressed: Box::new(|focused, theme| {
                            theme.pressed(focused, false, &Button::IconVertical)
                        }),
                    }
                } else {
                    Button::IconVertical
                })
                .padding([space_none, h_padding, space_xxs, h_padding])
                .on_press_maybe(
                    self.menu
                        .is_none()
                        .then_some(Message::SelectGroup(group_ref)),
                ),
                move |data, _| {
                    Message::FinishDndOffer(
                        group_ref,
                        data.and_then(|data| load_desktop_file(&[], data.0)),
                    )
                },
            )
            .drag_id(group_ref.map(|i| i as u64 + 1).unwrap_or(0))
            .on_enter(move |_, _, _| Message::StartDndOffer(group_ref))
            .on_leave(move || Message::LeaveDndOffer(group_ref))
        };

        let add_group_btn = button::custom(
            column![
                container(
                    icon::icon(icon::from_name("folder-new-symbolic").into())
                        .width(Length::Fixed(group_icon_size))
                        .height(Length::Fixed(group_icon_size))
                )
                .padding(space_xxs),
                text::body(ADD_GROUP.as_str()).width(Length::Shrink)
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .height(Length::Fixed(group_height))
        .width(Length::Fixed(group_width))
        .class(theme::Button::IconVertical)
        .padding([space_none, h_padding, space_xxs, h_padding])
        .on_press(Message::StartNewGroup);

        let home = AppLibraryConfig::home();
        let group_row = self
            .config
            .groups
            .iter()
            .enumerate()
            .fold(
                reorderable_flex_row::<GroupRowKey, Message>(Message::ReorderGroup)
                    .spacing(space_xxs)
                    .padding([space_s, space_none])
                    .push_locked(GroupRowKey::Home, build_group_button(None, home)),
                |row, (i, group)| {
                    let key = self.group_keys.get(i).copied().unwrap_or(i as u64);
                    row.push(GroupRowKey::Custom(key), build_group_button(Some(i), group))
                },
            )
            .push_locked(GroupRowKey::NewGroup, add_group_btn);

        let content = column![
            top_row,
            app_scrollable,
            container(horizontal_rule(1))
                .padding([space_none, space_xxl])
                .width(Length::Fill),
            group_row
        ]
        .align_x(Alignment::Center);

        let window = container(content)
            .height(Length::Fixed(690.))
            .max_height(690)
            .max_width(1200.0)
            .class(theme::Container::Custom(Box::new(|theme| {
                let t = theme.cosmic();
                let radii = t.radius_s().map(|x| if x < 4.0 { x } else { x + 4.0 });

                container::Style {
                    text_color: Some(t.on_bg_color().into()),
                    icon_color: Some(t.on_bg_color().into()),
                    background: Some(Color::from(t.background(theme.transparent).base).into()),
                    border: Border {
                        radius: radii.into(),
                        width: 1.0,
                        color: t.bg_divider().into(),
                    },
                    shadow: Shadow::default(),
                    snap: true,
                }
            })))
            .center_x(Length::Fill)
            .width(Length::Fixed(1200.));
        stack![
            mouse_area(
                container(space::horizontal().width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .on_press(Message::Hide),
            column!(
                space::vertical().height(Length::Fixed(self.margin + 16.)),
                mouse_area(window).on_press(Message::CloseContextMenu),
            )
            .align_x(Alignment::Center)
            .width(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            desktop_files(0).map(|_| Message::LoadApps),
            listen_with(|e, status, id| match e {
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::Layer(e, _, id),
                )) => Some(Message::Layer(e, id)),
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::OverlapNotify(event, ..),
                )) => Some(Message::Overlap(event)),
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyReleased {
                    key: Key::Named(Named::Escape),
                    modifiers: _mods,
                    ..
                }) => None,
                cosmic::iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Pixels { y, .. },
                }) if id == SurfaceId::RESERVED => {
                    Some(Message::TrackpadScroll(-y, Instant::now()))
                }
                cosmic::iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))
                    if id == SurfaceId::RESERVED =>
                {
                    Some(Message::CloseContextMenu)
                }
                cosmic::iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    text: key_text,
                    modifiers,
                    ..
                }) => {
                    if matches!(status, iced::event::Status::Ignored)
                        && !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(key_text) = key_text
                        && key_text.chars().any(|character| !character.is_control())
                    {
                        Some(Message::TypeToSearch(key_text.to_string()))
                    } else {
                        match key {
                            Key::Character(c) if modifiers.control() && (c == "p" || c == "k") => {
                                Some(Message::PrevRow)
                            }
                            Key::Character(c) if modifiers.control() && (c == "n" || c == "j") => {
                                Some(Message::NextRow)
                            }
                            Key::Character(c) if modifiers.control() && (c == "f" || c == "l") => {
                                Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                            }
                            Key::Character(c) if modifiers.control() && (c == "b" || c == "h") => {
                                Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                            }
                            Key::Named(Named::ArrowUp)
                                if matches!(status, iced::event::Status::Ignored) =>
                            {
                                Some(Message::PrevRow)
                            }
                            Key::Named(Named::ArrowDown)
                                if matches!(status, iced::event::Status::Ignored) =>
                            {
                                Some(Message::NextRow)
                            }
                            Key::Named(Named::ArrowLeft)
                                if matches!(status, iced::event::Status::Ignored) =>
                            {
                                Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                            }
                            Key::Named(Named::ArrowRight)
                                if matches!(status, iced::event::Status::Ignored) =>
                            {
                                Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                            }
                            _ => None,
                        }
                    }
                }
                cosmic::iced::Event::Window(WindowEvent::Opened { position: _, size }) => {
                    Some(Message::Opened(size, id))
                }
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::Output(event, _),
                )) => Some(Message::Output(event)),
                _ => None,
            }),
            keyboard_nav::subscription().map(Message::KeyboardNav),
            self.core
                .watch_config::<cosmic_app_list_config::AppListConfig>(
                    cosmic_app_list_config::APP_ID,
                )
                .map(|config| Message::AppListConfig(config.config)),
        ];

        if self.last_trackpad_scroll.is_some() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(8)).map(Message::MomentumTick));
        }

        Subscription::batch(subscriptions)
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Args) -> (Self, iced::Task<cosmic::Action<Self::Message>>) {
        core.set_keyboard_nav(false);
        core.set_app_type(cosmic::core::AppType::System);

        let helper = AppLibraryConfig::helper();

        let config: AppLibraryConfig = helper
            .as_ref()
            .map(|helper| {
                AppLibraryConfig::get_entry(helper).unwrap_or_else(|(errors, config)| {
                    for err in errors {
                        error!("{:?}", err);
                    }
                    config
                })
            })
            .unwrap_or_default();
        let scrollable_id = Id::new("group-home");
        let group_count = config.groups.len() as u64;
        let group_keys: Vec<u64> = (0..group_count).collect();
        let mut self_ = Self {
            locale: std::env::var("LANG")
                .ok()
                .and_then(|l| l.split(".").next().map(str::to_string)),
            config,
            core,
            helper,
            last_hide: None,
            margin: 0.,
            overlap: HashMap::new(),
            size: Size::new(1920., 1080.),
            scrollable_id,
            group_keys,
            next_group_key: group_count,
            super_shortcut_enabled: Self::read_super_shortcut().unwrap_or(false),
            ..Default::default()
        };

        // Open the board on a normal desktop launch as well as in standalone mode.
        // The single-instance runner forwards later launches over D-Bus.
        let task = if !matches!(flags.subcommand, Some(ApplicationsTasks::Close)) {
            Task::done(cosmic::Action::App(Message::Activate))
        } else {
            Task::none()
        };
        let dummy_task = self_.create_dummy_layer_surface();
        (self_, Task::batch([dummy_task, task]))
    }
}
