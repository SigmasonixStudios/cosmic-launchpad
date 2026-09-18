use cosmic::Element;
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;

const APP_ID: &str = "com.sigmasonix.CosmicLaunchpadApplet";
const LAUNCHER_ID: &str = "com.sigmasonix.CosmicLaunchpad";

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<LaunchpadApplet>(())
}

#[derive(Default)]
struct LaunchpadApplet {
    core: Core,
}

#[derive(Clone, Debug)]
enum Message {
    Activate,
    Surface(cosmic::surface::Action<Message>),
}

impl cosmic::Application for LaunchpadApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        (Self { core }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Activate => {
                let executable = std::env::current_exe()
                    .ok()
                    .and_then(|path| {
                        path.parent()
                            .map(|parent| parent.join("cosmic-application-board"))
                    })
                    .unwrap_or_else(|| "cosmic-application-board".into());
                let command = executable.to_string_lossy().into_owned();
                tokio::spawn(async move {
                    cosmic::desktop::spawn_desktop_exec(
                        command,
                        Vec::<(String, String)>::new(),
                        Some(LAUNCHER_ID),
                        false,
                    )
                    .await;
                });
            }
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Surface(action));
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let button = self
            .core
            .applet
            .icon_button("application-menu-symbolic")
            .on_press(Message::Activate);

        self.core
            .applet
            .applet_tooltip(button, "Launchpad", false, Message::Surface, None)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        "".into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
