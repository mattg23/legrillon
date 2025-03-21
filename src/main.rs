use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicUsize},
};

use controls::MainControls;
use db::{LeGrillonDb, OpenWindow};
use fltk::app;
use fltk_theme::WidgetTheme;
use req_window::RequestWindow;

mod controls;
mod db;
mod req_params;
mod req_window;

#[derive(Debug)]
enum GlobalAppMsg {
    OpenEmptyWindow,
    Restore(OpenWindow),
    SaveWindowState(OpenWindow),
    CloseWindow(usize),
    CloseApp,
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
    ctrls: MainControls,
    windows: std::collections::BTreeMap<usize, Box<dyn AppWindow>>,
    db: Arc<LeGrillonDb>,
}

impl LeGrillon {
    pub async fn new() -> Self {
        let app = app::App::default();
        let (s, receiver) = app::channel();
        app::set_font_size(18);
        let widget_theme = WidgetTheme::new(fltk_theme::ThemeType::Dark);
        widget_theme.apply();

        let ctrls = crate::controls::MainControls::new(s);

        let window_map: BTreeMap<usize, Box<dyn AppWindow>> = std::collections::BTreeMap::new();

        let db = Arc::new(LeGrillonDb::new().await);

        LeGrillon {
            app,
            receiver,
            ctrls,
            windows: window_map,
            db,
        }
    }

    pub fn run(&mut self) {
        LeGrillonDb::restore(self.db.clone());

        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                println!("MAIN::RUN:: {msg:?}");
                match msg {
                    GlobalAppMsg::OpenEmptyWindow => self.open(None),
                    GlobalAppMsg::CloseWindow(id) => self.close(id),
                    GlobalAppMsg::SaveWindowState(_) => (),
                    GlobalAppMsg::Restore(ref open_window) => self.open(Some(open_window)),
                    GlobalAppMsg::CloseApp => {
                        for wnd in self.windows.values_mut() {
                            wnd.close();
                        }
                    }
                }
                LeGrillonDb::handle(self.db.clone(), msg);
            }
        }
    }

    fn close(&mut self, id: usize) {
        if let Some((_, mut window)) = self.windows.remove_entry(&id) {
            window.close()
        }
    }

    fn open(&mut self, wnd: Option<&OpenWindow>) {
        let req_win = RequestWindow::new(wnd);
        self.windows.insert(req_win.id(), Box::new(req_win));
    }
}

#[tokio::main]
async fn main() {
    LeGrillon::new().await.run();
}
