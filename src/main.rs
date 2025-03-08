use std::{collections::BTreeMap, sync::atomic::AtomicUsize};

use fltk::app;
use fltk_theme::WidgetTheme;
use req_window::RequestWindow;

mod controls;
mod req_window;

#[derive(Clone, Copy, Debug)]
enum GlobalAppMsg {
    OpenWindow,
    CloseWindow(usize),
}

pub(crate) trait HasId {
    fn id(&self) -> usize;
}

pub(crate) trait AppWindow {
    fn close(&mut self);
}

static WINDOW_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_window_id() -> usize {
    WINDOW_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

struct LeGrillon {
    app: app::App,
    receiver: app::Receiver<GlobalAppMsg>,
    windows: std::collections::BTreeMap<usize, Box<dyn AppWindow>>,
}

impl LeGrillon {
    pub fn new() -> Self {
        let app = app::App::default();
        let (s, receiver) = app::channel();
        app::set_font_size(18);
        let widget_theme = WidgetTheme::new(fltk_theme::ThemeType::Dark);
        widget_theme.apply();

        let ctrls = crate::controls::MainControls::new(s);

        let mut window_map: BTreeMap<usize, Box<dyn AppWindow>> = std::collections::BTreeMap::new();
        window_map.insert(ctrls.id(), Box::new(ctrls));

        LeGrillon {
            app,
            receiver,
            windows: window_map,
        }
    }

    pub fn run(&mut self) {
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                println!("{msg:?}");
                match msg {
                    GlobalAppMsg::OpenWindow => self.open(),
                    GlobalAppMsg::CloseWindow(id) => self.close(id),
                }
            }
        }
    }

    fn close(&mut self, id: usize) {
        if let Some((_, mut window)) = self.windows.remove_entry(&id) {
            window.close()
        }
    }

    fn open(&mut self) {
        let req_win = RequestWindow::new();
        self.windows.insert(req_win.id(), Box::new(req_win));
    }
}

#[tokio::main]
async fn main() {
    LeGrillon::new().run();
}
